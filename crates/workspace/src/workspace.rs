//! `WorkspaceView` — the root three-pane layout of the main window.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                 Header (38 px)                      │
//! ├──────────┬─────────────────────────────┬──────────┤
//! │          │   Tabs (36 px)              │          │
//! │ Sidebar  ├─────────────────────────────┤  Files   │
//! │ (256 px) │                             │ (280 px) │
//! │          │     Terminal area            │          │
//! │          │                             │          │
//! ├──────────┴─────────────────────────────┴──────────┤
//! │                 Status bar (28 px)                │
//! └─────────────────────────────────────────────────────┘
//! ```
pub mod connection_dialog;
pub mod files;
pub mod item;
pub mod monitor;
pub mod monitor_store;
pub mod runtime_manager;
pub mod session_store;
pub mod settings;
pub mod sidebar;
pub mod state;
pub mod statusbar;
pub mod terminal_store;
pub mod transfer_store;

pub mod terminal;
pub mod title_bar;
pub mod welcome;
use std::collections::VecDeque;

use crate::{
    connection_dialog::ConnectionDialog,
    files::FilesPane,
    monitor::MonitorPanel,
    session_store::SessionStore,
    settings::SettingsView,
    sidebar::Sidebar,
    state::AppState,
    statusbar::StatusBar,
    terminal::{CloseTerminalAction, OpenTerminalAction},
    terminal_store::{TerminalEntry, TerminalStore},
    title_bar::PlatformTitleBar,
    transfer_store::TransferStore,
};
use ::settings::Settings;
use ::terminal::{TerminalBounds, TerminalBuilder};
use ::theme::{ActiveTheme, Theme};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    Root,
    resizable::{h_resizable, resizable_panel, v_resizable},
};
use log::info;
use protocol::{SessionId, TabId, monitor::MonitorStore};
use schemars::JsonSchema;
use serde::Deserialize;
use terminal_view::TerminalView;
use utils::collections::HashMap;

// actions!(workspace, [OpenTerminal, OpenNewSession]);
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema, Action)]
#[action(namespace = session_manager)]
pub struct EditAction {
    pub session_id: Option<SessionId>,
}

pub struct WorkspaceView {
    state: Entity<AppState>,
    title_bar: Entity<PlatformTitleBar>,
    sidebar: Entity<crate::sidebar::Sidebar>,
    // terminal_pane: Entity<TerminalPane>,
    files_pane: Entity<FilesPane>,
    settings_view: Entity<SettingsView>,
    status_bar: Entity<StatusBar>,
    monitor_panel: Entity<MonitorPanel>,
    session_manager: Entity<SessionStore>,
    // runtime_manager: Entity<runtime_manager::RuntimeManager>,
    tabs: Vec<TabId>,
    active_tab: Option<TabId>,
    views: ViewRegistry,
}

impl WorkspaceView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (event_tx, event_rx) = futures::channel::mpsc::unbounded();
        let session_manager = cx.new(|cx| SessionStore::new(cx));
        let terminal_store = cx.new(|cx| TerminalStore::new());
        let monitor_manager = cx.new(|cx| MonitorStore::new());
        let transfer_store = cx.new(|cx| TransferStore::new());
        let runtime_manager = runtime_manager::RuntimeManager::new(event_tx.clone());

        let state = cx.new(|cx| {
            state::AppState::new(
                runtime_manager,
                session_manager.clone(),
                terminal_store.clone(),
                monitor_manager.clone(),
                transfer_store.clone(),
            )
        });
        state.update(cx, |_this, cx| {
            AppState::start_event_dispatcher(state.clone(), event_rx, cx);
        });

        // Subscribe so any state changes re-render the chrome.
        cx.observe(&state, |_, _, cx| cx.notify()).detach();

        let sidebar = cx.new(|cx| Sidebar::new(state.clone(), session_manager.clone(), window, cx));

        let title_bar = cx.new(|cx| PlatformTitleBar::new("title_bar", cx));
        // let terminal_pane = cx.new(|cx| {
        //     let mut pane = TerminalPane::new(state.clone(), session_manager.clone(), window, cx);
        //     pane.background_task(cx);
        //     pane
        // });
        let files_pane = cx.new(|cx| FilesPane::new(state.clone()));

        let settings_view = cx.new(|cx| SettingsView::new(state.clone()));
        let status_bar = cx.new(|cx| StatusBar::new(state.clone()));
        let monitor_panel = cx.new(|cx| MonitorPanel::new(state.clone()));
        // state.update(cx, |this, cx| {
        // });
        let views = ViewRegistry::new();
        Self {
            state,
            title_bar,
            sidebar,
            // terminal_pane,
            files_pane,
            settings_view,
            status_bar,
            monitor_panel,
            session_manager,

            tabs: vec![],
            active_tab: None,
            views: views,
        }
    }
    fn open_connection_dialog(
        &mut self,
        session_id: Option<SessionId>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session_manager = self.session_manager.clone();

        cx.defer(move |cx| {
            let current_rem_size: f32 = theme_settings::ThemeSettings::get_global(cx)
                .ui_font_size(cx)
                .into();

            let default_bounds = DEFAULT_ADDITIONAL_WINDOW_SIZE;

            let default_rem_size = 16.0_f32;

            let scale_factor = current_rem_size / default_rem_size;

            let scaled_bounds: gpui::Size<Pixels> = default_bounds.map(|axis| axis * scale_factor);

            let result = cx.open_window(
                WindowOptions {
                    titlebar: None,

                    focus: true,

                    show: true,

                    is_movable: true,

                    kind: WindowKind::Dialog,

                    window_background: cx.theme().window_background_appearance(),

                    window_bounds: Some(WindowBounds::centered(scaled_bounds, cx)),

                    ..Default::default()
                },
                move |window, cx| {
                    let dialog =
                        cx.new(|cx| ConnectionDialog::new(window, cx, session_manager, session_id));

                    cx.new(|cx| Root::new(dialog, window, cx))
                },
            );

            if let Err(error) = result {
                log::error!("failed to open connection dialog: {}", error);
            }
        });
    }

    pub fn open_terminal(
        &mut self,
        action: &OpenTerminalAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("Workspace: open_terminal {}", action.session_id.to_string());
        let session = self
            .session_manager
            .read(cx)
            .query(action.session_id)
            .expect("")
            .clone();
        let tab_id = self
            .state
            .update(cx, |this, cx| this.runtime_manager.open_session(session));

        if let Ok((tab_id, handle)) = tab_id {
            let builder = TerminalBuilder::new_terminal(tab_id, TerminalBounds::default(), handle);

            let terminal = cx.new(|cx| builder.subscribe(cx));

            self.state.update(cx, |this, cx| {
                this.terminal_store.update(cx, |this, cx| {
                    this.insert(TerminalEntry {
                        tab_id,
                        session_id: SessionId::new(),
                        title: "tielte".into(),
                        runtime: terminal.clone(),
                    });
                });
            });
            let terminal_view = cx.new(|cx| TerminalView::new(terminal.clone(), window, cx));
            // ② 注册 View
            self.views.register_terminal(tab_id, terminal_view);
            self.tabs.push(tab_id);
            self.activate_tab(tab_id, window, cx);
            // self.views.update(cx, |registry, _| {
            //     registry.register_terminal(tab_id.clone(), view);
            // });
        }

        // self.terminal_pane.update(cx, |pane, cx| {
        //     pane.open_terminal(action, window, cx);
        // });
    }
    pub fn close_terminal(
        &mut self,
        action: &CloseTerminalAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("Workspace: close_terminal {}", action.tab_id.to_string());
        // self.terminal_pane.update(cx, |pane, cx| {
        //     pane.close_terminal(action, _window, cx);
        // });
    }
    fn activate_tab(&mut self, tab_id: TabId, _window: &mut Window, cx: &mut Context<Self>) {
        self.active_tab = Some(tab_id);

        cx.notify();
    }
    pub fn edit_session(
        &mut self,
        action: &EditAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_connection_dialog(action.session_id, window, cx);
    }
    fn render_tab_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.theme();

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
            .when_some(self.active_tab, |e, active_tab| {
                e.children(self.tabs.iter().map(|tab_id| {
                    let tab_id = tab_id.clone(); // 复制一份
                    div()
                        .id(format!("terminal-tab-{}", tab_id))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .px(px(12.))
                        .h_full()
                        .border_r_1()
                        .border_color(t.colors().border)
                        .bg(if active_tab == tab_id {
                            t.colors().element_background
                        } else {
                            Hsla::transparent_black()
                        })
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _event, window, cx| {
                                this.activate_tab(tab_id, window, cx);
                            }),
                        )
                        .child(div().size(px(6.)).rounded_full().bg(t.colors().icon_accent))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(t.colors().text)
                                .when_some(
                                    self.state.read(cx).terminal_store.read(cx).get(&tab_id),
                                    |e, terminal| e.child(terminal.title.clone()),
                                ), // .when_none(&terminal, |e| e.child(tab_id.to_string())),
                        )
                        .child(
                            div()
                                .id(format!("terminal-tab-close-{}", tab_id.to_string()))
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
                                    cx.listener(
                                        move |_this, _event: &MouseDownEvent, window, cx| {
                                            window.dispatch_action(
                                                Box::new(CloseTerminalAction {
                                                    tab_id: tab_id.clone(),
                                                }),
                                                cx,
                                            );
                                        },
                                    ),
                                ),
                        )
                }))
            })
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
                            window.dispatch_action(Box::new(EditAction { session_id: None }), cx);
                        }),
                    ),
            )
            .into_any_element()
    }
    fn render_content(&self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(tab_id) = self.active_tab {
            if let Some(view) = self.views.get_terminal(&tab_id) {
                return div().size_full().child(view.clone()).into_any_element();
            }
        }

        div().size_full().into_any_element()
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(session_id) = self
            .state
            .update(cx, |state, _cx| state.pending_open_session_id.take())
        {
            // self.terminal_pane.update(cx, |pane, cx| {
            //     // pane.open_terminal(&OpenTerminalAction { session_id }, windows, cx);
            // });
        }
        self.title_bar.update(cx, |this, cx| {
            let t = cx.theme();
            this.set_children([div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .h_full()
                        // .h(px(t.layout.header_height))
                        .px(px(16.0))
                        .bg(t.colors().background)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                // .ml(px(16.0))
                                .text_color(t.colors().text)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(
                                    // logo glyph (>) — replace with SVG path in production
                                    div().text_color(t.styles.colors.icon_accent).child(">"),
                                )
                                .child("Termi"),
                        )
                        // top-level menu
                        .child(
                            div()
                                .flex()
                                .gap(px(2.0))
                                .ml(px(24.0))
                                .child(menu_button("File", &t))
                                .child(menu_button("Edit", &t))
                                .child(menu_button("View", &t))
                                .child(menu_button("Window", &t))
                                .child(menu_button("Help", &t)),
                        ),
                )
                .into_any_element()]);
        });
        let t = cx.theme();

        div()
            .id("lumen-workspace")
            .on_action(cx.listener(Self::open_terminal))
            .on_action(cx.listener(Self::close_terminal))
            .on_action(cx.listener(Self::edit_session))
            .flex()
            .flex_col()
            .size_full()
            .bg(t.colors().background)
            // ============== HEADER ==============
            .child(
                div()
                    .child(self.title_bar.clone())
                    .border_b_1()
                    .border_color(t.colors().border),
            )
            .child(
                div().flex_1().overflow_hidden().child(
                    v_resizable("main-layout")
                        .child(
                            resizable_panel().overflow_hidden().child(
                                h_resizable("nested-layout")
                                    .child(
                                        resizable_panel()
                                            .size(px(300.))
                                            .size_range(px(280.)..px(500.))
                                            .child(self.sidebar.clone()),
                                    )
                                    .child(
                                        resizable_panel().child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .size_full()
                                                .bg(t.colors().background)
                                                .child(self.render_tab_bar(windows, cx))
                                                .child(self.render_content(windows, cx)),
                                            // .child(self.terminal_pane.clone()),
                                        ),
                                    )
                                    .child(
                                        resizable_panel()
                                            // .size(px(300.))
                                            // .size_range(px(300.)..px(500.))
                                            .child(self.files_pane.clone()),
                                    ),
                            ),
                        )
                        .child(
                            resizable_panel()
                                .overflow_hidden()
                                .size(px(192.))
                                .child(self.monitor_panel.clone()),
                        ),
                ),
            )
            .child(self.status_bar.clone())
    }
}

fn menu_button(label: &'static str, t: &Theme) -> impl IntoElement {
    div()
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(4.0))
        // .text_color(t)
        .text_size(px(12.0))
        .cursor_pointer()
        .hover(|s| s.bg(t.colors().background).text_color(t.colors().text))
        .child(label)
}

pub struct ViewRegistry {
    terminal_views: HashMap<TabId, Entity<TerminalView>>,
}
impl ViewRegistry {
    pub fn new() -> Self {
        Self {
            terminal_views: HashMap::default(),
        }
    }

    pub fn register_terminal(&mut self, tab_id: TabId, view: Entity<TerminalView>) {
        self.terminal_views.insert(tab_id, view);
    }
    pub fn get_terminal(&self, tab_id: &TabId) -> Option<&Entity<TerminalView>> {
        self.terminal_views.get(tab_id)
    }
    pub fn remove_terminal(&mut self, tab_id: &TabId) -> Option<Entity<TerminalView>> {
        self.terminal_views.remove(tab_id)
    }
}
