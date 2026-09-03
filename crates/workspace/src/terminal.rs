//! `TerminalPane` — the actual terminal viewport.
//!
//! In a real implementation this would be backed by `portable-pty` +
//! `vte::Parser` to translate ANSI escape sequences into styled glyphs.
//! This reference paints a representative static frame.

use crate::EditAction;
use crate::{session_manager::SessionStore, state::AppState, welcome::WelcomePage};
use anyhow::Ok;
use futures::StreamExt;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gpui::{prelude::FluentBuilder, *};
use log::{error, info};
use protocol::{SessionId, SystemEvent, TabId};
use schemars::JsonSchema;
use serde::Deserialize;
use terminal::{TerminalBounds, TerminalBuilder};
use terminal_view::TerminalView;
use theme::{ActiveTheme, Theme};
use uuid::Uuid;
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = workspace)]
pub struct OpenTerminalAction {
    pub session_id: SessionId,
}
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = workspace)]
pub struct ActivateTerminalAction {
    pub tab_id: TabId,
}
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = workspace)]
pub struct CloseTerminalAction {
    pub tab_id: TabId,
}

struct TerminalTab {
    tab_id: TabId,
    session_id: SessionId,
    title: SharedString,
    view: Entity<TerminalView>,
}
pub struct TerminalPane {
    state: Entity<AppState>,
    session_manager: Entity<SessionStore>,
    focus_handle: FocusHandle,
    items: Vec<TerminalTab>,
    active_item_index: usize,
    should_display_welcome_page: bool,
    welcome_page: Option<Entity<WelcomePage>>,
    // 接受从backend返回的事件
    events_rx: Option<UnboundedReceiver<SystemEvent>>,
    events_tx: UnboundedSender<SystemEvent>,
    event_loop_task: Task<Result<(), anyhow::Error>>,
}

impl TerminalPane {
    pub fn new(
        state: Entity<AppState>,
        session_manager: Entity<SessionStore>,
        windows: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let welcome_page = cx.new(|cx| WelcomePage::new(true, windows, cx));
        let (events_tx, events_rx) = unbounded();
        Self {
            state,
            session_manager,
            welcome_page: Some(welcome_page),

            focus_handle: cx.focus_handle(),
            items: vec![],
            active_item_index: 0,
            should_display_welcome_page: false,
            events_rx: Some(events_rx),
            events_tx: events_tx,

            event_loop_task: Task::ready(Ok(())),
        }
    }
    pub fn background_task(&mut self, cx: &mut Context<Self>) {
        let mut events_rx = self.events_rx.take().unwrap();
        self.event_loop_task = cx.spawn(async move |this, cx| {
            while let Some(event) = events_rx.next().await {
                this.update(cx, |this, cx| {
                    //write_output
                    this.process_event(event, cx);
                })
                .unwrap();
            }
            anyhow::Ok(())
        });
    }
    pub fn process_event(&mut self, event: SystemEvent, cx: &mut Context<Self>) {
        match event {
            SystemEvent::Output { tab_id, bytes } => {
                if let Some(tab) = self.items.iter().find(|tab| tab.tab_id == tab_id) {
                    tab.view.update(cx, |this, cx| {
                        this.terminal()
                            .update(cx, |this, _cx| this.write_output(&bytes))
                    });
                }
            }
            SystemEvent::Status { tab_id, text } => {}
            SystemEvent::Connected { tab_id } => {
                info!("连接成功，tab_id: {:?}", tab_id);
            }
            SystemEvent::Error { tab_id, message } => {
                let output = format!("\r\n连接失败：{message}\r\n");
                info!("{}", output);
                if let Some(tab) = self.items.iter().find(|tab| tab.tab_id == tab_id) {

                    // tab.view.update(cx, |this, cx| {
                    //     this.terminal()
                    //         .update(cx, |this, _cx| this.write_output(output.as_bytes()))
                    // });
                }
            }
            SystemEvent::CommandComplete(_) => {}
            SystemEvent::TitleUpdate { tab_id, title } => {}
            SystemEvent::ClearScreen => {}
            SystemEvent::ProcessStarted(_) => {}
            SystemEvent::ProcessTerminated => {}
        }
    }

    pub fn open_terminal(
        &mut self,
        action: &OpenTerminalAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = TabId::new();
        let session_id = action.session_id.clone();

        let session = self
            .session_manager
            .read(cx)
            .query(action.session_id)
            .unwrap()
            .clone();
        let title = session.name.clone().into();
        info!("接受到打开session{:?}", session);
        let builder = TerminalBuilder::new_terminal(TerminalBounds::default());

        let terminal =
            cx.new(|cx| builder.subscribe(session, tab_id.clone(), self.events_tx.clone(), cx));

        let terminal_view = cx.new(|cx| TerminalView::new(terminal.clone(), window, cx));
        self.items.push(TerminalTab {
            tab_id: tab_id.clone(),
            session_id: session_id,
            title: title,
            view: terminal_view,
        });
        self.active_item_index = self.items.len() - 1;
        self.should_display_welcome_page = false;
        self.state.update(cx, |state, cx| {
            if !state.open_session_ids.contains(&tab_id) {
                state.open_session_ids.push(tab_id.clone());
            }
            state.set_active_tab(&tab_id);
            cx.notify();
        });
        cx.notify();
    }
    pub fn activate_terminal(
        &mut self,
        action: &ActivateTerminalAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self
            .items
            .iter()
            .position(|tab| tab.tab_id == action.tab_id)
        {
            self.active_item_index = index;
            self.state.update(cx, |state, cx| {
                state.set_active_tab(&action.tab_id);
                cx.notify();
            });
            cx.notify();
        }
    }
    pub fn close_terminal(
        &mut self,
        action: &CloseTerminalAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self
            .items
            .iter()
            .position(|tab| tab.tab_id == action.tab_id)
        {
            self.items.remove(index);
            self.active_item_index = self
                .active_item_index
                .saturating_sub(usize::from(index <= self.active_item_index));
            let next_id = self
                .items
                .get(self.active_item_index)
                .map(|tab| tab.tab_id.clone());
            self.state.update(cx, |state, cx| {
                state.open_session_ids.retain(|id| id != &action.tab_id);
                state.active_tab_id = next_id;
                cx.notify();
            });
            cx.notify();
        }
    }
}

impl Render for TerminalPane {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme();
        let state = self.state.read(cx);
        let active_id = state.active_tab_id.clone();

        div()
            .on_action(cx.listener(Self::activate_terminal))
            .on_action(cx.listener(Self::close_terminal))
            .track_focus(&self.focus_handle)
            .flex_1()
            .size_full()
            .id("lumen-terminal")
            .when_else(
                self.should_display_welcome_page || self.items.is_empty(),
                |e| e.child(self.welcome_page.as_ref().unwrap().clone()),
                |e| {
                    e.flex()
                        .flex_col()
                        .size_full() // ← 关键：让列容器占满父容器
                        .child(
                            div()
                                .id("terminal-tabs")
                                .flex()
                                .flex_row()
                                .items_center()
                                .h(px(36.))
                                // ← 防止 tabs 被压缩
                                .bg(t.colors().background)
                                .border_b_1()
                                .border_color(t.colors().border)
                                .overflow_x_scroll()
                                .children(
                                    self.items
                                        .iter()
                                        .map(|tab| render_tab(tab, active_id, &t, &cx)),
                                )
                                .child(
                                    div()
                                        .px(px(12.))
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .text_color(t.colors().text_muted)
                                        .child("+")
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |_this, _event, window, cx| {
                                                window.dispatch_action(
                                                    Box::new(EditAction { session_id: None }),
                                                    cx,
                                                );
                                            }),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .id("terminal-content2")
                                .flex_1() // ← 占据剩余空间
                                .min_h_0() // ← 重要：允许 flex 子项收缩
                                .bg(t.colors().terminal_background)
                                .child(
                                    self.items[self.active_item_index].view.clone(), // ← 确保完全填充
                                ),
                        )
                },
            )
    }
}

fn render_tab(
    tab: &TerminalTab,
    active_id: Option<TabId>,
    t: &Theme,
    cx: &&mut Context<TerminalPane>,
) -> AnyElement {
    let tab_id = tab.tab_id;
    let is_active = active_id == Some(tab_id);
    let activate_id = tab_id.clone();
    let close_id = tab_id.clone();
    div()
        .id(format!("terminal-tab-{}", activate_id.to_string()))
        .flex()
        .items_center()
        .gap(px(8.))
        .px(px(12.))
        .h_full()
        .border_r_1()
        .border_color(t.colors().border)
        .bg(if is_active {
            t.colors().element_background
        } else {
            Hsla::transparent_black()
        })
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |_this, _event, window, cx| {
                window.dispatch_action(
                    Box::new(ActivateTerminalAction {
                        tab_id: activate_id.clone(),
                    }),
                    cx,
                );
            }),
        )
        .child(div().size(px(6.)).rounded_full().bg(t.colors().icon_accent))
        .child(
            div()
                .text_size(px(12.5))
                .text_color(t.colors().text)
                .child(tab.title.clone()),
        )
        .child(
            div()
                .id(format!("terminal-tab-close-{}", close_id.to_string()))
                .size(px(18.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .text_color(t.colors().text_placeholder)
                .hover(|style| style.bg(t.colors().elevated_surface_background))
                .child("×")
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_this, _event: &MouseDownEvent, window, cx| {
                        window.dispatch_action(
                            Box::new(CloseTerminalAction {
                                tab_id: close_id.clone(),
                            }),
                            cx,
                        );
                    }),
                ),
        )
        .into_any_element()
}

// fn sample_lines(t: &Theme, _session: Option<&str>) -> Vec<AnyElement> {
//     // use crate::TerminalPalette;
//     // let pal = &t.terminal;

//     // let prompt = |user: &str, path: &str, cmd: &str| -> AnyElement {
//     //     div()
//     //         .flex()
//     //         .flex_row()
//     //         .children([
//     //             div().text_color(pal.prompt).child(text!(user)),
//     //             div().text_color(t.colors().icon_accent).child(text!(":")),
//     //             div().text_color(pal.path).child(text!(path)),
//     //             div().text_color(t.colors().icon_accent).child(text!("$ ")),
//     //             div().text_color(pal.text).child(text!(cmd)),
//     //         ])
//     //         .into_any_element()
//     // };

//     let plain = |s: &str, color: Hsla| -> AnyElement {
//         div().text_color(color).child(text!(s)).into_any_element()
//     };

//     let blank = || -> AnyElement { div().child(" ").into_any_element() };

//     vec![
//         plain(
//             "Last login: Tue Aug  6 09:08:14 2026 from 10.0.99.42",
//             pal.gray,
//         ),
//         prompt("deploy@prod-web-01", "~", "ls -la"),
//         plain("total 36", pal.text),
//         plain("drwx------  1 deploy deploy  240 Aug  6 09:08 .", pal.text),
//         plain(
//             "drwxr-xr-x  1 root    root     72 Aug  6 08:55 ..",
//             pal.text,
//         ),
//         plain(
//             "-rw-r--r--  1 deploy deploy  220 Apr 18  2024 .bash_logout",
//             pal.text,
//         ),
//         plain(
//             "-rw-r--r--  1 deploy deploy 3.7K Apr 18  2024 .bashrc",
//             pal.text,
//         ),
//         plain(
//             "drwxr-xr-x  3 deploy deploy   18 Aug  6 09:00 .config",
//             pal.text,
//         ),
//         plain(
//             "drwxr-xr-x  5 deploy deploy   72 Aug  6 08:50 .npm",
//             pal.text,
//         ),
//         blank(),
//         prompt("deploy@prod-web-01", "~", "uptime"),
//         div()
//             .flex()
//             .flex_row()
//             .children([
//                 div()
//                     .text_color(pal.text)
//                     .child(" 09:12:30 up 42 days, 14:08,  1 user,  load average: "),
//                 div().text_color(pal.amber).child("0.05"),
//                 div().text_color(pal.text).child(", "),
//                 div().text_color(pal.amber).child("0.12"),
//                 div().text_color(pal.text).child(", "),
//                 div().text_color(pal.amber).child("0.08"),
//             ])
//             .into_any_element(),
//         blank(),
//         prompt("deploy@prod-web-01", "~", "systemctl status nginx"),
//         div()
//             .flex()
//             .flex_row()
//             .children([
//                 div().text_color(pal.prompt).child("●"),
//                 div().ml(px(4.0)).text_color(pal.text).child(
//                     " nginx.service - A high performance web server and a reverse proxy server",
//                 ),
//             ])
//             .into_any_element(),
//         plain(
//             "   Loaded: loaded (/lib/systemd/system/nginx.service; enabled; vendor preset: enabled)",
//             pal.text,
//         ),
//         div()
//             .flex()
//             .flex_row()
//             .children([
//                 div().text_color(pal.text).child("   Active: "),
//                 div().text_color(pal.prompt).child("active (running)"),
//                 div()
//                     .text_color(pal.text)
//                     .child(" since Mon 2026-06-15 09:24:32 UTC; 1 month 11 days ago"),
//             ])
//             .into_any_element(),
//         plain("     Docs: man:nginx(8)", pal.text),
//         plain(
//             "  Process: 1234 ExecStartPre=/usr/sbin/nginx -t -q -g daemon on; master_process on; (code=exited, status=0/SUCCESS)",
//             pal.text,
//         ),
//         plain(
//             "  Process: 1235 ExecStart=/usr/sbin/nginx -g daemon on; master_process on; (code=exited, status=0/SUCCESS)",
//             pal.text,
//         ),
//         plain(" Main PID: 1235 (nginx)", pal.text),
//         plain("    Tasks: 5 (limit: 4915)", pal.text),
//         plain("   Memory: 18.4M", pal.text),
//         plain("      CPU: 1.215s", pal.text),
//         plain("   CGroup: /system.slice/nginx.service", pal.text),
//         plain(
//             "           ├─1235 \"nginx: master process /usr/sbin/nginx -g daemon on; master_process on;\"",
//             pal.purple,
//         ),
//         plain("           ├─1236 \"nginx: worker process\"", pal.purple),
//         plain("           ├─1237 \"nginx: worker process\"", pal.purple),
//         plain("           └─1238 \"nginx: worker process\"", pal.purple),
//         blank(),
//         prompt("deploy@prod-web-01", "~", ""),
//         // blinking cursor glyph
//         div()
//             .flex()
//             .flex_row()
//             .items_center()
//             .child(
//                 div()
//                     .id("terminal-cursor")
//                     .w(px(7.0))
//                     .h(px(14.0))
//                     .bg(pal.text)
//                     .ml(px(2.0)),
//             )
//             .into_any_element(),
//     ]
// }
