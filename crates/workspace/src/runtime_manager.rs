use std::collections::VecDeque;

use anyhow::Ok;
use futures::{
    StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
};

use gpui::Task;
use protocol::{
    RuntimeCommand, RuntimeEvent, Session, SessionId, TabId, TerminalCommand, TransferId,
    ssh::{SshConnection, TerminalChannel},
};

use russh::ChannelMsg;

use terminal::{Content, Terminal, TerminalBounds, new_term, normalize_terminal_bounds};
use utils::collections::HashMap;
use vte::ansi::{Processor, StdSyncHandler};

use crate::{
    monitor_store::MonitorRuntimeHandle,
    transfer_store::{SftpRuntimeHandle, TransferRuntimeHandle},
};

// ============================================================
// RuntimeManager
// ============================================================

pub struct RuntimeManager {
    runtimes: HashMap<SessionId, SessionRuntimeHandle>,
    event_tx: UnboundedSender<RuntimeEvent>,
    event_loop_task: Task<Result<(), anyhow::Error>>,
}

impl RuntimeManager {
    pub fn new(event_tx: UnboundedSender<RuntimeEvent>) -> Self {
        Self {
            runtimes: HashMap::default(),
            event_tx,
            event_loop_task: Task::ready(Ok(())),
        }
    }

    pub fn open_session(&mut self, session: Session) -> anyhow::Result<TabId> {
        let runtime = self.get_or_create(session.clone())?;

        let tab_id = TabId::new();

        runtime.open_terminal(tab_id.clone());

        Ok(tab_id)
    }
    pub fn get_or_create(&mut self, session: Session) -> anyhow::Result<SessionRuntimeHandle> {
        let session_id = session.id.clone();
        if let None = self.runtimes.get(&session_id) {
            let (runtime, handle) = SessionRuntime::new(session.clone(), self.event_tx.clone());
            self.runtimes.insert(session_id, handle);
            // 启动 Runtime 主循环
            tokio::spawn(async move {
                runtime.run().await;
            });
        }
        if let Some(handle) = self.runtimes.get(&session_id) {
            return Ok(handle.clone());
        }
        anyhow::bail!("Session not found")
    }

    pub fn get(&self, session_id: &SessionId) -> Option<SessionRuntimeHandle> {
        self.runtimes.get(session_id).cloned()
    }

    pub fn remove(&mut self, session_id: &SessionId) {
        if let Some(handle) = self.runtimes.remove(session_id) {
            handle.disconnect();
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
            .unbounded_send(RuntimeCommand::Monitor(protocol::MonitorCommand::Start));
    }

    pub fn stop_monitor(&self) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Monitor(protocol::MonitorCommand::Stop));
    }

    pub fn list_directory(&self, path: impl Into<String>) {
        let _ = self
            .tx
            .unbounded_send(RuntimeCommand::Files(protocol::FileCommand::List {
                path: path.into(),
            }));
    }

    pub fn disconnect(&self) {
        let _ = self.tx.unbounded_send(RuntimeCommand::Disconnect);
    }
}

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
// TerminalHandle
//
// Runtime / UI 只通过这个 Handle 控制 Terminal。
// 它不拥有 SSH Channel。
// ============================================================

#[derive(Clone)]
pub struct TerminalHandle {
    cmd_tx: UnboundedSender<TerminalCommand>,
}

impl TerminalHandle {
    pub fn new(cmd_tx: UnboundedSender<TerminalCommand>) -> Self {
        Self { cmd_tx }
    }

    pub fn input(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.unbounded_send(TerminalCommand::Input { data });
    }
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

    // ========================================================
    // Runtime 主循环
    //
    // command_rx 只能由这里消费。
    //
    // 绝对不能在 open_terminal() 里面 take command_rx。
    // ========================================================

    pub async fn run(mut self) {
        if let Err(error) = self.connect().await {
            let _ = self.event_tx.unbounded_send(RuntimeEvent::Error {
                message: format!("SSH 连接失败: {error}"),
            });

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

    async fn connect(&mut self) -> anyhow::Result<()> {
        let connection = SshConnection::connect(&self.session).await?;

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
                self.open_terminal(tab_id).await?;
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

                let _ = terminal
                    .cmd_tx
                    .unbounded_send(TerminalCommand::Resize { cols, rows });
            }

            // ------------------------------------------------
            // Close
            // ------------------------------------------------
            TerminalCommand::Close => {
                if let Some(terminal) = self.terminals.remove(&tab_id) {
                    let _ = terminal.cmd_tx.unbounded_send(TerminalCommand::Close);
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

        // let terminal_bounds = normalize_terminal_bounds(TerminalBounds::default());
        // // 放到alac 中的消息通道，用来把alacrity term中解析出来的事件响应给前端ui界面处理
        // let (events_tx, events_rx) = futures::channel::mpsc::unbounded();
        // let term = new_term(terminal_bounds, events_tx.clone());

        // let handle = TerminalHandle {
        //     term,
        //     event_loop_task: Task::ready(Ok(())),
        //     id: String::new(),
        //     title: String::new(),
        //     dynamic_title: String::new(),
        //     status: String::new(),
        //     connected: false,
        //     last_content: Content {
        //         terminal_bounds,
        //         ..Default::default()
        //     },
        //     disconnected_reason: None,
        //     backend_generation: 0,
        //     backend_initialized: false,
        //     output_processor: Processor::<StdSyncHandler>::new(),
        //     events: VecDeque::with_capacity(10),
        //     keyboard_input_sent: false,
        //     init_command_startup_marker: None,
        //     init_command_startup_tx: None,

        //     scroll_pixel_y: 0.,
        //     backend: cmd_tx,
        // };
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

    async fn handle_monitor_command(
        &mut self,
        command: protocol::MonitorCommand,
    ) -> anyhow::Result<()> {
        match command {
            protocol::MonitorCommand::Start => {
                // TODO
            }

            protocol::MonitorCommand::Stop => {
                // TODO
            }

            protocol::MonitorCommand::Refresh => {
                // TODO
            }
        }

        Ok(())
    }

    // ========================================================
    // Files
    // ========================================================

    async fn handle_file_command(&mut self, command: protocol::FileCommand) -> anyhow::Result<()> {
        match command {
            protocol::FileCommand::List { path } => {
                // TODO
                //
                // self.sftp
                //     .as_ref()
                //     ...
            }

            protocol::FileCommand::Mkdir { path } => {
                // TODO
            }

            protocol::FileCommand::Delete { path } => {
                // TODO
            }

            protocol::FileCommand::Rename { from, to } => {
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
            let _ = terminal.cmd_tx.unbounded_send(TerminalCommand::Close);
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
                        ChannelMsg::Data {
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
                        ChannelMsg::ExtendedData {
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

                    Some(ChannelMsg::Close)
                    | Some(ChannelMsg::Eof)
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
