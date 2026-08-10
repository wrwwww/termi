//! `ConnectionDialog` — view shown when `ActiveView::NewConnection` is active.
//!
//! Layout: 2-column with a marketing/feature list on the left and a card with
//! the connection form on the right. The card model is purely visual;
//! the actual field state lives in `AppState` once the user saves.

use crate::{
    state::{ActiveView, AppState},
    theme::active,
};
use gpui::*;

pub struct ConnectionDialog {
    state: Entity<AppState>,
}

impl ConnectionDialog {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for ConnectionDialog {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = active(cx).clone();
        let state = self.state.clone();

        div()
            .id("lumen-newconn")
            .flex()
            .flex_1()
            .bg(t.surfaces.bg)
            .items_center()
            .justify_center()
            .p(px(32.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .w_full()
                    .max_w(px(1080.0))
                    .gap(px(40.0))
                    .items_start()
                    .child(render_hero(&t))
                    .child(render_form_card(&t, state, cx)),
            )
    }
}

fn render_hero(t: &crate::theme::Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(eyebrow(t, "NEW CONNECTION"))
        .child(
            div()
                .mt(px(8.0))
                .text_size(px(32.0))
                .font_weight(FontWeight::BOLD)
                .text_color(t.text.text)
                .line_height(px(36.8))
                .child("A new way to remote in."),
        )
        .child(
            div()
                .mt(px(16.0))
                .text_size(px(14.0))
                .text_color(t.text.text_muted)
                .line_height(px(23.1))
                .child("Save your servers, keys and connection preferences once. Connect with a single click — across SSH, Mosh, or local shell."),
        )
        .child(feature(&t, "First-class key management", "Ed25519, RSA, ECDSA, PuTTY .ppk — auto-detected from ~/.ssh", "🔐", t.accent.accent, t.accent.accent_soft))
        .child(feature(&t, "Port forwarding", "Local, remote, dynamic — all set per-session", "↔", t.semantic.amber, amber_bg()))
        .child(feature(&t, "Snippets & macros", "Save commands, bind hotkeys, replay with one keystroke", "↻", t.semantic.green, green_bg()))
        .child(
            div()
                .mt(px(32.0))
                .text_size(px(11.0))
                .text_color(t.text.text_subtle)
                .child("Tip — paste any ssh:// URL above; we'll parse it for you."),
        )
}

fn eyebrow(t: &crate::theme::Theme, label: &str) -> impl IntoElement {
    div()
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(t.text.text_subtle)
        .child(text!(label))
}

fn feature(
    t: &crate::theme::Theme,
    title: &str,
    desc: &str,
    icon: &str,
    color: Hsla,
    bg: Hsla,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(12.0))
        .mt(px(16.0))
        .child(icon_tile(color, bg, icon))
        .child(
            div()
                .flex()
                .flex_col()
                .child(title_line(&t.text.text, title))
                .child(desc_line(&t.text.text_subtle, desc)),
        )
}

fn icon_tile(color: Hsla, bg: Hsla, icon: &str) -> impl IntoElement {
    div()
        .size(px(32.0))
        .rounded(px(8.0))
        .bg(bg)
        .text_color(color)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(16.0))
        .child(text!(icon))
}

fn title_line(color: &Hsla, text: &str) -> impl IntoElement {
    div()
        .text_size(px(13.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(*color)
        .child(text!(text))
}

fn desc_line(color: &Hsla, text: &str) -> impl IntoElement {
    div()
        .mt(px(2.0))
        .text_size(px(12.0))
        .text_color(*color)
        .line_height(px(18.6))
        .child(text!(text))
}

fn render_form_card(
    t: &crate::theme::Theme,
    state: Entity<AppState>,
    cx: &mut Context<ConnectionDialog>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .bg(t.surfaces.surface)
        .border_1()
        .border_color(t.border.border)
        .rounded(px(12.0))
        .shadow_md()
        .overflow_hidden()
        .child(card_header(t))
        .child(card_body(t))
        .child(card_footer(t, state, cx))
}

fn card_header(t: &crate::theme::Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .px(px(24.0))
        .py(px(20.0))
        .border_b_1()
        .border_color(t.border.border)
        .child(icon_tile(t.accent.accent, t.accent.accent_soft, "⌒"))
        .child(
            div()
                .flex()
                .flex_col()
                .child(title_line(&t.text.text, "Connection details"))
                .child(desc_line(
                    &t.text.text_subtle,
                    "All fields can be edited later.",
                )),
        )
        .child(div().flex_1())
        .child(div().text_color(t.text.text_muted).child("⋯"))
}

fn card_body(t: &crate::theme::Theme) -> impl IntoElement {
    let row_label = |label: &str| -> AnyElement {
        div()
            .w(px(130.0))
            .text_size(px(12.5))
            .text_color(t.text.text_muted)
            .text_align(TextAlign::Right)
            .child(text!(label))
            .into_any_element()
    };

    let mut body = div()
        .flex()
        .flex_col()
        .px(px(24.0))
        .py(px(20.0))
        .gap(px(12.0));

    body = body
        .child(field_row(
            row_label("Name"),
            input(t, "Production Web 01", false),
        ))
        .child(field_row(
            row_label("Group"),
            select_dd(t, &["Production", "Staging", "Personal", "+ New group"]),
        ))
        .child(field_row(row_label("Host"), input(t, "10.0.1.21", true)))
        .child(field_row(row_label("Port"), input(t, "22", true)))
        .child(field_row(row_label("Username"), input(t, "deploy", false)))
        .child(field_row(
            row_label("Protocol"),
            radio_row(t, &["SSH", "Mosh", "Telnet", "Local"], 0),
        ))
        .child(field_row(
            row_label("Authentication"),
            radio_row(t, &["SSH key", "Password", "Agent"], 0),
        ))
        .child(field_row(row_label("Identity file"), file_input(t)));

    body
}

fn card_footer(
    t: &crate::theme::Theme,
    state: Entity<AppState>,
    cx: &mut Context<ConnectionDialog>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(px(24.0))
        .py(px(16.0))
        .border_t_1()
        .border_color(t.border.border)
        .bg(t.surfaces.surface_2)
        .child(
            div()
                .text_size(px(11.0))
                .text_color(t.text.text_subtle)
                .child("⌘S Save · ⌘↩ Connect"),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(button(t, ButtonKind::Ghost, "Cancel"))
                .child(button(t, ButtonKind::Secondary, "Save only"))
                .child(
                    button(t, ButtonKind::Primary, "Save & connect").on_mouse_down_out(
                        cx.listener(move |this, _, _, cx| {
                            this.state.update(cx, |state, cx| {
                                state.set_active_view(ActiveView::Workspace)
                            });
                        }),
                    ),
                )
                .child(div().id("__state_ref").child(state_to_hidden_marker(state))),
        )
}

// Tiny invisible marker that retains the Model handle so the lifetime is obvious
// in this reference code. Remove once you wire the model into a form provider.
fn state_to_hidden_marker(_s: Entity<AppState>) -> AnyElement {
    div().id("state-ref").into_any_element()
}

// ---------------------------------------------------------------------------
// Building blocks
// ---------------------------------------------------------------------------

fn field_row(label: AnyElement, control: AnyElement) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(label)
        .child(div().flex_1().child(control))
}

fn input(t: &crate::theme::Theme, value: &str, mono: bool) -> AnyElement {
    let mut d = div()
        .flex_1()
        .px(px(10.0))
        .py(px(7.0))
        .rounded(px(6.0))
        .bg(t.surfaces.surface_2)
        .border_1()
        .border_color(t.border.border_strong)
        .text_size(px(12.5))
        .text_color(t.text.text);
    if mono {
        d = d.font_family(t.font.mono);
    }
    d.child(text!(value)).into_any_element()
}

fn select_dd(t: &crate::theme::Theme, options: &[&str]) -> AnyElement {
    div()
        .flex_1()
        .px(px(10.0))
        .py(px(7.0))
        .rounded(px(6.0))
        .bg(t.surfaces.surface_2)
        .border_1()
        .border_color(t.border.border_strong)
        .text_size(px(12.5))
        .text_color(t.text.text)
        .child(text!(options.first().copied().unwrap_or("")))
        .into_any_element()
}

fn radio_row(t: &crate::theme::Theme, options: &[&str], active_idx: usize) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .gap(px(16.0))
        .children(options.iter().enumerate().map(|(i, opt)| {
            let active = i == active_idx;
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .text_size(px(12.5))
                .text_color(t.text.text)
                .child(
                    div()
                        .size(px(14.0))
                        .rounded_full()
                        .border_1()
                        .border_color(if active {
                            t.accent.accent
                        } else {
                            t.border.border_strong
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if active {
                            div()
                                .size(px(7.0))
                                .rounded_full()
                                .bg(t.accent.accent)
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }),
                )
                .child(text!(*opt))
        }))
        .into_any_element()
}

fn file_input(t: &crate::theme::Theme) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .flex_1()
        .gap(px(8.0))
        .items_center()
        .child(
            div()
                .flex_1()
                .px(px(10.0))
                .py(px(7.0))
                .rounded(px(6.0))
                .bg(t.surfaces.surface_2)
                .border_1()
                .border_color(t.border.border_strong)
                .font_family(t.font.mono)
                .text_size(px(12.5))
                .text_color(t.text.text)
                .child("~/.ssh/id_ed25519"),
        )
        .child(
            div()
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .bg(t.surfaces.surface_3)
                .border_1()
                .border_color(t.border.border_strong)
                .text_color(t.text.text)
                .text_size(px(12.0))
                .child("Browse…"),
        )
        .into_any_element()
}

#[derive(Clone, Copy)]
enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
}

fn button(t: &crate::theme::Theme, kind: ButtonKind, label: &str) -> Div {
    let (bg, fg, border) = match kind {
        ButtonKind::Primary => (
            t.accent.accent_strong,
            hsla(0., 0., 0.97, 1.),
            transparent_hsla(),
        ),
        ButtonKind::Secondary => (t.surfaces.surface_3, t.text.text, t.border.border_strong),
        ButtonKind::Ghost => (transparent_hsla(), t.text.text_muted, transparent_hsla()),
    };
    div()
        // .id(("btn", label))
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .bg(bg)
        .text_color(fg)
        .border_1()
        .border_color(border)
        .text_size(px(12.5))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .child(text!(label))
}

// ---------- tiny colour helpers ----------

fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla { h, s, l, a }
}

fn transparent_hsla() -> Hsla {
    Hsla {
        h: 0.,
        s: 0.,
        l: 0.,
        a: 0.,
    }
}

fn amber_bg() -> Hsla {
    // rgba(251, 191, 36, 0.12)
    Hsla {
        h: 41.0,
        s: 0.96,
        l: 0.56,
        a: 0.12,
    }
}

fn green_bg() -> Hsla {
    // rgba(134, 239, 172, 0.12)
    Hsla {
        h: 134.0,
        s: 0.77,
        l: 0.73,
        a: 0.12,
    }
}
