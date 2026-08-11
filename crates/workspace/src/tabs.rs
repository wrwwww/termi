//! `TabsBar` — horizontal strip of open terminal tabs above the pane area.

use crate::{state::AppState, theme::active};
use gpui::*;
use theme::Theme;

pub struct TabsBar {
    state: Entity<AppState>,
}

impl TabsBar {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for TabsBar {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = active(cx).clone();
        let state = self.state.read(cx);
        let active_id = state.active_session_id.clone();
        let connected_ids: Vec<String> = state
            .sessions
            .iter()
            .filter(|s| s.status == crate::state::SessionStatus::Connected)
            .map(|s| s.id.clone())
            .collect();

        let mut row = div()
            .id("lumen-tabs")
            .flex()
            .flex_row()
            .items_center()
            .h(px(t.layout.tabs_height))
            .bg(t.surfaces.surface)
            .border_b_1()
            .border_color(t.border.border)
            .overflow_x_scroll();

        // One tab per connected session, in order.
        // row = row.children(connected_ids.iter().map(|id| {
        //     let session = state.sessions.iter().find(|s| &s.id == id);
        //     if let Some(session) = session {
        //         let is_active = Some(id.as_str()) == active_id.as_deref();
        //         render_tab(session, is_active, &t, cx).into_any_element()
        //     } else {
        //         div().into_any_element()
        //     }
        // }));

        row = row.child(render_new_tab_btn(&t));

        row
    }
}

fn render_tab(
    session: &crate::state::Session,
    is_active: bool,
    t: &Theme,
    _cx: &mut Context<TabsBar>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .h_full()
        .border_r_1()
        .border_color(t.border.border)
        .bg(if is_active {
            t.surfaces.bg
        } else {
            Hsla::transparent_black()
        })
        .relative()
        .child(div().size(px(6.0)).rounded_full().bg(t.semantic.green))
        .child(
            div()
                .text_size(px(12.5))
                .text_color(if is_active {
                    t.text.text
                } else {
                    t.text.text_muted
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
                .text_color(t.text.text_subtle)
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
        .border_color(t.border.border)
        .text_color(t.text.text_muted)
        .child("+")
}
