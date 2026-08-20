//! `Sidebar` — left-pane session list, grouped by group name.
//!
//! Each group is collapsible. Selecting an entry sets the active session
//! and (in the real app) brings the tab forward.

use crate::{
    connection_dialog::ConnectionDialog,
    session_manager::{self, SessionManager},
    state::AppState,
    terminal::OpenTerminalAction,
};
use gpui::*;
use gpui_component::{IconName, Root, button::Button, menu::ContextMenuExt};
use log::info;
use protocol::{Session, SessionStatus};
use schemars::JsonSchema;
use serde::Deserialize;
use settings::Settings;
use theme::ActiveTheme;

// actions!(session_manager, [Edit, Copy, Delete]);
fn default_1() -> usize {
    1
}
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = session_manager)]
pub struct EditAction {
    session_id: String,
}
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = session_manager)]
pub struct CopyAction {
    session_id: String,
}
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = session_manager)]
pub struct DeleteAction {
    session_id: String,
}
pub struct Sidebar {
    state: Entity<AppState>,
    // Persisted client-side UI state (not serialised).
    collapsed: std::collections::HashSet<String>,
    search_query: String,

    session_manager: Entity<SessionManager>,
}

impl Sidebar {
    pub fn new(state: Entity<AppState>, session_manager: Entity<SessionManager>) -> Self {
        let mut collapsed = std::collections::HashSet::new();
        collapsed.insert("Personal".into()); // match `preview.html` collapsed group
        Self {
            state,
            collapsed,
            search_query: String::new(),
            session_manager,
        }
    }

    pub fn del_session(
        &mut self,
        action: &DeleteAction,
        window: &mut Window,
        cx: &mut Context<'_, Sidebar>,
    ) {
        let session_id = &action.session_id;
        info!("xx{}", session_id);
        self.session_manager.update(cx, |this, cx| {
            this.del_session(session_id);
            info!("xx{}", this.list().len());
        });
    }
    pub fn edit_session(
        &mut self,
        action: &EditAction,
        window: &mut Window,
        cx: &mut Context<'_, Sidebar>,
    ) {
    }
    pub fn copy_session(
        &mut self,
        action: &CopyAction,
        window: &mut Window,
        cx: &mut Context<'_, Sidebar>,
    ) {
        let session_id = action.session_id.clone();
        self.session_manager.update(cx, |this, _cx| {
            this.copy_session(&session_id);
        });
    }
}

impl Render for Sidebar {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme();
        let grouped = self.session_manager.read(cx).grouped_sessions();
        let active_id = self.state.read(cx).active_session_id.clone();
        let query = self.search_query.to_lowercase();

        let groups: Vec<_> = grouped
            .into_iter()
            .map(|(group_name, mut sessions)| {
                // Filter by search query.
                if !query.is_empty() {
                    sessions.retain(|s| {
                        s.name.to_lowercase().contains(&query)
                            || s.hostname.to_lowercase().contains(&query)
                    });
                }
                (group_name, sessions)
            })
            .filter(|(_, s)| query.is_empty() || !s.is_empty())
            .collect();

        div()
            .on_action(cx.listener(Self::del_session))
            .on_action(cx.listener(Self::edit_session))
            .on_action(cx.listener(Self::copy_session))
            .flex()
            .flex_col()
            .w(px(256.))
            .h_full()
            .bg(t.colors().background)
            .border_r_1()
            .border_color(t.colors().border)
            // ============ Toolbar row ============
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .h(px(44.))
                    .px(px(12.0))
                    .gap(px(8.0))
                    .border_b_1()
                    .border_color(t.colors().border)
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.colors().icon_accent)
                            .child("SESSIONS"),
                    )
                    .text_color(t.colors().text_accent)
                    .child(Button::new("1").icon(IconName::Redo2).on_click(|_, _, _| {
                        info!("点击事件");
                    }))
                    .child(Button::new("2").icon(IconName::Plus).on_click(cx.listener({
                        |this, _, window, cx| {
                            let session_manager = this.session_manager.clone();

                            cx.defer(|cx| {
                                let current_rem_size: f32 =
                                    theme_settings::ThemeSettings::get_global(cx)
                                        .ui_font_size(cx)
                                        .into();

                                let default_bounds = DEFAULT_ADDITIONAL_WINDOW_SIZE;
                                let default_rem_size = 16.0;
                                let scale_factor = current_rem_size / default_rem_size;
                                let scaled_bounds: gpui::Size<Pixels> =
                                    default_bounds.map(|axis| axis * scale_factor);

                                cx.open_window(
                                    WindowOptions {
                                        titlebar: Some(TitlebarOptions {
                                            title: None,
                                            appears_transparent: true,
                                            traffic_light_position: Some(point(px(12.0), px(12.0))),
                                        }),
                                        focus: true,
                                        show: true,
                                        is_movable: true,
                                        kind: gpui::WindowKind::Dialog,
                                        window_background: cx
                                            .theme()
                                            .window_background_appearance(),

                                        window_bounds: Some(WindowBounds::centered(
                                            scaled_bounds,
                                            cx,
                                        )),
                                        ..Default::default()
                                    },
                                    |window, cx| {
                                        let connection_dialog = cx.new(|cx| {
                                            ConnectionDialog::new(window, cx, session_manager, None)
                                        });
                                        // settings_window.update(cx, |settings_window, cx| {
                                        //     callback(settings_window, window, cx);
                                        // });

                                        // connection_dialog
                                        cx.new(|cx| Root::new(connection_dialog, window, cx))
                                    },
                                )
                                .expect("Failed to open window");
                            });
                        }
                    })))
                    .child(
                        Button::new("3")
                            .icon(IconName::Ellipsis)
                            .on_click(|_, _, _| {
                                info!("点击事件3");
                            }),
                    ),
            )
            // ============ Search box ============
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .border_b_1()
                    .border_color(t.colors().border)
                    .child(
                        div()
                            .text_color(t.colors().icon_accent)
                            .text_size(px(12.0))
                            .child(IconName::Search),
                    )
                    .child(
                        div()
                            .flex_1()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .bg(t.colors().background)
                            .border_1()
                            .border_color(t.colors().border)
                            .text_size(px(12.5))
                            .text_color(t.colors().text)
                            .child(if self.search_query.is_empty() {
                                div()
                                    .text_color(t.colors().icon_accent)
                                    .child("Search sessions…")
                                    .into_any_element()
                            } else {
                                div().child(self.search_query.clone()).into_any_element()
                            }),
                    ),
            )
            // ============ Group list ============
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    // .overflow_y_scroll()
                    .py(px(8.0))
                    .children(groups.into_iter().map(|(group_name, sessions)| {
                        let collapsed = self.collapsed.contains(&group_name);
                        render_group(group_name, sessions, collapsed, active_id.clone(), &cx)
                    })),
            )
    }
}

fn render_group(
    name: String,
    sessions: Vec<Session>,
    collapsed: bool,
    active_id: Option<String>,

    cx: &&mut Context<Sidebar>,
) -> impl IntoElement {
    let t = cx.theme();
    let chevron_rot = if collapsed {
        IconName::ChevronRight
    } else {
        IconName::ChevronDown
    };

    div()
        .flex()
        .flex_col()
        .mb(px(12.0))
        // group header
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .px(px(12.0))
                .py(px(4.0))
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.colors().icon_accent)
                .cursor_pointer()
                .child(chevron_rot)
                .child(
                    div()
                        .flex_1()
                        .child(format!("{} · {}", name, sessions.len())),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        if this.collapsed.contains(&name) {
                            this.collapsed.remove(&name);
                        } else {
                            this.collapsed.insert(name.clone());
                        }
                        cx.notify();
                    }),
                ),
        )
        .children(if !collapsed {
            sessions
                .into_iter()
                .map(|s| {
                    let is_active = active_id.as_deref() == Some(&s.id);
                    render_session_item(s, is_active, cx)
                })
                .collect()
        } else {
            Vec::new()
        })
}

fn render_session_item(
    session: Session,
    is_active: bool,
    cx: &&mut Context<Sidebar>,
) -> impl IntoElement {
    let t = cx.theme();
    let id = session.id.clone();
    let status_color = match session.status {
        SessionStatus::Connected => t.colors().icon_accent,
        SessionStatus::Idle => t.status().conflict,
        SessionStatus::Disconnected => t.colors().icon_accent,
        SessionStatus::Error => t.status().error,
    };

    let bg = if is_active {
        t.colors().icon_accent
    } else {
        Hsla::transparent_black()
    };
    let border_left_color = if is_active {
        t.colors().icon_accent
    } else {
        Hsla::transparent_black()
    };

    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .py(px(6.0))
        .bg(bg)
        .border_l_2()
        .border_color(border_left_color)
        .cursor_pointer()
        .hover(|s| s.bg(t.colors().background))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                if e.click_count == 2 {
                    // this.session_manager.update(cx, |this, cx| {
                    //     this.open_session(session.id.clone());
                    // });
                    // this.state
                    //     .update(cx, |state, cx| state.set_active_session(&id));

                    window.dispatch_action(
                        Box::new(OpenTerminalAction {
                            session_id: session.id.clone(),
                        }),
                        cx,
                    );
                    cx.notify();
                    info!("发送消息");
                }
            }),
        )
        .context_menu({
            let id = id.clone();
            move |menu, window, cx| {
                menu.menu(
                    "编辑",
                    Box::new(EditAction {
                        session_id: id.clone(),
                    }),
                )
                .separator()
                .menu(
                    "复制",
                    Box::new(CopyAction {
                        session_id: id.clone(),
                    }),
                )
                .separator()
                .menu(
                    "删除",
                    Box::new(DeleteAction {
                        session_id: id.clone(),
                    }),
                )
            }
        })
        // status dot
        .child(div().size(px(7.0)).rounded_full().bg(status_color))
        // name + meta
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.colors().text)
                        .child(session.name),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(t.colors().icon_accent)
                        .child(format!(
                            "{}:{} · {}",
                            session.hostname, session.port, session.username
                        )),
                ),
        )
}
