//! `SettingsView` — two-column preferences page (nav + content).

use crate::{
    state::{AppState, ThemeMode},
    theme::active,
};
use gpui::*;

pub struct SettingsView {
    state: Entity<AppState>,
    section: SettingsSection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsSection {
    Appearance,
    Terminal,
    Ssh,
    Sftp,
    Keyboard,
    Advanced,
}

impl SettingsView {
    pub fn new(state: Entity<AppState>) -> Self {
        Self {
            state,
            section: SettingsSection::Appearance,
        }
    }
}

impl Render for SettingsView {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = active(cx).clone();

        div().flex().flex_row().flex_1().bg(t.surfaces.bg)
        // // ===== Left nav =====
        // .child(nav(&t, self.section, cx))
        // // ===== Content area =====
        // .child(render_section(&t, self.section, &self.state, cx))
    }
}

// // ---------------------------------------------------------------------------
// // Navigation column
// // ---------------------------------------------------------------------------

// fn nav(
//     t: &crate::theme::Theme,
//     active_section: SettingsSection,
//     cx: &mut Context<SettingsView>,
// ) -> impl IntoElement {
//     let make_item = |section: SettingsSection, label: &'static str, icon: &'static str| {
//         let active = section == active_section;
//         div()
//             .flex()
//             .flex_row()
//             .items_center()
//             .gap(px(8.0))
//             .px(px(16.0))
//             .py(px(7.0))
//             .text_size(px(12.5))
//             .text_color(if active {
//                 t.text.text
//             } else {
//                 t.text.text_muted
//             })
//             .font_weight(if active {
//                 FontWeight::MEDIUM
//             } else {
//                 FontWeight::NORMAL
//             })
//             .bg(if active {
//                 t.accent.accent_soft
//             } else {
//                 transparent_hsla()
//             })
//             .border_l_2()
//             .border_color(if active {
//                 t.accent.accent
//             } else {
//                 transparent_hsla()
//             })
//             .cursor_pointer()
//             .hover(|s| s.bg(t.surfaces.surface_2).text_color(t.text.text))
//             .on_click(cx.listener(move |this, _, _| {
//                 this.section = section;
//             }))
//             .child(
//                 div()
//                     .text_color(if active {
//                         t.accent.accent
//                     } else {
//                         t.text.text_muted
//                     })
//                     .child(icon),
//             )
//             .child(label)
//     };

//     div()
//         .flex()
//         .flex_col()
//         .w(px(220.0))
//         .h_full()
//         .bg(t.surfaces.surface)
//         .border_r_1()
//         .border_color(t.border.border)
//         .py(px(12.0))
//         .child(make_item(SettingsSection::Appearance, "Appearance", "◐"))
//         .child(make_item(SettingsSection::Terminal, "Terminal", ">_"))
//         .child(make_item(SettingsSection::Ssh, "SSH", "🔒"))
//         .child(make_item(SettingsSection::Sftp, "SFTP", "📁"))
//         .child(make_item(SettingsSection::Keyboard, "Keyboard", "⌨"))
//         .child(make_item(SettingsSection::Advanced, "Advanced", "⚙"))
// }

// // ---------------------------------------------------------------------------
// // Section content
// // ---------------------------------------------------------------------------

// fn render_section(
//     t: &crate::theme::Theme,
//     section: SettingsSection,
//     state: &Entity<AppState>,
//     _cx: &mut Context<SettingsView>,
// ) -> impl IntoElement {
//     div()
//         .flex()
//         .flex_col()
//         .flex_1()
//         .h_full()
//         .overflow_y_scroll()
//         .p(px(32.0))
//         .px(px(40.0))
//         .child(match section {
//             SettingsSection::Appearance => appearance_panel(t, state),
//             _ => placeholder_panel(t, section),
//         })
// }

// fn placeholder_panel(t: &crate::theme::Theme, section: SettingsSection) -> AnyElement {
//     let title = match section {
//         SettingsSection::Appearance => "Appearance",
//         SettingsSection::Terminal => "Terminal",
//         SettingsSection::Ssh => "SSH",
//         SettingsSection::Sftp => "SFTP",
//         SettingsSection::Keyboard => "Keyboard",
//         SettingsSection::Advanced => "Advanced",
//     };
//     div()
//         .flex()
//         .flex_col()
//         .child(panel_title(t, title))
//         .child(panel_lead(
//             t,
//             "More options for this section live here. Wire them up as you build out the model.",
//         ))
//         .into_any_element()
// }

// fn appearance_panel(t: &crate::theme::Theme, state: &Entity<AppState>) -> AnyElement {
//     div()
//         .flex()
//         .flex_col()
//         // Header
//         .child(panel_title(t, "Appearance"))
//         .child(panel_lead(
//             t,
//             "Customise how Lumen looks and feels. Theme preference syncs across sessions.",
//         ))
//         // Theme group
//         .child(group(
//             t,
//             "Theme",
//             vec![
//                 row_select(
//                     t,
//                     "Colour mode",
//                     "Follow the system appearance, or pin a mode manually.",
//                     state,
//                     &[ThemeMode::Dark, ThemeMode::Light, ThemeMode::System],
//                     |m| match m {
//                         ThemeMode::Dark => "Dark",
//                         ThemeMode::Light => "Light",
//                         ThemeMode::System => "System",
//                     },
//                 ),
//                 row_droplist(
//                     t,
//                     "Accent colour",
//                     "Used for focus rings, links, and active indicators.",
//                     &["Sky", "Indigo", "Emerald", "Amber", "Rose"],
//                     0,
//                 ),
//                 row_droplist(
//                     t,
//                     "Window style",
//                     "Transparent or solid background behind the terminal.",
//                     &["Solid", "Translucent", "Blur"],
//                     0,
//                 ),
//             ],
//         ))
//         // Typography group
//         .child(group(
//             t,
//             "Typography",
//             vec![
//                 row_droplist(
//                     t,
//                     "UI font",
//                     "Used for menus, sidebars, and panels.",
//                     &["Inter", "SF Pro", "system-ui"],
//                     0,
//                 ),
//                 row_droplist(
//                     t,
//                     "Terminal font",
//                     "Used inside the terminal pane.",
//                     &["JetBrains Mono", "Berkeley Mono", "Menlo", "Consolas"],
//                     0,
//                 ),
//                 row_number(
//                     t,
//                     "Terminal font size",
//                     "Applies to all terminals. ⌘+ / ⌘− to override locally.",
//                     13,
//                 ),
//                 row_number(
//                     t,
//                     "Line height",
//                     "Vertical rhythm inside terminal output.",
//                     155,
//                 ), // 1.55 stored *100
//             ],
//         ))
//         // Cursor group
//         .child(group(
//             t,
//             "Cursor",
//             vec![
//                 row_droplist(t, "Style", "", &["Block", "Beam", "Underline"], 0),
//                 row_checkbox(t, "Blinking", "Off automatically in zsh, vim, etc.", true),
//             ],
//         ))
//         .into_any_element()
// }

// // ---------------------------------------------------------------------------
// // Row / control helpers
// // ---------------------------------------------------------------------------

// fn panel_title(t: &crate::theme::Theme, s: &str) -> impl IntoElement {
//     div()
//         .text_size(px(22.0))
//         .font_weight(FontWeight::SEMIBOLD)
//         .text_color(t.text.text)
//         .line_height(px(26.4))
//         .child(s)
// }

// fn panel_lead(t: &crate::theme::Theme, s: &str) -> impl IntoElement {
//     div()
//         .mt(px(8.0))
//         .text_size(px(13.0))
//         .text_color(t.text.text_muted)
//         .child(s)
// }

// fn group(t: &crate::theme::Theme, title: &str, rows: Vec<AnyElement>) -> impl IntoElement {
//     div()
//         .flex()
//         .flex_col()
//         .mt(px(32.0))
//         .child(
//             div()
//                 .text_size(px(11.0))
//                 .font_weight(FontWeight::SEMIBOLD)
//                 .text_color(t.text.text_subtle)
//                 .mb(px(12.0))
//                 .child(title),
//         )
//         .children(rows)
// }

// fn row_layout(
//     t: &crate::theme::Theme,
//     label: &str,
//     desc: &str,
//     control: AnyElement,
// ) -> impl IntoElement {
//     div()
//         .flex()
//         .flex_row()
//         .items_center()
//         .gap(px(16.0))
//         .py(px(12.0))
//         .border_t_1()
//         .border_color(t.border.border)
//         .child(
//             div()
//                 .flex_1()
//                 .child(
//                     div()
//                         .text_size(px(13.0))
//                         .font_weight(FontWeight::MEDIUM)
//                         .text_color(t.text.text)
//                         .child(label),
//                 )
//                 .child(
//                     div()
//                         .mt(px(2.0))
//                         .text_size(px(12.0))
//                         .text_color(t.text.text_subtle)
//                         .child(desc),
//                 ),
//         )
//         .child(div().w(px(200.0)).flex().justify_end().child(control))
// }

// fn row_select<M: Copy>(
//     t: &crate::theme::Theme,
//     label: &str,
//     desc: &str,
//     state: &Entity<AppState>,
//     opts: &[M],
//     label_of: impl Fn(M) -> &'static str,
// ) -> AnyElement {
//     let current = state.read(self_cx()).settings.theme_mode;
//     let active_label = opts
//         .iter()
//         .find(|m| std::mem::discriminant(**m) == std::mem::discriminant(current))
//         .map(|m| label_of(*m))
//         .unwrap_or("");

//     let body = div()
//         .flex()
//         .items_center()
//         .p(px(2.0))
//         .rounded_full()
//         .bg(t.surfaces.surface_2)
//         .border_1()
//         .border_color(t.border.border)
//         .children(opts.iter().map(|m| {
//             let is_active = std::mem::discriminant(*m) == std::mem::discriminant(current);
//             let (bg, color) = if is_active {
//                 (t.surfaces.surface, t.text.text)
//             } else {
//                 (transparent_hsla(), t.text.text_muted)
//             };
//             div()
//                 .id(("opt", label_of(*m)))
//                 .px(px(10.0))
//                 .py(px(4.0))
//                 .rounded_full()
//                 .bg(bg)
//                 .text_size(px(11.0))
//                 .font_weight(FontWeight::MEDIUM)
//                 .text_color(color)
//                 .child(label_of(*m))
//         }));

//     row_layout(t, label, desc, body.into_any_element()).into_any_element()
// }

// fn row_droplist(
//     t: &crate::theme::Theme,
//     label: &str,
//     desc: &str,
//     options: &[&str],
//     idx: usize,
// ) -> AnyElement {
//     let body = div()
//         .w_full()
//         .px(px(10.0))
//         .py(px(7.0))
//         .rounded(px(6.0))
//         .bg(t.surfaces.surface_2)
//         .border_1()
//         .border_color(t.border.border_strong)
//         .text_size(px(12.5))
//         .text_color(t.text.text)
//         .text_align(TextAlign::Right)
//         .child(options.get(idx).copied().unwrap_or(""));
//     row_layout(t, label, desc, body.into_any_element()).into_any_element()
// }

// fn row_number(t: &crate::theme::Theme, label: &str, desc: &str, value: i64) -> AnyElement {
//     let body = div()
//         .w(px(80.0))
//         .px(px(10.0))
//         .py(px(7.0))
//         .rounded(px(6.0))
//         .bg(t.surfaces.surface_2)
//         .border_1()
//         .border_color(t.border.border_strong)
//         .text_size(px(12.5))
//         .text_color(t.text.text)
//         .text_align(TextAlign::Right)
//         .child(format!("{}", value));
//     row_layout(t, label, desc, body.into_any_element()).into_any_element()
// }

// fn row_checkbox(t: &crate::theme::Theme, label: &str, desc: &str, checked: bool) -> AnyElement {
//     let bg = if checked {
//         t.accent.accent_strong
//     } else {
//         t.surfaces.surface_2
//     };
//     let border = if checked {
//         t.accent.accent_strong
//     } else {
//         t.border.border_strong
//     };
//     let body = div()
//         .flex()
//         .flex_row()
//         .items_center()
//         .gap(px(8.0))
//         .text_size(px(12.5))
//         .text_color(t.text.text)
//         .child(
//             div()
//                 .size(px(14.0))
//                 .rounded(px(3.0))
//                 .bg(bg)
//                 .border_1()
//                 .border_color(border)
//                 .flex()
//                 .items_center()
//                 .justify_center()
//                 .child(if checked {
//                     div()
//                         .w(px(5.0))
//                         .h(px(8.0))
//                         .border_2()
//                         .border_color(Hsla {
//                             h: 0.,
//                             s: 0.,
//                             l: 0.97,
//                             a: 1.,
//                         })
//                         .into_any_element()
//                 } else {
//                     div().into_any_element()
//                 }),
//         )
//         .child(label);
//     row_layout(t, label, desc, body.into_any_element()).into_any_element()
// }

// // ---------- tiny colour helper ----------
// fn transparent_hsla() -> Hsla {
//     Hsla {
//         h: 0.,
//         s: 0.,
//         l: 0.,
//         a: 0.,
//     }
// }

// // `self_cx()` exists so helper functions can reach back into the AppState's
// // model via the ViewContext. In real code we would thread the context through
// // each row helper; this is a reference implementation that makes the call site
// // readable.
// fn self_cx() -> &'static mut AppContext {
//     unreachable!("`self_cx()` is a placeholder — pass `cx` explicitly in real code")
// }
