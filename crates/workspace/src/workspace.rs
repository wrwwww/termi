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
pub mod monitor;
pub mod settings;
pub mod sidebar;
pub mod state;
pub mod statusbar;
pub mod tabs;
pub mod terminal;
// pub mod theme;
pub mod session_manager;
pub mod title_bar;
use crate::{
    connection_dialog::ConnectionDialog,
    files::FilesPane,
    monitor::MonitorPanel,
    session_manager::SessionManager,
    settings::SettingsView,
    sidebar::Sidebar,
    state::{ActiveView, AppState},
    statusbar::StatusBar,
    tabs::TabsBar,
    terminal::TerminalPane,
};
use ::theme::{ActiveTheme, Theme};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{Root, TitleBar, white};
use log::info;

actions!(workspace, [OpenTerminal, OpenNewSession]);

pub struct WorkspaceView {
    state: Entity<AppState>,

    sidebar: Entity<crate::sidebar::Sidebar>,
    tabsbar: Entity<TabsBar>,
    terminal_pane: Entity<TerminalPane>,
    files_pane: Entity<FilesPane>,
    // connection_dialog: Entity<ConnectionDialog>,
    settings_view: Entity<SettingsView>,
    status_bar: Entity<StatusBar>,

    monitor_panel: Entity<MonitorPanel>,
}

impl WorkspaceView {
    pub fn new(state: Entity<AppState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Subscribe so any state changes re-render the chrome.
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        let session_manager = cx.new(|cx| SessionManager::new());
        // let connection_dialog =
        // cx.new(|cx| ConnectionDialog::new(window, cx, session_manager.clone()));
        let sidebar = cx.new(|cx| {
            Sidebar::new(
                state.clone(),
                // connection_dialog.clone(),
                session_manager.clone(),
            )
        });
        let tabsbar = cx.new(|cx| TabsBar::new(state.clone(), session_manager.clone()));
        let terminal_pane = cx.new(|cx| TerminalPane::new(state.clone()));
        let files_pane = cx.new(|cx| FilesPane::new(state.clone()));

        let settings_view = cx.new(|cx| SettingsView::new(state.clone()));
        let status_bar = cx.new(|cx| StatusBar::new(state.clone()));
        let monitor_panel = cx.new(|cx| MonitorPanel::new(state.clone()));

        Self {
            state,

            sidebar,
            tabsbar,
            terminal_pane,
            files_pane,
            // connection_dialog,
            settings_view,
            status_bar,
            monitor_panel,
        }
    }
    pub fn open_terminal(
        &mut self,
        action: &OpenTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        info!("收到打开终端action");
    }
    pub fn open_new_session(
        &mut self,
        _: &OpenNewSession,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        info!("收到打开终端action");
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(windows, cx);
        let t = cx.theme();

        // Choose the central canvas based on active_view.
        let canvas: AnyElement = div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h_0()
            .child(self.sidebar.clone())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .bg(t.colors().background)
                    .child(self.tabsbar.clone())
                    .child(self.terminal_pane.clone()),
            )
            .child(self.files_pane.clone())
            .into_any_element();

        div()
            .id("lumen-workspace")
            .flex()
            .on_action(cx.listener(Self::open_new_session))
            .on_action(
                cx.listener(|this: &mut WorkspaceView, _: &OpenTerminal, _window, _cx| {
                    info!("收到 OpenTerminal");
                }),
            )
            .flex_col()
            .size_full()
            .bg(t.colors().background)
            // ============== HEADER ==============
            .child(
                TitleBar::new()
                    .bg(t.colors().background)
                    .border_color(t.colors().border)
                    .text_color(white())
                    .child(
                        div().flex().items_center().gap_3().child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .h_full()
                                // .h(px(t.layout.header_height))
                                .px(px(16.0))
                                .bg(t.colors().background)
                                .border_b_1()
                                .border_color(t.colors().border)
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
                                            div()
                                                .text_color(t.styles.colors.icon_accent)
                                                .child(">"),
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
                        ),
                    ),
            )
            // .child(render_header(&t, windows, cx))
            // ============== CANVAS ==============
            .child(canvas)
            // ============== MONITOR (workspace mode only) ==============
            .child(self.monitor_panel.clone())
            // ============== STATUS BAR ==============
            .child(self.status_bar.clone())
            .children(dialog_layer)
    }
}
struct TitleBarState {
    should_move: bool,
}
impl Render for TitleBarState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn menu_button(label: &'static str, t: &Theme) -> impl IntoElement {
    div()
        // .id(("menu", label))
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(4.0))
        // .text_color(t)
        .text_size(px(12.0))
        .cursor_pointer()
        .hover(|s| s.bg(t.colors().background).text_color(t.colors().text))
        .child(label)
}

// fn header_icon_button(t: &Theme) -> impl IntoElement {
//     div()
//         .size(px(28.0))
//         .flex()
//         .items_center()
//         .justify_center()
//         .rounded(px(4.0))
//         .text_color(t.colors().text_muted)
//         .cursor_pointer()
//         .hover(|s| s.bg(t.colors().background).text_color(t.colors().text))
//         // placeholder dot — replace with svg::path()
//         .child(
//             div()
//                 .size(px(14.0))
//                 .rounded_full()
//                 .border_1()
//                 .border_color(t.colors().border_variant),
//         )
// }

// impl RenderOnce for TitleBar {
//     fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
//         let t = cx.theme();

//         div()
//             .flex()
//             .flex_row()
//             .items_center()
//             .h(px(t.layout.header_height))
//             .px(px(16.0))
//             .bg(t.colors().background)
//             .border_b_1()
//             .border_color(t.colors().border.border)
//             // brand
//             .child(
//                 div()
//                     .flex()
//                     .items_center()
//                     .gap(px(8.0))
//                     .ml(px(16.0))
//                     .text_color(t.colors().text)
//                     .font_weight(FontWeight::SEMIBOLD)
//                     .child(
//                         // logo glyph (>) — replace with SVG path in production
//                         div().text_color(t.colors().icon_accent).child(">"),
//                     )
//                     .child("Lumen"),
//             )
//             // top-level menu
//             .child(
//                 div()
//                     .flex()
//                     .gap(px(2.0))
//                     .ml(px(24.0))
//                     .child(menu_button("File", &t))
//                     .child(menu_button("Edit", &t))
//                     .child(menu_button("View", &t))
//                     .child(menu_button("Window", &t))
//                     .child(menu_button("Help", &t)),
//             )
//             // spacer
//             .child(
//                 div()
//                     .flex_1()
//                     .h_full()
//                     .window_control_area(WindowControlArea::Drag),
//             ) // header actions (right-aligned)
//             .child(AppMenuBar::new(cx))
//     }
// }
