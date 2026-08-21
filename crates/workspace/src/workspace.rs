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
pub mod session_manager;
pub mod settings;
pub mod sidebar;
pub mod state;
pub mod statusbar;
pub mod tabs;
pub mod terminal;
pub mod title_bar;
pub mod welcome;
use crate::{
    files::FilesPane, monitor::MonitorPanel, session_manager::SessionManager,
    settings::SettingsView, sidebar::Sidebar, state::AppState, statusbar::StatusBar, tabs::TabsBar,
    terminal::TerminalPane, title_bar::PlatformTitleBar,
};
use ::theme::{ActiveTheme, Theme};
use gpui::*;
use ui::pane::{Pane, PaneLayout};

// actions!(workspace, [OpenTerminal, OpenNewSession]);

pub struct WorkspaceView {
    state: Entity<AppState>,
    title_bar: Entity<PlatformTitleBar>,
    sidebar: Entity<crate::sidebar::Sidebar>,
    tabsbar: Entity<TabsBar>,
    terminal_pane: Entity<TerminalPane>,
    files_pane: Entity<FilesPane>,
    settings_view: Entity<SettingsView>,
    status_bar: Entity<StatusBar>,
    monitor_panel: Entity<MonitorPanel>,
    session_manager: Entity<SessionManager>,
}

impl WorkspaceView {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Subscribe so any state changes re-render the chrome.
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let session_manager = cx.new(|cx| SessionManager::new());
        let sidebar = cx.new(|cx| {
            Sidebar::new(
                state.clone(),
                // connection_dialog.clone(),
                session_manager.clone(),
            )
        });

        let title_bar = cx.new(|cx| PlatformTitleBar::new("title_bar", cx));
        let tabsbar = cx.new(|cx| TabsBar::new(state.clone(), session_manager.clone()));
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
            tabsbar,
            terminal_pane,
            files_pane,
            settings_view,
            status_bar,
            monitor_panel,
            session_manager,
        }
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        info!("渲染");
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
                        // .border_b_1()
                        // .border_color(t.colors().border)
                        // brand
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

        // Choose the central canvas based on active_view.
        // let canvas: AnyElement = div()
        //     .flex()
        //     .flex_row()
        //     .flex_1()
        //     .min_h_0()
        //     .child()
        //     .child(
        //         div()
        //             .flex()
        //             .flex_col()
        //             .flex_1()
        //             .min_w_0()
        //             .min_h_0()
        //             .bg(t.colors().background)
        //             .child(self.tabsbar.clone())
        //             .child(self.terminal_pane.clone()),
        //     )
        //     .child(self.files_pane.clone())
        //     .into_any_element();

        div()
            .id("lumen-workspace")
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
            // .child(render_header(&t, windows, cx))
            // ============== CANVAS ==============
            // .child(canvas)
            // ============== MONITOR (workspace mode only) ==============
            .child(
                PaneLayout::vertical()
                    .child(
                        Pane::new().child(
                            PaneLayout::horizontal()
                                .child(Pane::new().child(self.sidebar.clone()))
                                .child(Pane::new().child(self.terminal_pane.clone()))
                                .child(Pane::new().child(self.files_pane.clone())),
                        ),
                    )
                    .child(Pane::new().child(self.monitor_panel.clone())),
            )
            // ============== STATUS BAR ==============
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
