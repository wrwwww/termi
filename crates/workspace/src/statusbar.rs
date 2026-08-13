//! `StatusBar` — bottom strip showing connection state, encoding, theme.
//!
//! Mirrors the preview's footer pattern: connected chip · user@host ·
//! encoding/line endings/grid · theme + agent forwarding (right).

use crate::{session_manager::SessionStatus, state::AppState};

use gpui::*;
use settings::Settings;
use settings_content::theme::ThemeAppearanceMode;
use termi_action::theme::ToggleMode;
use theme::{ActiveTheme, Theme};
use theme_settings::ThemeSettings;

pub struct StatusBar {
    state: Entity<AppState>,
}

impl StatusBar {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
    fn toggle_theme_mode(&mut self, _: &ToggleMode, _window: &mut Window, cx: &mut Context<Self>) {
        let current_mode = ThemeSettings::get_global(cx).theme.mode();
        let next_mode = match current_mode {
            Some(ThemeAppearanceMode::Light) => ThemeAppearanceMode::Dark,
            Some(ThemeAppearanceMode::Dark) => ThemeAppearanceMode::Light,
            Some(ThemeAppearanceMode::System) | None => match cx.theme().appearance() {
                theme::Appearance::Light => ThemeAppearanceMode::Dark,
                theme::Appearance::Dark => ThemeAppearanceMode::Light,
            },
        };

        // let fs = self.project().read(cx).fs().clone();
        // settings::update_settings_file(fs, cx, move |settings, _cx| {
        // theme_settings::set_mode(settings, next_mode);
        // });
    }
}

impl Render for StatusBar {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme();
        let state = self.state.read(cx);
        let active = state
            .active_session_id
            .as_ref()
            .and_then(|id| state.sessions.iter().find(|s| &s.id == id));

        let connected = active
            .map(|s| s.status == SessionStatus::Connected)
            .unwrap_or(false);
        let latency_text = active
            .and_then(|s| s.latencies_ms.last().copied())
            .map(|ms| format!("{} ms", ms))
            .unwrap_or_default();
        let user_host = active
            .map(|s| format!("{}@{}", s.username, s.name))
            .unwrap_or_else(|| "No active session".into());

        div()
            .on_action(cx.listener(Self::toggle_theme_mode))
            .id("lumen-statusbar")
            .flex()
            .flex_row()
            .items_center()
            .h(px(28.))
            .px(px(12.0))
            .bg(t.colors().background)
            .border_t_1()
            .border_color(t.colors().border)
            .text_size(px(11.5))
            .text_color(t.colors().text_muted)
            // group 1: connection chip
            .child(group(
                &t,
                |s| {
                    let dot = div().size(px(6.0)).rounded_full().bg(if connected {
                        t.colors().icon_accent
                    } else {
                        t.colors().icon_accent
                    });
                    s.child(dot)
                        .child(
                            div()
                                .px(px(8.0))
                                .py(px(2.0))
                                .rounded_full()
                                .border_1()
                                .border_color(t.colors().border)
                                .bg(t.colors().background)
                                .text_color(if connected {
                                    t.colors().icon_accent
                                } else {
                                    t.colors().text_muted
                                })
                                .child(if connected {
                                    "Connected"
                                } else {
                                    "Disconnected"
                                }),
                        )
                        .child(div().text_color(t.colors().text_muted).child(latency_text))
                },
                true,
            ))
            // group 2: user@host
            .child(group(
                &t,
                |s| s.child(div().text_color(t.colors().text_muted).child(user_host)),
                true,
            ))
            // group 3: encoding + dimensions
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .h_full()
                    .border_r_1()
                    .border_color(t.colors().border)
                    .child(pill(&t, "UTF-8"))
                    .child(pill(&t, "CR/LF"))
                    .child(pill(&t, "132×40")),
            )
            // spacer
            .child(div().flex_1())
            // right group: theme + agent
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .h_full()
                    .child(render_theme_toggle(&t))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .text_color(t.colors().text_muted)
                            .child(div().child("⌑"))
                            .child("Agent forwarding"),
                    ),
            )
    }
}

fn group<F>(t: &Theme, f: F, with_border: bool) -> impl IntoElement
where
    F: FnOnce(Div) -> Div,
{
    let mut d = div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(12.0))
        .h_full();
    if with_border {
        d = d.border_r_1().border_color(t.colors().border);
    }
    f(d)
}

fn pill(t: &Theme, label: &str) -> impl IntoElement {
    div()
        .text_color(t.colors().text_muted)
        .text_size(px(11.5))
        .child(text!(label))
}

fn render_theme_toggle(t: &Theme) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .p(px(2.0))
        .rounded_full()
        .bg(t.colors().background)
        .border_1()
        .border_color(t.colors().border)
        .child(theme_pill_btn(t, "Dark", true))
        .child(theme_pill_btn(t, "Light", false))
        .child(theme_pill_btn(t, "System", false))
}

fn theme_pill_btn(t: &Theme, label: &str, active: bool) -> impl IntoElement {
    let (bg, color) = if active {
        (t.colors().background, t.colors().text)
    } else {
        (Hsla::transparent_black(), t.colors().text_muted)
    };
    div()
        .px(px(10.0))
        .py(px(4.0))
        .rounded_full()
        .bg(bg)
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color)
        .child(text!(label))
}
