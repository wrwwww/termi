//! `Sidebar` — left-pane session list, grouped by group name.
//!
//! Each group is collapsible. Selecting an entry sets the active session
//! and (in the real app) brings the tab forward.

use crate::state::{AppState, SessionStatus};
use gpui::*;
use theme::{ActiveTheme, Theme};

pub struct Sidebar {
    state: Entity<AppState>,
    // Persisted client-side UI state (not serialised).
    collapsed: std::collections::HashSet<String>,
    search_query: String,
}

impl Sidebar {
    pub fn new(state: Entity<AppState>) -> Self {
        let mut collapsed = std::collections::HashSet::new();
        collapsed.insert("Personal".into()); // match `preview.html` collapsed group
        Self {
            state,
            collapsed,
            search_query: String::new(),
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme();
        let grouped = self.state.read(cx).grouped_sessions();
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
                    ), // .child(icon_button(t, "↻")) // refresh
                       // .child(icon_button(t, "+")) // new
                       // .child(icon_button(t, "⋯")), // more
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
                            .child("⌕"),
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
                    .py(px(8.0)), // .children(groups.into_iter().map(|(group_name, sessions)| {
                                  //     let collapsed = self.collapsed.contains(&group_name);
                                  //     // render_group(group_name, sessions, collapsed, active_id.clone(), &t, cx)
                                  // })),
            )
    }
}

fn render_group(
    name: String,
    sessions: Vec<crate::state::Session>,
    collapsed: bool,
    active_id: Option<String>,
    t: &Theme,
    cx: &mut Context<Sidebar>,
) -> impl IntoElement {
    let chevron_rot = if collapsed { "›" } else { "⌄" };

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
                .child(div().child(chevron_rot))
                .child(
                    div()
                        .flex_1()
                        .child(format!("{} · {}", name, sessions.len())),
                ), // .on_click(cx.listener(move |this, _, _| {
                   //     if this.collapsed.contains(&name) {
                   //         this.collapsed.remove(&name);
                   //     } else {
                   //         this.collapsed.insert(name.clone());
                   //     }
                   // })),
        )
    // entries (only when expanded)
    // .children(if !collapsed {
    //     sessions
    //         .into_iter()
    //         .map(|s| {
    //             let is_active = active_id.as_deref() == Some(&s.id);
    //             render_session_item(s, is_active, t, cx)
    //         })
    //         .collect()
    // } else {
    //     Vec::new()
    // })
}

fn render_session_item(
    session: crate::state::Session,
    is_active: bool,
    t: &Theme,
    cx: &mut Context<Sidebar>,
) -> impl IntoElement {
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
        // .on_click(cx.listener(move |this, _, _| {
        //     this.state.update(this.cx(), |s| s.set_active_session(&id));
        // }))
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

fn icon_button(t: &Theme, glyph: &'static str) -> impl IntoElement {
    div()
        .size(px(24.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .text_color(t.colors().icon_accent)
        .text_size(px(13.0))
        .cursor_pointer()
        .hover(|s| s.bg(t.colors().background).text_color(t.colors().text))
        .child(glyph)
}
