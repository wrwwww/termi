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
pub mod monitor_manager;
pub mod runtime_manager;
pub mod session_store;
pub mod settings;
pub mod sidebar;
pub mod state;
pub mod statusbar;
pub mod transfer_manager;

pub mod terminal;
pub mod title_bar;
pub mod welcome;
use crate::{
    connection_dialog::ConnectionDialog,
    files::FilesPane,
    monitor::MonitorPanel,
    session_store::SessionStore,
    settings::SettingsView,
    sidebar::Sidebar,
    state::AppState,
    statusbar::StatusBar,
    terminal::{CloseTerminalAction, OpenTerminalAction, TerminalPane},
    title_bar::PlatformTitleBar,
};
use ::settings::Settings;
use ::theme::{ActiveTheme, Theme};
use gpui::*;
use gpui_component::{
    Root,
    resizable::{h_resizable, resizable_panel, v_resizable},
};
use protocol::SessionId;
use schemars::JsonSchema;
use serde::Deserialize;

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
    // tabsbar: Entity<TabsBar>,
    terminal_pane: Entity<TerminalPane>,
    files_pane: Entity<FilesPane>,
    settings_view: Entity<SettingsView>,
    status_bar: Entity<StatusBar>,
    monitor_panel: Entity<MonitorPanel>,
    session_manager: Entity<SessionStore>,
    runtime_manager: Entity<runtime_manager::RuntimeManager>,
}

impl WorkspaceView {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (event_tx, event_rx) = futures::channel::mpsc::unbounded();
        // Subscribe so any state changes re-render the chrome.
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let session_manager = cx.new(|cx| SessionStore::new(state.clone(), cx));
        let runtime_manager = cx.new(|cx| {
            let mut runtime = runtime_manager::RuntimeManager::new(event_tx.clone());

            runtime
        });
        let sidebar = cx.new(|cx| Sidebar::new(state.clone(), session_manager.clone(), window, cx));

        let title_bar = cx.new(|cx| PlatformTitleBar::new("title_bar", cx));
        let terminal_pane = cx.new(|cx| {
            let mut pane = TerminalPane::new(state.clone(), session_manager.clone(), window, cx);
            pane.background_task(cx);
            pane
        });
        let files_pane = cx.new(|cx| FilesPane::new(state.clone()));

        let settings_view = cx.new(|cx| SettingsView::new(state.clone()));
        let status_bar = cx.new(|cx| StatusBar::new(state.clone()));
        let monitor_panel = cx.new(|cx| MonitorPanel::new(state.clone()));

        Self {
            state,
            title_bar,
            sidebar,
            terminal_pane,
            files_pane,
            settings_view,
            status_bar,
            monitor_panel,
            session_manager,
            runtime_manager,
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
        self.runtime_manager.update(cx, |manager, cx| {
            let _ = manager.open_session(session);
        });
        self.terminal_pane.update(cx, |pane, cx| {
            pane.open_terminal(action, window, cx);
        });
    }
    pub fn close_terminal(
        &mut self,
        action: &CloseTerminalAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!("Workspace: close_terminal {}", action.tab_id.to_string());
        self.terminal_pane.update(cx, |pane, cx| {
            pane.close_terminal(action, _window, cx);
        });
    }
    pub fn edit_session(
        &mut self,
        action: &EditAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_connection_dialog(action.session_id, window, cx);
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(session_id) = self
            .state
            .update(cx, |state, _cx| state.pending_open_session_id.take())
        {
            self.terminal_pane.update(cx, |pane, cx| {
                pane.open_terminal(&OpenTerminalAction { session_id }, windows, cx);
            });
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
                                                .child(self.terminal_pane.clone()),
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
