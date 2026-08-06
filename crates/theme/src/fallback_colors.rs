use gpui::{Hsla, WindowBackgroundAppearance, hsla};

use crate::{
    Appearance, DEFAULT_DARK_THEME, Theme, ThemeFamily,
    colors::{StatusColors, ThemeColors, ThemeStyles},
    default_colors::{self, SystemColors, default_color_scales},
};

/// The default theme family for Zed.
///
/// This is used to construct the default theme fallback values, as well as to
/// have a theme available at compile time for tests.
pub fn zed_default_themes() -> ThemeFamily {
    ThemeFamily {
        id: "zed-default".to_string(),
        name: "Zed Default".into(),
        author: "".into(),
        themes: vec![zed_default_dark()],
        scales: default_color_scales(),
    }
}

pub(crate) fn zed_default_dark() -> Theme {
    let bg = hsla(215. / 360., 12. / 100., 15. / 100., 1.);
    let editor = hsla(220. / 360., 12. / 100., 18. / 100., 1.);
    let elevated_surface = hsla(225. / 360., 12. / 100., 17. / 100., 1.);
    let hover = hsla(225.0 / 360., 11.8 / 100., 26.7 / 100., 1.0);

    let blue = hsla(207.8 / 360., 81. / 100., 66. / 100., 1.0);
    let gray = hsla(218.8 / 360., 10. / 100., 40. / 100., 1.0);
    let green = hsla(95. / 360., 38. / 100., 62. / 100., 1.0);
    let orange = hsla(29. / 360., 54. / 100., 61. / 100., 1.0);
    let purple = hsla(286. / 360., 51. / 100., 64. / 100., 1.0);
    let red = hsla(355. / 360., 65. / 100., 65. / 100., 1.0);
    let teal = hsla(187. / 360., 47. / 100., 55. / 100., 1.0);
    let yellow = hsla(39. / 360., 67. / 100., 69. / 100., 1.0);

    const ADDED_COLOR: Hsla = Hsla {
        h: 134. / 360.,
        s: 0.55,
        l: 0.40,
        a: 1.0,
    };
    const WORD_ADDED_COLOR: Hsla = Hsla {
        h: 134. / 360.,
        s: 0.55,
        l: 0.40,
        a: 0.35,
    };
    const MODIFIED_COLOR: Hsla = Hsla {
        h: 48. / 360.,
        s: 0.76,
        l: 0.47,
        a: 1.0,
    };
    const REMOVED_COLOR: Hsla = Hsla {
        h: 350. / 360.,
        s: 0.88,
        l: 0.25,
        a: 1.0,
    };
    const WORD_DELETED_COLOR: Hsla = Hsla {
        h: 350. / 360.,
        s: 0.88,
        l: 0.25,
        a: 0.80,
    };

    Theme {
        id: "one_dark".to_string(),
        name: DEFAULT_DARK_THEME.into(),
        appearance: Appearance::Dark,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            colors: ThemeColors {
                border: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                border_variant: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                border_focused: hsla(223. / 360., 78. / 100., 65. / 100., 1.),
                border_selected: hsla(222.6 / 360., 77.5 / 100., 65.1 / 100., 1.0),
                border_transparent: SystemColors::default().transparent,
                border_disabled: hsla(222.0 / 360., 11.6 / 100., 33.7 / 100., 1.0),
                elevated_surface_background: elevated_surface,
                surface_background: bg,
                background: bg,
                element_background: hsla(223.0 / 360., 13. / 100., 21. / 100., 1.0),
                element_hover: hover,
                element_active: hsla(220.0 / 360., 11.8 / 100., 20.0 / 100., 1.0),
                element_selected: hsla(224.0 / 360., 11.3 / 100., 26.1 / 100., 1.0),
                element_disabled: SystemColors::default().transparent,

                drop_target_background: hsla(220.0 / 360., 8.3 / 100., 21.4 / 100., 1.0),
                drop_target_border: hsla(221. / 360., 11. / 100., 86. / 100., 1.0),
                ghost_element_background: SystemColors::default().transparent,
                ghost_element_hover: hover,
                ghost_element_active: hsla(220.0 / 360., 11.8 / 100., 20.0 / 100., 1.0),
                ghost_element_selected: hsla(224.0 / 360., 11.3 / 100., 26.1 / 100., 1.0),
                ghost_element_disabled: SystemColors::default().transparent,
                text: hsla(221. / 360., 11. / 100., 86. / 100., 1.0),
                text_muted: hsla(218.0 / 360., 7. / 100., 46. / 100., 1.0),
                text_placeholder: hsla(220.0 / 360., 6.6 / 100., 44.5 / 100., 1.0),
                text_disabled: hsla(220.0 / 360., 6.6 / 100., 44.5 / 100., 1.0),
                text_accent: hsla(222.6 / 360., 77.5 / 100., 65.1 / 100., 1.0),
                icon: hsla(222.9 / 360., 9.9 / 100., 86.1 / 100., 1.0),
                icon_muted: hsla(220.0 / 360., 12.1 / 100., 66.1 / 100., 1.0),
                icon_disabled: hsla(220.0 / 360., 6.4 / 100., 45.7 / 100., 1.0),
                icon_placeholder: hsla(220.0 / 360., 6.4 / 100., 45.7 / 100., 1.0),
                icon_accent: blue,
                debugger_accent: red,
                status_bar_background: bg,
                title_bar_background: bg,
                title_bar_inactive_background: bg,
                toolbar_background: editor,
                tab_bar_background: bg,
                tab_inactive_background: bg,
                tab_active_background: editor,
                search_match_background: bg,
                search_active_match_background: bg,

                terminal_background: bg,
                // todo("Use one colors for terminal")
                terminal_ansi_background: default_colors::black().dark().step_12(),
                terminal_foreground: default_colors::white().dark().step_12(),
                terminal_bright_foreground: default_colors::white().dark().step_11(),
                terminal_dim_foreground: default_colors::white().dark().step_10(),
                terminal_ansi_black: default_colors::black().dark().step_12(),
                terminal_ansi_red: default_colors::red().dark().step_11(),
                terminal_ansi_green: default_colors::green().dark().step_11(),
                terminal_ansi_yellow: default_colors::yellow().dark().step_11(),
                terminal_ansi_blue: default_colors::blue().dark().step_11(),
                terminal_ansi_magenta: default_colors::violet().dark().step_11(),
                terminal_ansi_cyan: default_colors::cyan().dark().step_11(),
                terminal_ansi_white: default_colors::neutral().dark().step_12(),
                terminal_ansi_bright_black: default_colors::black().dark().step_11(),
                terminal_ansi_bright_red: default_colors::red().dark().step_10(),
                terminal_ansi_bright_green: default_colors::green().dark().step_10(),
                terminal_ansi_bright_yellow: default_colors::yellow().dark().step_10(),
                terminal_ansi_bright_blue: default_colors::blue().dark().step_10(),
                terminal_ansi_bright_magenta: default_colors::violet().dark().step_10(),
                terminal_ansi_bright_cyan: default_colors::cyan().dark().step_10(),
                terminal_ansi_bright_white: default_colors::neutral().dark().step_11(),
                terminal_ansi_dim_black: default_colors::black().dark().step_10(),
                terminal_ansi_dim_red: default_colors::red().dark().step_9(),
                terminal_ansi_dim_green: default_colors::green().dark().step_9(),
                terminal_ansi_dim_yellow: default_colors::yellow().dark().step_9(),
                terminal_ansi_dim_blue: default_colors::blue().dark().step_9(),
                terminal_ansi_dim_magenta: default_colors::violet().dark().step_9(),
                terminal_ansi_dim_cyan: default_colors::cyan().dark().step_9(),
                terminal_ansi_dim_white: default_colors::neutral().dark().step_10(),
                panel_background: bg,
                panel_focused_border: blue,
                panel_indent_guide: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                panel_indent_guide_hover: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                panel_indent_guide_active: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                panel_overlay_background: bg,
                panel_overlay_hover: hover,
                pane_focused_border: blue,
                pane_group_border: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                scrollbar_thumb_background: gpui::transparent_black(),
                scrollbar_thumb_hover_background: hover,
                scrollbar_thumb_active_background: hsla(
                    225.0 / 360.,
                    11.8 / 100.,
                    26.7 / 100.,
                    1.0,
                ),
                scrollbar_thumb_border: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                scrollbar_track_background: gpui::transparent_black(),
                scrollbar_track_border: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                minimap_thumb_background: hsla(225.0 / 360., 11.8 / 100., 26.7 / 100., 0.7),
                minimap_thumb_hover_background: hsla(225.0 / 360., 11.8 / 100., 26.7 / 100., 0.7),
                minimap_thumb_active_background: hsla(225.0 / 360., 11.8 / 100., 26.7 / 100., 0.7),
                minimap_thumb_border: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                link_text_hover: blue,
                vim_normal_background: SystemColors::default().transparent,
                vim_insert_background: SystemColors::default().transparent,
                vim_replace_background: SystemColors::default().transparent,
                vim_visual_background: SystemColors::default().transparent,
                vim_visual_line_background: SystemColors::default().transparent,
                vim_visual_block_background: SystemColors::default().transparent,
                vim_yank_background: hsla(207.8 / 360., 81. / 100., 66. / 100., 0.2),
                vim_helix_jump_label_foreground: red,
                vim_helix_normal_background: SystemColors::default().transparent,
                vim_helix_select_background: SystemColors::default().transparent,
                vim_normal_foreground: SystemColors::default().transparent,
                vim_insert_foreground: SystemColors::default().transparent,
                vim_replace_foreground: SystemColors::default().transparent,
                vim_visual_foreground: SystemColors::default().transparent,
                vim_visual_line_foreground: SystemColors::default().transparent,
                vim_visual_block_foreground: SystemColors::default().transparent,
                vim_helix_normal_foreground: SystemColors::default().transparent,
                vim_helix_select_foreground: SystemColors::default().transparent,
                element_selection_background: SystemColors::default().transparent,
            },
            status: StatusColors {
                conflict: yellow,
                conflict_background: yellow,
                conflict_border: yellow,
                created: green,
                created_background: green,
                created_border: green,
                deleted: red,
                deleted_background: red,
                deleted_border: red,
                error: red,
                error_background: red,
                error_border: red,
                hidden: gray,
                hidden_background: gray,
                hidden_border: gray,
                hint: blue,
                hint_background: blue,
                hint_border: blue,
                ignored: gray,
                ignored_background: gray,
                ignored_border: gray,
                info: blue,
                info_background: blue,
                info_border: blue,
                modified: yellow,
                modified_background: yellow,
                modified_border: yellow,
                predictive: gray,
                predictive_background: gray,
                predictive_border: gray,
                renamed: blue,
                renamed_background: blue,
                renamed_border: blue,
                success: green,
                success_background: green,
                success_border: green,
                unreachable: gray,
                unreachable_background: gray,
                unreachable_border: gray,
                warning: yellow,
                warning_background: yellow,
                warning_border: yellow,
            },
        },
    }
}
