//! `TabsBar` — horizontal strip of open terminal tabs above the pane area.

use crate::{
    session_manager::{self, Session, SessionManager, SessionStatus},
    state::AppState,
};
use gpui::*;
use theme::{ActiveTheme, Theme};

pub struct TabsBar {
    state: Entity<AppState>,
    session_manager: Entity<SessionManager>,
}

impl TabsBar {
    pub fn new(state: Entity<AppState>, session_manager: Entity<SessionManager>) -> Self {
        Self {
            state,
            session_manager,
        }
    }
}

impl Render for TabsBar {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme();
        let state = self.state.read(cx);
        let active_id = state.active_session_id.clone();
        let connected_ids: Vec<String> = state
            .sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Connected)
            .map(|s| s.id.clone())
            .collect();
        let mut connected_ids = self.session_manager.read(cx).open_sessions.clone();

        let mut row = div()
            .id("lumen-tabs")
            .flex()
            .flex_row()
            .items_center()
            .h(px(36.))
            .bg(t.colors().background)
            .border_b_1()
            .border_color(t.colors().border)
            .overflow_x_scroll();
        let list = self.session_manager.read(cx).connectioned();
        // One tab per connected session, in order.
        row = row.children(list.iter().map(|session| {
            // let session = list.iter().find(|s| *s.id == *id);
            // if let Some(session) = session {
            // let is_active = Some(id.as_str()) == active_id.as_deref();
            render_tab(session, true, &t, &cx).into_any_element()
            // } else {
            // div().into_any_element()
            // }
        }));

        row = row.child(render_new_tab_btn(&t));

        row
    }
}

fn render_tab(
    session: &Session,
    is_active: bool,
    t: &Theme,
    _cx: &&mut Context<TabsBar>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .h_full()
        .border_r_1()
        .border_color(t.colors().border)
        .bg(if is_active {
            t.colors().background
        } else {
            Hsla::transparent_black()
        })
        .relative()
        .child(div().size(px(6.0)).rounded_full().bg(t.status().conflict))
        .child(
            div()
                .text_size(px(12.5))
                .text_color(if is_active {
                    t.colors().text
                } else {
                    t.colors().text_muted
                })
                .child(session.name.clone()),
        )
        .child(
            div()
                .size(px(18.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .text_color(t.colors().text_placeholder)
                .child("×"),
        )
}

fn render_new_tab_btn(t: &Theme) -> impl IntoElement {
    div()
        .id("tab-new")
        .px(px(12.0))
        .h_full()
        .flex()
        .items_center()
        .border_r_1()
        .border_color(t.colors().border)
        .text_color(t.colors().text_muted)
        .child("+")
}
