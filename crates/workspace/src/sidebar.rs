//! `Sidebar` — left-pane session list, grouped by group name.
//!
//! Each group is collapsible. Selecting an entry sets the active session
//! and (in the real app) brings the tab forward.

use crate::{
    OpenNewSession, OpenTerminal,
    connection_dialog::ConnectionDialog,
    session_manager::{self, Session, SessionManager, SessionStatus},
    state::{ActiveView::Workspace, AppState},
};
use gpui::*;
use gpui_component::{IconName, WindowExt, button::Button};
use log::info;
use theme::{ActiveTheme, Theme};

pub struct Sidebar {
    state: Entity<AppState>,
    // Persisted client-side UI state (not serialised).
    collapsed: std::collections::HashSet<String>,
    search_query: String,
    connection_dialog: Entity<ConnectionDialog>,
    session_manager: Entity<SessionManager>,
}

impl Sidebar {
    pub fn new(
        state: Entity<AppState>,
        connection_dialog: Entity<ConnectionDialog>,
        session_manager: Entity<SessionManager>,
    ) -> Self {
        let mut collapsed = std::collections::HashSet::new();
        collapsed.insert("Personal".into()); // match `preview.html` collapsed group
        Self {
            state,
            collapsed,
            search_query: String::new(),
            connection_dialog,
            session_manager,
        }
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
                            || s.host.to_lowercase().contains(&query)
                    });
                }
                (group_name, sessions)
            })
            .filter(|(_, s)| query.is_empty() || !s.is_empty())
            .collect();

        div()
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
                            let d = this.connection_dialog.clone();
                            window.open_dialog(cx, move |dialog, window, cx| {
                                dialog
                                    // .h(px(450.))
                                    .w(px(700.))
                                    .title("新建会话")
                                    .close_button(true)
                                    .overlay(true)
                                    .child(div().flex().flex_1().min_h_0().child(d.clone()))
                            })
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
                this.session_manager.update(cx, |this, cx| {
                    this.open_session(session.id.clone());
                });
                this.state
                    .update(cx, |state, cx| state.set_active_session(&id));
                if e.click_count == 2 {
                    info!("这里");
                    window.dispatch_action(OpenTerminal.boxed_clone(), cx);
                }
                cx.notify();
            }),
        )
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
                            session.host, session.port, session.username
                        )),
                ),
        )
}
