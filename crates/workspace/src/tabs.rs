//! Terminal tab strip. Session definitions live in `SessionManager`; this view
//! only renders the runtime tabs recorded in `AppState`.

use crate::{
    EditAction,
    session_manager::SessionManager,
    state::AppState,
    terminal::{ActivateTerminalAction, CloseTerminalAction},
};
use gpui::*;
use protocol::Session;
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme().clone();
        let state = self.state.read(cx);
        let active_id = state.active_tab_id.clone();
        let tabs: Vec<Session> = state
            .open_session_ids
            .iter()
            .filter_map(|id| self.session_manager.read(cx).query(id).cloned())
            .collect();
        div()
            .id("terminal-tabs")
            .flex()
            .flex_row()
            .items_center()
            .h(px(36.))
            .bg(t.colors().background)
            .border_b_1()
            .border_color(t.colors().border)
            .overflow_x_scroll()
            .children(
                tabs.into_iter()
                    .map(|session| render_tab(session, active_id.as_deref(), &t, cx)),
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
                            window.dispatch_action(Box::new(EditAction { session_id: None }), cx);
                        }),
                    ),
            )
    }
}

fn render_tab(
    session: Session,
    active_id: Option<&str>,
    t: &Theme,
    cx: &mut Context<TabsBar>,
) -> AnyElement {
    let session_id = session.id.clone();
    let is_active = active_id == Some(session_id.as_str());
    let activate_id = session_id.clone();
    let close_id = session_id.clone();
    div()
        .id(format!("terminal-tab-{activate_id}"))
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
                .child(session.name),
        )
        .child(
            div()
                .id(format!("terminal-tab-close-{close_id}"))
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
