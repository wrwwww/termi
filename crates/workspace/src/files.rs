//! `FilesPane` — right-hand SFTP-style file browser. Static demo list.

use crate::state::AppState;
use gpui::*;
use theme::{ActiveTheme, Theme};

pub struct FilesPane {
    state: Entity<AppState>,
}

impl FilesPane {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for FilesPane {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme();

        div()
            .id("lumen-files")
            .flex()
            .flex_col()
            // .h_full()
            .w_full()
            .bg(t.colors().background)
            .border_l_1()
            .border_color(t.colors().border)
            // ===== Toolbar =====
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(44.))
                    .px(px(12.0))
                    .gap(px(8.0))
                    .border_b_1()
                    .border_color(t.colors().border)
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.colors().icon_accent)
                            .child("FILES"),
                    ), // .child(icon_btn(t, "↻"))
                       // .child(icon_btn(t, "+"))
                       // .child(icon_btn(t, "⋯")),
            )
            // ===== Breadcrumb =====
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .px(px(12.0))
                    .py(px(8.0))
                    .font_family("JetBrains Mono")
                    .text_size(px(12.0))
                    .text_color(t.colors().text_muted)
                    .border_b_1()
                    .border_color(t.colors().border)
                    .child(div().child("~"))
                    .child(div().text_color(t.colors().icon_accent).child("/"))
                    .child(div().child("deploy"))
                    .child(div().text_color(t.colors().icon_accent).child("/"))
                    .child(div().child("projects")),
            )
            // ===== File list =====
            .child(
                div()
                    .id("filelist")
                    .flex()
                    .flex_col()
                    .flex_1()
                    // .min_h_0()
                    .overflow_y_scroll()
                    .py(px(4.0))
                    .font_family("JetBrains Mono")
                    .text_size(px(12.5))
                    .children(vec![
                        row(&t, "..", "dr-x", "—", false),
                        row(&t, ".config", "rwx--", "512 B", true),
                        row(&t, "api-server", "rwx--", "128 K", true),
                        row(&t, "web-console", "rwx--", "96 K", true),
                        row(&t, "deploy.sh", "rwx--", "2.4 K", false),
                        row(&t, "Dockerfile", "rwx--", "1.1 K", false),
                        row(&t, "README.md", "rw---", "3.8 K", false),
                        row(&t, "package-lock.json", "r----", "412 K", false),
                    ]),
            )
    }
}

fn row(t: &Theme, name: &str, perms: &str, size: &str, is_dir: bool) -> impl IntoElement {
    let icon = if is_dir { "▸" } else { "·" };
    let icon_color = if is_dir {
        t.status().warning
    } else {
        t.colors().text_muted
    };
    div()
        // .id(("file", name))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .px(px(16.0))
        .py(px(4.0))
        .cursor_pointer()
        .hover(|s| s.bg(t.colors().background))
        // icon
        .child(div().w(px(22.0)).text_color(icon_color).child(icon))
        // name
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .text_color(t.colors().text)
                .child(text!(name)),
        )
        // permissions chip
        .child(
            div()
                .px(px(6.0))
                .py(px(1.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(t.colors().border)
                .bg(t.colors().background)
                .text_color(t.status().warning)
                .text_size(px(11.0))
                .child(text!(perms)),
        )
        // size
        .child(div().text_color(t.colors().icon_accent).child(text!(size)))
}
