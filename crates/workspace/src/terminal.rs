//! `TerminalPane` — the actual terminal viewport.
//!
//! In a real implementation this would be backed by `portable-pty` +
//! `vte::Parser` to translate ANSI escape sequences into styled glyphs.
//! This reference paints a representative static frame.

use crate::{state::AppState, theme::active};
use gpui::*;

pub struct TerminalPane {
    state: Entity<AppState>,
}

impl TerminalPane {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for TerminalPane {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = active(cx).clone();
        let active = self.state.read(cx).active_session_id.clone();

        div()
            .id("lumen-terminal")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(t.terminal.bg)
            .child(
                div()
                    .id("terminal-viewport")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .p(px(16.0))
                    .px(px(20.0))
                    .font_family(t.font.mono)
                    .text_size(px(13.0))
                    .line_height(px(20.15)) // matches 1.55 with 13px
                    .text_color(t.terminal.text)
                    .overflow_y_scroll()
                    .children(sample_lines(&t, active.as_deref())),
            )
    }
}

/// Sample lines that approximate what a real shell session looks like.
fn sample_lines(t: &Theme, _session: Option<&str>) -> Vec<AnyElement> {
    use crate::theme::TerminalPalette;
    let pal = &t.terminal;

    let prompt = |user: &str, path: &str, cmd: &str| -> AnyElement {
        div()
            .flex()
            .flex_row()
            .children([
                div().text_color(pal.prompt).child(text!(user)),
                div().text_color(t.text.text_subtle).child(text!(":")),
                div().text_color(pal.path).child(text!(path)),
                div().text_color(t.text.text_subtle).child(text!("$ ")),
                div().text_color(pal.text).child(text!(cmd)),
            ])
            .into_any_element()
    };

    let plain = |s: &str, color: Hsla| -> AnyElement {
        div().text_color(color).child(text!(s)).into_any_element()
    };

    let blank = || -> AnyElement { div().child(" ").into_any_element() };

    vec![
        plain(
            "Last login: Tue Aug  6 09:08:14 2026 from 10.0.99.42",
            pal.gray,
        ),
        prompt("deploy@prod-web-01", "~", "ls -la"),
        plain("total 36", pal.text),
        plain("drwx------  1 deploy deploy  240 Aug  6 09:08 .", pal.text),
        plain(
            "drwxr-xr-x  1 root    root     72 Aug  6 08:55 ..",
            pal.text,
        ),
        plain(
            "-rw-r--r--  1 deploy deploy  220 Apr 18  2024 .bash_logout",
            pal.text,
        ),
        plain(
            "-rw-r--r--  1 deploy deploy 3.7K Apr 18  2024 .bashrc",
            pal.text,
        ),
        plain(
            "drwxr-xr-x  3 deploy deploy   18 Aug  6 09:00 .config",
            pal.text,
        ),
        plain(
            "drwxr-xr-x  5 deploy deploy   72 Aug  6 08:50 .npm",
            pal.text,
        ),
        blank(),
        prompt("deploy@prod-web-01", "~", "uptime"),
        div()
            .flex()
            .flex_row()
            .children([
                div()
                    .text_color(pal.text)
                    .child(" 09:12:30 up 42 days, 14:08,  1 user,  load average: "),
                div().text_color(pal.amber).child("0.05"),
                div().text_color(pal.text).child(", "),
                div().text_color(pal.amber).child("0.12"),
                div().text_color(pal.text).child(", "),
                div().text_color(pal.amber).child("0.08"),
            ])
            .into_any_element(),
        blank(),
        prompt("deploy@prod-web-01", "~", "systemctl status nginx"),
        div()
            .flex()
            .flex_row()
            .children([
                div().text_color(pal.prompt).child("●"),
                div().ml(px(4.0)).text_color(pal.text).child(
                    " nginx.service - A high performance web server and a reverse proxy server",
                ),
            ])
            .into_any_element(),
        plain(
            "   Loaded: loaded (/lib/systemd/system/nginx.service; enabled; vendor preset: enabled)",
            pal.text,
        ),
        div()
            .flex()
            .flex_row()
            .children([
                div().text_color(pal.text).child("   Active: "),
                div().text_color(pal.prompt).child("active (running)"),
                div()
                    .text_color(pal.text)
                    .child(" since Mon 2026-06-15 09:24:32 UTC; 1 month 11 days ago"),
            ])
            .into_any_element(),
        plain("     Docs: man:nginx(8)", pal.text),
        plain(
            "  Process: 1234 ExecStartPre=/usr/sbin/nginx -t -q -g daemon on; master_process on; (code=exited, status=0/SUCCESS)",
            pal.text,
        ),
        plain(
            "  Process: 1235 ExecStart=/usr/sbin/nginx -g daemon on; master_process on; (code=exited, status=0/SUCCESS)",
            pal.text,
        ),
        plain(" Main PID: 1235 (nginx)", pal.text),
        plain("    Tasks: 5 (limit: 4915)", pal.text),
        plain("   Memory: 18.4M", pal.text),
        plain("      CPU: 1.215s", pal.text),
        plain("   CGroup: /system.slice/nginx.service", pal.text),
        plain(
            "           ├─1235 \"nginx: master process /usr/sbin/nginx -g daemon on; master_process on;\"",
            pal.purple,
        ),
        plain("           ├─1236 \"nginx: worker process\"", pal.purple),
        plain("           ├─1237 \"nginx: worker process\"", pal.purple),
        plain("           └─1238 \"nginx: worker process\"", pal.purple),
        blank(),
        prompt("deploy@prod-web-01", "~", ""),
        // blinking cursor glyph
        div()
            .flex()
            .flex_row()
            .items_center()
            .child(
                div()
                    .id("terminal-cursor")
                    .w(px(7.0))
                    .h(px(14.0))
                    .bg(pal.text)
                    .ml(px(2.0)),
            )
            .into_any_element(),
    ]
}

// Import Hsla trait so `.text_color()` accepts our token types.
use gpui::Hsla;
use theme::Theme;
