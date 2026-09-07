use std::collections::HashMap;

use futures::{
    StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
};
use protocol::{
    error::ProtocolError,
    ssh::{ProtocolChannelMsg, SshConnection, TerminalChannel},
};

use crate::{
    error::RuntimeError,
    id::{TabId, TransferId},
    runtime::{
        MonitorRuntimeHandle, SftpRuntimeHandle, TerminalHandle, TransferRuntimeHandle,
        event::{FileCommand, MonitorCommand, RuntimeCommand, RuntimeEvent, TerminalCommand},
    },
    session::session::Session,
};

pub mod session;
// ============================================================
// SessionRuntime
//
// 一个 Session 对应一个 Runtime。
// Runtime 持有 SSH Connection。
// Runtime 不直接持有 TerminalChannel。
// ============================================================

pub struct SessionRuntime {
    /// 持久化的 Session 配置
    session: Session,

    /// 接收外部控制命令
    command_rx: UnboundedReceiver<RuntimeCommand>,

    /// 向应用层发送运行时事件
    event_tx: UnboundedSender<RuntimeEvent>,

    /// 一个 Session 对应一个 SSH 长连接
    connection: Option<SshConnection>,

    /// 当前 Session 创建的 Terminal
    terminals: HashMap<TabId, TerminalHandle>,

    /// 监控子系统
    monitor: Option<MonitorRuntimeHandle>,

    /// SFTP 子系统
    sftp: Option<SftpRuntimeHandle>,

    /// 当前进行中的文件传输
    transfers: HashMap<TransferId, TransferRuntimeHandle>,
}

// ============================================================
// SessionRuntime
// ============================================================

impl SessionRuntime {
    pub fn new(
        session: Session,
        event_tx: UnboundedSender<RuntimeEvent>,
    ) -> (Self, SessionRuntimeHandle) {
        let (command_tx, command_rx) = futures::channel::mpsc::unbounded();

        // let (event_tx, event_rx) = futures::channel::mpsc::unbounded();

        let runtime = Self {
            session,

            command_rx,

            event_tx,

            connection: None,

            terminals: HashMap::default(),

            monitor: None,

            sftp: None,

            transfers: HashMap::default(),
        };

        let handle = SessionRuntimeHandle::new(command_tx);

        (runtime, handle)
    }
    fn send_err(&mut self, err: ProtocolError) {
        let _ = self
            .event_tx
            .unbounded_send(RuntimeEvent::Error { error: err.into() });
    }
    // ========================================================
    // Runtime 主循环
    //
    // command_rx 只能由这里消费。
    //
    // 绝对不能在 open_terminal() 里面 take command_rx。
    // ========================================================

    pub async fn run(mut self) {
        if let Err(error) = self.connect().await {
            let _ = self.send_err(error);
            return;
        }

        let _ = self.event_tx.unbounded_send(RuntimeEvent::Connected {
            session_id: self.session.id.clone(),
        });

        while let Some(command) = self.command_rx.next().await {
            let result = self.handle_command(command).await;

            if let Err(error) = result {
                log::error!("SessionRuntime command error: {error:#}");
            }
        }

        // command channel 关闭
        // Runtime 结束
        self.shutdown().await;
    }

    // ========================================================
    // SSH Connect
    // ========================================================

    async fn connect(&mut self) -> Result<(), ProtocolError> {
        let connection = SshConnection::connect(&protocol::ssh::SshConfig {
            hostname: self.session.hostname.clone(),
            port: self.session.port,
            username: self.session.username.clone(),
            auth: self.session.auth.clone(),
        })
        .await?;

        self.connection = Some(connection);

        Ok(())
    }

    // ========================================================
    // RuntimeCommand
    // ========================================================

    async fn handle_command(&mut self, command: RuntimeCommand) -> anyhow::Result<()> {
        match command {
            // ------------------------------------------------
            // Terminal
            // ------------------------------------------------
            RuntimeCommand::Terminal { tab_id, command } => {
                self.handle_terminal_command(tab_id, command).await?;
            }

            // ------------------------------------------------
            // Monitor
            // ------------------------------------------------
            RuntimeCommand::Monitor(command) => {
                self.handle_monitor_command(command).await?;
            }

            // ------------------------------------------------
            // Files
            // ------------------------------------------------
            RuntimeCommand::Files(command) => {
                self.handle_file_command(command).await?;
            }

            // ------------------------------------------------
            // Disconnect
            // ------------------------------------------------
            RuntimeCommand::Disconnect => {
                self.shutdown().await;

                return Ok(());
            }
        }

        Ok(())
    }

    // ========================================================
    // Terminal
    // ========================================================

    async fn handle_terminal_command(
        &mut self,
        tab_id: TabId,
        command: TerminalCommand,
    ) -> anyhow::Result<()> {
        match command {
            // ------------------------------------------------
            // 打开 Terminal
            // ------------------------------------------------
            TerminalCommand::Open => {
                self.open_terminal(tab_id).await.map_err(|err| {
                    // let terminal = self
                    //     .terminals
                    //     .get(&tab_id)
                    //     .ok_or_else(|| anyhow::anyhow!("Terminal 不存在: {:?}", tab_id))?;
                });
            }

            // ------------------------------------------------
            // 输入
            // ------------------------------------------------
            TerminalCommand::Input { data } => {
                let terminal = self
                    .terminals
                    .get(&tab_id)
                    .ok_or_else(|| anyhow::anyhow!("Terminal 不存在: {:?}", tab_id))?;

                let _ = terminal.input(data);
            }

            // ------------------------------------------------
            // Resize
            // ------------------------------------------------
            TerminalCommand::Resize { cols, rows } => {
                let terminal = self
                    .terminals
                    .get(&tab_id)
                    .ok_or_else(|| anyhow::anyhow!("Terminal 不存在: {:?}", tab_id))?;

                terminal.resize(cols, rows);
            }

            // ------------------------------------------------
            // Close
            // ------------------------------------------------
            TerminalCommand::Close => {
                if let Some(terminal) = self.terminals.remove(&tab_id) {
                    terminal.close();
                }
            }
        }

        Ok(())
    }

    // ========================================================
    // 创建 Terminal
    // ========================================================

    async fn open_terminal(&mut self, tab_id: TabId) -> anyhow::Result<()> {
        // 防止重复创建
        if self.terminals.contains_key(&tab_id) {
            return Ok(());
        }

        let terminal = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SSH 连接未建立"))?
            .open_terminal()
            .await?;

        // ----------------------------------------------------
        // 每个 Terminal 自己拥有一套 command channel
        // ----------------------------------------------------

        let (cmd_tx, cmd_rx) = futures::channel::mpsc::unbounded();

        // Runtime 只保存 Handle
        let handle = TerminalHandle::new(cmd_tx);
        self.terminals.insert(tab_id.clone(), handle);

        let event_tx = self.event_tx.clone();

        // ----------------------------------------------------
        // Terminal Task
        //
        // 这个 Task 独占 terminal。
        //
        // terminal:
        //     SSH Channel
        //
        // cmd_rx:
        //     UI → Terminal
        //
        // event_tx:
        //     Terminal → Runtime
        // ----------------------------------------------------

        tokio::spawn(async move {
            run_terminal(tab_id, terminal, cmd_rx, event_tx).await;
        });

        Ok(())
    }

    // ========================================================
    // Monitor
    // ========================================================

    async fn handle_monitor_command(&mut self, command: MonitorCommand) -> anyhow::Result<()> {
        match command {
            MonitorCommand::Start => {
                // TODO
            }

            MonitorCommand::Stop => {
                // TODO
            }

            MonitorCommand::Refresh => {
                // TODO
            }
        }

        Ok(())
    }

    // ========================================================
    // Files
    // ========================================================

    async fn handle_file_command(&mut self, command: FileCommand) -> anyhow::Result<()> {
        match command {
            FileCommand::List { path } => {
                // TODO
                //
                // self.sftp
                //     .as_ref()
                //     ...
            }

            FileCommand::Mkdir { path } => {
                // TODO
            }

            FileCommand::Delete { path } => {
                // TODO
            }

            FileCommand::Rename { from, to } => {
                // TODO
            }
        }

        Ok(())
    }

    // ========================================================
    // Shutdown
    // ========================================================

    async fn shutdown(&mut self) {
        // 关闭所有 Terminal
        for (_, terminal) in self.terminals.drain() {
            terminal.close();
        }

        self.monitor = None;

        self.sftp = None;

        self.transfers.clear();

        self.connection = None;
    }
}
// ============================================================
// Terminal Task
//
// 一个 Terminal 对应一个 Tokio Task。
// 这个 Task 独占 TerminalChannel。
//
// 这里同时负责：
//     1. UI → SSH
//     2. SSH → Runtime
// ============================================================

async fn run_terminal(
    tab_id: TabId,

    mut terminal: TerminalChannel,

    mut command_rx: UnboundedReceiver<TerminalCommand>,

    event_tx: UnboundedSender<RuntimeEvent>,
) {
    loop {
        tokio::select! {

            // =================================================
            // UI → Terminal
            // =================================================

            command = command_rx.next() => {

                match command {

                    Some(
                        TerminalCommand::Input {
                            data,
                        }
                    ) => {

                        if let Err(error) =
                            terminal
                                .write(&data)
                                .await
                        {
                            log::error!(
                                "Terminal input error: {error}"
                            );

                            break;
                        }
                    }


                    Some(
                        TerminalCommand::Resize {
                            cols,
                            rows,
                        }
                    ) => {

                        if let Err(error) =
                            terminal
                                .window_change(
                                    cols.into(),
                                    rows.into()
                                )
                                .await
                        {
                            log::error!(
                                "Terminal resize error: {error}"
                            );
                        }
                    }


                    Some(
                        TerminalCommand::Close
                    )
                    | None => {

                        terminal.eof().await.ok();

                        break;
                    }


                    // 如果 Open 也属于 TerminalCommand
                    Some(
                        TerminalCommand::Open {
                            ..
                        }
                    ) => {
                        // Terminal 已经打开，
                        // Task 内不需要处理 Open。
                    }
                }
            }


            // =================================================
            // SSH → Terminal
            // =================================================

            message = terminal.read() => {

                match message {

                    // ------------------------------------------------
                    // stdout
                    // ------------------------------------------------

                    Some(
                        ProtocolChannelMsg::Data {
                            data,
                        }
                    ) => {

                        let _ =
                            event_tx.unbounded_send(
                                RuntimeEvent::TerminalOutput {
                                    tab_id: tab_id.clone(),
                                    bytes: data.to_vec(),
                                }
                            );
                    }


                    // ------------------------------------------------
                    // stderr
                    // ------------------------------------------------

                    Some(
                         ProtocolChannelMsg::ExtendedData {
                            data,
                            ..
                        }
                    ) => {

                        let _ =
                            event_tx.unbounded_send(
                                RuntimeEvent::TerminalOutput {
                                    tab_id: tab_id.clone(),
                                    bytes: data.to_vec(),
                                }
                            );
                    }


                    // ------------------------------------------------
                    // SSH Channel 结束
                    // ------------------------------------------------

                    Some( ProtocolChannelMsg::Close)
                    | Some( ProtocolChannelMsg::Eof)
                    | None => {

                        let _ =
                            event_tx.unbounded_send(
                                RuntimeEvent::TerminalExit { tab_id }
                            );

                        break;
                    }


                    _ => {}
                }
            }
        }
    }
}
// ============================================================
// SessionRuntimeHandle
//
// UI 持有这个 Handle。
// UI 不直接接触 SessionRuntime / SSH Channel。
// ============================================================

#[derive(Clone)]
pub struct SessionRuntimeHandle {
    tx: UnboundedSender<RuntimeCommand>,
}

impl SessionRuntimeHandle {
    pub fn new(tx: UnboundedSender<RuntimeCommand>) -> Self {
        Self { tx }
    }
    pub fn open_terminal(&self, tab_id: TabId) {
        let _ = self.tx.unbounded_send(RuntimeCommand::Terminal {
            tab_id,
            command: TerminalCommand::Open,
        });
    }

    pub fn terminal_input(&self, tab_id: TabId, data: Vec<u8>) {
        let _ = self.tx.unbounded_send(RuntimeCommand::Terminal {
            tab_id,
            command: TerminalCommand::Input { data },
        });
    }

    pub fn terminal_resize(&self, tab_id: TabId, cols: u16, rows: u16) {
        let _ = self.tx.unbounded_send(RuntimeCommand::Terminal {
            tab_id,
            command: TerminalCommand::Resize { cols, rows },
        });
    }

    pub fn terminal_close(&self, tab_id: TabId) {
        let _ = self.tx.unbounded_send(RuntimeCommand::Terminal {
            tab_id,
            command: TerminalCommand::Close,
        });
    }

    pub fn start_monitor(&self) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Monitor(MonitorCommand::Start));
    }

    pub fn stop_monitor(&self) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Monitor(MonitorCommand::Stop));
    }

    pub fn list_directory(&self, path: impl Into<String>) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Files(FileCommand::List {
                path: path.into(),
            }));
    }

    pub fn disconnect(&self) {
        let _ = self.tx.unbounded_send(RuntimeCommand::Disconnect);
    }
}
