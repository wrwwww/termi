#![allow(missing_docs)]

use std::sync::Arc;

use gpui::{App, Hsla, SharedString, WindowBackgroundAppearance};

use refineable::Refineable;
use serde::Deserialize;
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

use crate::{
    ActiveTheme,
    default_colors::{
        SystemColors, amber, blue, cyan, gold, grass, indigo, iris, jade, lime, neutral, orange,
        pink, purple, red, tomato, yellow,
    },
};

#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct ThemeColors {
    /// Border color. Used for most borders, is usually a high contrast color.
    pub border: Hsla,
    /// Border color. Used for deemphasized borders, like a visual divider between two sections
    pub border_variant: Hsla,
    /// Border color. Used for focused elements, like keyboard focused list item.
    pub border_focused: Hsla,
    /// Border color. Used for selected elements, like an active search filter or selected checkbox.
    pub border_selected: Hsla,
    /// Border color. Used for transparent borders. Used for placeholder borders when an element gains a border on state change.
    pub border_transparent: Hsla,
    /// Border color. Used for disabled elements, like a disabled input or button.
    pub border_disabled: Hsla,
    /// Border color. Used for elevated surfaces, like a context menu, popup, or dialog.
    pub elevated_surface_background: Hsla,
    /// Background Color. Used for grounded surfaces like a panel or tab.
    pub surface_background: Hsla,
    /// Background Color. Used for the app background and blank panels or windows.
    pub background: Hsla,
    /// Background Color. Used for the background of an element that should have a different background than the surface it's on.
    ///
    /// Elements might include: Buttons, Inputs, Checkboxes, Radio Buttons...
    ///
    /// For an element that should have the same background as the surface it's on, use `ghost_element_background`.
    pub element_background: Hsla,
    /// Background Color. Used for the hover state of an element that should have a different background than the surface it's on.
    ///
    /// Hover states are triggered by the mouse entering an element, or a finger touching an element on a touch screen.
    pub element_hover: Hsla,
    /// Background Color. Used for the active state of an element that should have a different background than the surface it's on.
    ///
    /// Active states are triggered by the mouse button being pressed down on an element, or the Return button or other activator being pressed.
    pub element_active: Hsla,
    /// Background Color. Used for the selected state of an element that should have a different background than the surface it's on.
    ///
    /// Selected states are triggered by the element being selected (or "activated") by the user.
    ///
    /// This could include a selected checkbox, a toggleable button that is toggled on, etc.
    pub element_selected: Hsla,
    /// Background Color. Used for the background of selections in a UI element.
    pub element_selection_background: Hsla,
    /// Background Color. Used for the disabled state of an element that should have a different background than the surface it's on.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a disabled button or input.
    pub element_disabled: Hsla,
    /// Background Color. Used for the area that shows where a dragged element will be dropped.
    pub drop_target_background: Hsla,
    /// Border Color. Used for the border that shows where a dragged element will be dropped.
    pub drop_target_border: Hsla,
    /// Used for the background of a ghost element that should have the same background as the surface it's on.
    ///
    /// Elements might include: Buttons, Inputs, Checkboxes, Radio Buttons...
    ///
    /// For an element that should have a different background than the surface it's on, use `element_background`.
    pub ghost_element_background: Hsla,
    /// Background Color. Used for the hover state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Hover states are triggered by the mouse entering an element, or a finger touching an element on a touch screen.
    pub ghost_element_hover: Hsla,
    /// Background Color. Used for the active state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Active states are triggered by the mouse button being pressed down on an element, or the Return button or other activator being pressed.
    pub ghost_element_active: Hsla,
    /// Background Color. Used for the selected state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Selected states are triggered by the element being selected (or "activated") by the user.
    ///
    /// This could include a selected checkbox, a toggleable button that is toggled on, etc.
    pub ghost_element_selected: Hsla,
    /// Background Color. Used for the disabled state of a ghost element that should have the same background as the surface it's on.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a disabled button or input.
    pub ghost_element_disabled: Hsla,
    /// Text Color. Default text color used for most text.
    pub text: Hsla,
    /// Text Color. Color of muted or deemphasized text. It is a subdued version of the standard text color.
    pub text_muted: Hsla,
    /// Text Color. Color of the placeholder text typically shown in input fields to guide the user to enter valid data.
    pub text_placeholder: Hsla,
    /// Text Color. Color used for text denoting disabled elements. Typically, the color is faded or grayed out to emphasize the disabled state.
    pub text_disabled: Hsla,
    /// Text Color. Color used for emphasis or highlighting certain text, like an active filter or a matched character in a search.
    pub text_accent: Hsla,
    /// Fill Color. Used for the default fill color of an icon.
    pub icon: Hsla,
    /// Fill Color. Used for the muted or deemphasized fill color of an icon.
    ///
    /// This might be used to show an icon in an inactive pane, or to deemphasize a series of icons to give them less visual weight.
    pub icon_muted: Hsla,
    /// Fill Color. Used for the disabled fill color of an icon.
    ///
    /// Disabled states are shown when a user cannot interact with an element, like a icon button.
    pub icon_disabled: Hsla,
    /// Fill Color. Used for the placeholder fill color of an icon.
    ///
    /// This might be used to show an icon in an input that disappears when the user enters text.
    pub icon_placeholder: Hsla,
    /// Fill Color. Used for the accent fill color of an icon.
    ///
    /// This might be used to show when a toggleable icon button is selected.
    pub icon_accent: Hsla,
    /// Color used to accent some debugger elements
    /// Is used by breakpoints
    pub debugger_accent: Hsla,

    // ===
    // UI Elements
    // ===
    pub status_bar_background: Hsla,
    pub title_bar_background: Hsla,
    pub title_bar_inactive_background: Hsla,
    pub toolbar_background: Hsla,
    pub tab_bar_background: Hsla,
    pub tab_inactive_background: Hsla,
    pub tab_active_background: Hsla,
    pub search_match_background: Hsla,
    pub search_active_match_background: Hsla,
    pub panel_background: Hsla,
    pub panel_focused_border: Hsla,
    pub panel_indent_guide: Hsla,
    pub panel_indent_guide_hover: Hsla,
    pub panel_indent_guide_active: Hsla,

    /// The color of the overlay surface on top of panel.
    pub panel_overlay_background: Hsla,
    /// The color of the overlay surface on top of panel when hovered over.
    pub panel_overlay_hover: Hsla,

    pub pane_focused_border: Hsla,
    pub pane_group_border: Hsla,
    /// The color of the scrollbar thumb.
    pub scrollbar_thumb_background: Hsla,
    /// The color of the scrollbar thumb when hovered over.
    pub scrollbar_thumb_hover_background: Hsla,
    /// The color of the scrollbar thumb whilst being actively dragged.
    pub scrollbar_thumb_active_background: Hsla,
    /// The border color of the scrollbar thumb.
    pub scrollbar_thumb_border: Hsla,
    /// The background color of the scrollbar track.
    pub scrollbar_track_background: Hsla,
    /// The border color of the scrollbar track.
    pub scrollbar_track_border: Hsla,
    /// The color of the minimap thumb.
    pub minimap_thumb_background: Hsla,
    /// The color of the minimap thumb when hovered over.
    pub minimap_thumb_hover_background: Hsla,
    /// The color of the minimap thumb whilst being actively dragged.
    pub minimap_thumb_active_background: Hsla,
    /// The border color of the minimap thumb.
    pub minimap_thumb_border: Hsla,

    /// Background color for Vim Normal mode indicator.
    pub vim_normal_background: Hsla,
    /// Background color for Vim Insert mode indicator.
    pub vim_insert_background: Hsla,
    /// Background color for Vim Replace mode indicator.
    pub vim_replace_background: Hsla,
    /// Background color for Vim Visual mode indicator.
    pub vim_visual_background: Hsla,
    /// Background color for Vim Visual Line mode indicator.
    pub vim_visual_line_background: Hsla,
    /// Background color for Vim Visual Block mode indicator.
    pub vim_visual_block_background: Hsla,
    /// Background color for Vim yank highlight.
    pub vim_yank_background: Hsla,
    /// Foreground color for Helix jump labels.
    pub vim_helix_jump_label_foreground: Hsla,
    /// Background color for Vim Helix Normal mode indicator.
    pub vim_helix_normal_background: Hsla,
    /// Background color for Vim Helix Select mode indicator.
    pub vim_helix_select_background: Hsla,
    /// Foreground color for Vim Normal mode indicator.
    pub vim_normal_foreground: Hsla,
    /// Foreground color for Vim Insert mode indicator.
    pub vim_insert_foreground: Hsla,
    /// Foreground color for Vim Replace mode indicator.
    pub vim_replace_foreground: Hsla,
    /// Foreground color for Vim Visual mode indicator.
    pub vim_visual_foreground: Hsla,
    /// Foreground color for Vim Visual Line mode indicator.
    pub vim_visual_line_foreground: Hsla,
    /// Foreground color for Vim Visual Block mode indicator.
    pub vim_visual_block_foreground: Hsla,
    /// Foreground color for Vim Helix Normal mode indicator.
    pub vim_helix_normal_foreground: Hsla,
    /// Foreground color for Vim Helix Select mode indicator.
    pub vim_helix_select_foreground: Hsla,

    // ===
    // Terminal
    // ===
    /// Terminal layout background color.
    pub terminal_background: Hsla,
    /// Terminal foreground color.
    pub terminal_foreground: Hsla,
    /// Bright terminal foreground color.
    pub terminal_bright_foreground: Hsla,
    /// Dim terminal foreground color.
    pub terminal_dim_foreground: Hsla,
    /// Terminal ANSI background color.
    pub terminal_ansi_background: Hsla,
    /// Black ANSI terminal color.
    pub terminal_ansi_black: Hsla,
    /// Bright black ANSI terminal color.
    pub terminal_ansi_bright_black: Hsla,
    /// Dim black ANSI terminal color.
    pub terminal_ansi_dim_black: Hsla,
    /// Red ANSI terminal color.
    pub terminal_ansi_red: Hsla,
    /// Bright red ANSI terminal color.
    pub terminal_ansi_bright_red: Hsla,
    /// Dim red ANSI terminal color.
    pub terminal_ansi_dim_red: Hsla,
    /// Green ANSI terminal color.
    pub terminal_ansi_green: Hsla,
    /// Bright green ANSI terminal color.
    pub terminal_ansi_bright_green: Hsla,
    /// Dim green ANSI terminal color.
    pub terminal_ansi_dim_green: Hsla,
    /// Yellow ANSI terminal color.
    pub terminal_ansi_yellow: Hsla,
    /// Bright yellow ANSI terminal color.
    pub terminal_ansi_bright_yellow: Hsla,
    /// Dim yellow ANSI terminal color.
    pub terminal_ansi_dim_yellow: Hsla,
    /// Blue ANSI terminal color.
    pub terminal_ansi_blue: Hsla,
    /// Bright blue ANSI terminal color.
    pub terminal_ansi_bright_blue: Hsla,
    /// Dim blue ANSI terminal color.
    pub terminal_ansi_dim_blue: Hsla,
    /// Magenta ANSI terminal color.
    pub terminal_ansi_magenta: Hsla,
    /// Bright magenta ANSI terminal color.
    pub terminal_ansi_bright_magenta: Hsla,
    /// Dim magenta ANSI terminal color.
    pub terminal_ansi_dim_magenta: Hsla,
    /// Cyan ANSI terminal color.
    pub terminal_ansi_cyan: Hsla,
    /// Bright cyan ANSI terminal color.
    pub terminal_ansi_bright_cyan: Hsla,
    /// Dim cyan ANSI terminal color.
    pub terminal_ansi_dim_cyan: Hsla,
    /// White ANSI terminal color.
    pub terminal_ansi_white: Hsla,
    /// Bright white ANSI terminal color.
    pub terminal_ansi_bright_white: Hsla,
    /// Dim white ANSI terminal color.
    pub terminal_ansi_dim_white: Hsla,

    /// Represents a link text hover color.
    pub link_text_hover: Hsla,
}

#[derive(EnumIter, Debug, Clone, Copy, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum ThemeColorField {
    Border,
    BorderVariant,
    BorderFocused,
    BorderSelected,
    BorderTransparent,
    BorderDisabled,
    ElevatedSurfaceBackground,
    SurfaceBackground,
    Background,
    ElementBackground,
    ElementHover,
    ElementActive,
    ElementSelected,
    ElementDisabled,
    DropTargetBackground,
    DropTargetBorder,
    GhostElementBackground,
    GhostElementHover,
    GhostElementActive,
    GhostElementSelected,
    GhostElementDisabled,
    Text,
    TextMuted,
    TextPlaceholder,
    TextDisabled,
    TextAccent,
    Icon,
    IconMuted,
    IconDisabled,
    IconPlaceholder,
    IconAccent,
    StatusBarBackground,
    TitleBarBackground,
    TitleBarInactiveBackground,
    ToolbarBackground,
    TabBarBackground,
    TabInactiveBackground,
    TabActiveBackground,
    SearchMatchBackground,
    SearchActiveMatchBackground,
    PanelBackground,
    PanelFocusedBorder,
    PanelIndentGuide,
    PanelIndentGuideHover,
    PanelIndentGuideActive,
    PanelOverlayBackground,
    PanelOverlayHover,
    PaneFocusedBorder,
    PaneGroupBorder,
    ScrollbarThumbBackground,
    ScrollbarThumbHoverBackground,
    ScrollbarThumbActiveBackground,
    ScrollbarThumbBorder,
    ScrollbarTrackBackground,
    ScrollbarTrackBorder,
    MinimapThumbBackground,
    MinimapThumbHoverBackground,
    MinimapThumbActiveBackground,
    MinimapThumbBorder,

    TerminalBackground,
    TerminalForeground,
    TerminalBrightForeground,
    TerminalDimForeground,
    TerminalAnsiBackground,
    TerminalAnsiBlack,
    TerminalAnsiBrightBlack,
    TerminalAnsiDimBlack,
    TerminalAnsiRed,
    TerminalAnsiBrightRed,
    TerminalAnsiDimRed,
    TerminalAnsiGreen,
    TerminalAnsiBrightGreen,
    TerminalAnsiDimGreen,
    TerminalAnsiYellow,
    TerminalAnsiBrightYellow,
    TerminalAnsiDimYellow,
    TerminalAnsiBlue,
    TerminalAnsiBrightBlue,
    TerminalAnsiDimBlue,
    TerminalAnsiMagenta,
    TerminalAnsiBrightMagenta,
    TerminalAnsiDimMagenta,
    TerminalAnsiCyan,
    TerminalAnsiBrightCyan,
    TerminalAnsiDimCyan,
    TerminalAnsiWhite,
    TerminalAnsiBrightWhite,
    TerminalAnsiDimWhite,
    LinkTextHover,
}

impl ThemeColors {
    pub fn color(&self, field: ThemeColorField) -> Hsla {
        match field {
            ThemeColorField::Border => self.border,
            ThemeColorField::BorderVariant => self.border_variant,
            ThemeColorField::BorderFocused => self.border_focused,
            ThemeColorField::BorderSelected => self.border_selected,
            ThemeColorField::BorderTransparent => self.border_transparent,
            ThemeColorField::BorderDisabled => self.border_disabled,
            ThemeColorField::ElevatedSurfaceBackground => self.elevated_surface_background,
            ThemeColorField::SurfaceBackground => self.surface_background,
            ThemeColorField::Background => self.background,
            ThemeColorField::ElementBackground => self.element_background,
            ThemeColorField::ElementHover => self.element_hover,
            ThemeColorField::ElementActive => self.element_active,
            ThemeColorField::ElementSelected => self.element_selected,
            ThemeColorField::ElementDisabled => self.element_disabled,
            ThemeColorField::DropTargetBackground => self.drop_target_background,
            ThemeColorField::DropTargetBorder => self.drop_target_border,
            ThemeColorField::GhostElementBackground => self.ghost_element_background,
            ThemeColorField::GhostElementHover => self.ghost_element_hover,
            ThemeColorField::GhostElementActive => self.ghost_element_active,
            ThemeColorField::GhostElementSelected => self.ghost_element_selected,
            ThemeColorField::GhostElementDisabled => self.ghost_element_disabled,
            ThemeColorField::Text => self.text,
            ThemeColorField::TextMuted => self.text_muted,
            ThemeColorField::TextPlaceholder => self.text_placeholder,
            ThemeColorField::TextDisabled => self.text_disabled,
            ThemeColorField::TextAccent => self.text_accent,
            ThemeColorField::Icon => self.icon,
            ThemeColorField::IconMuted => self.icon_muted,
            ThemeColorField::IconDisabled => self.icon_disabled,
            ThemeColorField::IconPlaceholder => self.icon_placeholder,
            ThemeColorField::IconAccent => self.icon_accent,
            ThemeColorField::StatusBarBackground => self.status_bar_background,
            ThemeColorField::TitleBarBackground => self.title_bar_background,
            ThemeColorField::TitleBarInactiveBackground => self.title_bar_inactive_background,
            ThemeColorField::ToolbarBackground => self.toolbar_background,
            ThemeColorField::TabBarBackground => self.tab_bar_background,
            ThemeColorField::TabInactiveBackground => self.tab_inactive_background,
            ThemeColorField::TabActiveBackground => self.tab_active_background,
            ThemeColorField::SearchMatchBackground => self.search_match_background,
            ThemeColorField::SearchActiveMatchBackground => self.search_active_match_background,
            ThemeColorField::PanelBackground => self.panel_background,
            ThemeColorField::PanelFocusedBorder => self.panel_focused_border,
            ThemeColorField::PanelIndentGuide => self.panel_indent_guide,
            ThemeColorField::PanelIndentGuideHover => self.panel_indent_guide_hover,
            ThemeColorField::PanelIndentGuideActive => self.panel_indent_guide_active,
            ThemeColorField::PanelOverlayBackground => self.panel_overlay_background,
            ThemeColorField::PanelOverlayHover => self.panel_overlay_hover,
            ThemeColorField::PaneFocusedBorder => self.pane_focused_border,
            ThemeColorField::PaneGroupBorder => self.pane_group_border,
            ThemeColorField::ScrollbarThumbBackground => self.scrollbar_thumb_background,
            ThemeColorField::ScrollbarThumbHoverBackground => self.scrollbar_thumb_hover_background,
            ThemeColorField::ScrollbarThumbActiveBackground => {
                self.scrollbar_thumb_active_background
            }
            ThemeColorField::ScrollbarThumbBorder => self.scrollbar_thumb_border,
            ThemeColorField::ScrollbarTrackBackground => self.scrollbar_track_background,
            ThemeColorField::ScrollbarTrackBorder => self.scrollbar_track_border,
            ThemeColorField::MinimapThumbBackground => self.minimap_thumb_background,
            ThemeColorField::MinimapThumbHoverBackground => self.minimap_thumb_hover_background,
            ThemeColorField::MinimapThumbActiveBackground => self.minimap_thumb_active_background,
            ThemeColorField::MinimapThumbBorder => self.minimap_thumb_border,

            ThemeColorField::TerminalBackground => self.terminal_background,
            ThemeColorField::TerminalForeground => self.terminal_foreground,
            ThemeColorField::TerminalBrightForeground => self.terminal_bright_foreground,
            ThemeColorField::TerminalDimForeground => self.terminal_dim_foreground,
            ThemeColorField::TerminalAnsiBackground => self.terminal_ansi_background,
            ThemeColorField::TerminalAnsiBlack => self.terminal_ansi_black,
            ThemeColorField::TerminalAnsiBrightBlack => self.terminal_ansi_bright_black,
            ThemeColorField::TerminalAnsiDimBlack => self.terminal_ansi_dim_black,
            ThemeColorField::TerminalAnsiRed => self.terminal_ansi_red,
            ThemeColorField::TerminalAnsiBrightRed => self.terminal_ansi_bright_red,
            ThemeColorField::TerminalAnsiDimRed => self.terminal_ansi_dim_red,
            ThemeColorField::TerminalAnsiGreen => self.terminal_ansi_green,
            ThemeColorField::TerminalAnsiBrightGreen => self.terminal_ansi_bright_green,
            ThemeColorField::TerminalAnsiDimGreen => self.terminal_ansi_dim_green,
            ThemeColorField::TerminalAnsiYellow => self.terminal_ansi_yellow,
            ThemeColorField::TerminalAnsiBrightYellow => self.terminal_ansi_bright_yellow,
            ThemeColorField::TerminalAnsiDimYellow => self.terminal_ansi_dim_yellow,
            ThemeColorField::TerminalAnsiBlue => self.terminal_ansi_blue,
            ThemeColorField::TerminalAnsiBrightBlue => self.terminal_ansi_bright_blue,
            ThemeColorField::TerminalAnsiDimBlue => self.terminal_ansi_dim_blue,
            ThemeColorField::TerminalAnsiMagenta => self.terminal_ansi_magenta,
            ThemeColorField::TerminalAnsiBrightMagenta => self.terminal_ansi_bright_magenta,
            ThemeColorField::TerminalAnsiDimMagenta => self.terminal_ansi_dim_magenta,
            ThemeColorField::TerminalAnsiCyan => self.terminal_ansi_cyan,
            ThemeColorField::TerminalAnsiBrightCyan => self.terminal_ansi_bright_cyan,
            ThemeColorField::TerminalAnsiDimCyan => self.terminal_ansi_dim_cyan,
            ThemeColorField::TerminalAnsiWhite => self.terminal_ansi_white,
            ThemeColorField::TerminalAnsiBrightWhite => self.terminal_ansi_bright_white,
            ThemeColorField::TerminalAnsiDimWhite => self.terminal_ansi_dim_white,
            ThemeColorField::LinkTextHover => self.link_text_hover,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (ThemeColorField, Hsla)> + '_ {
        ThemeColorField::iter().map(move |field| (field, self.color(field)))
    }

    pub fn to_vec(&self) -> Vec<(ThemeColorField, Hsla)> {
        self.iter().collect()
    }
}

pub fn all_theme_colors(cx: &mut App) -> Vec<(Hsla, SharedString)> {
    let theme = cx.theme();
    ThemeColorField::iter()
        .map(|field| {
            let color = theme.colors().color(field);
            let name = field.as_ref().to_string();
            (color, SharedString::from(name))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeStyles {
    /// The background appearance of the window.
    pub window_background_appearance: WindowBackgroundAppearance,
    pub system: SystemColors,
    pub accents: AccentColors,
    /// An array of colors used for theme elements that iterate through a series of colors.
    ///
    ///
    pub colors: ThemeColors,

    pub status: StatusColors,
}

#[derive(Refineable, Clone, Debug, PartialEq)]
#[refineable(Debug, serde::Deserialize)]
pub struct StatusColors {
    /// Indicates some kind of conflict, like a file changed on disk while it was open, or
    /// merge conflicts in a Git repository.
    pub conflict: Hsla,
    pub conflict_background: Hsla,
    pub conflict_border: Hsla,

    /// Indicates something new, like a new file added to a Git repository.
    pub created: Hsla,
    pub created_background: Hsla,
    pub created_border: Hsla,

    /// Indicates that something no longer exists, like a deleted file.
    pub deleted: Hsla,
    pub deleted_background: Hsla,
    pub deleted_border: Hsla,

    /// Indicates a system error, a failed operation or a diagnostic error.
    pub error: Hsla,
    pub error_background: Hsla,
    pub error_border: Hsla,

    /// Represents a hidden status, such as a file being hidden in a file tree.
    pub hidden: Hsla,
    pub hidden_background: Hsla,
    pub hidden_border: Hsla,

    /// Indicates a hint or some kind of additional information.
    pub hint: Hsla,
    pub hint_background: Hsla,
    pub hint_border: Hsla,

    /// Indicates that something is deliberately ignored, such as a file or operation ignored by Git.
    pub ignored: Hsla,
    pub ignored_background: Hsla,
    pub ignored_border: Hsla,

    /// Represents informational status updates or messages.
    pub info: Hsla,
    pub info_background: Hsla,
    pub info_border: Hsla,

    /// Indicates a changed or altered status, like a file that has been edited.
    pub modified: Hsla,
    pub modified_background: Hsla,
    pub modified_border: Hsla,

    /// Indicates something that is predicted, like automatic code completion, or generated code.
    pub predictive: Hsla,
    pub predictive_background: Hsla,
    pub predictive_border: Hsla,

    /// Represents a renamed status, such as a file that has been renamed.
    pub renamed: Hsla,
    pub renamed_background: Hsla,
    pub renamed_border: Hsla,

    /// Indicates a successful operation or task completion.
    pub success: Hsla,
    pub success_background: Hsla,
    pub success_border: Hsla,

    /// Indicates some kind of unreachable status, like a block of code that can never be reached.
    pub unreachable: Hsla,
    pub unreachable_background: Hsla,
    pub unreachable_border: Hsla,

    /// Represents a warning status, like an operation that is about to fail.
    pub warning: Hsla,
    pub warning_background: Hsla,
    pub warning_border: Hsla,
}

pub struct DiagnosticColors {
    pub error: Hsla,
    pub warning: Hsla,
    pub info: Hsla,
}

impl StatusColors {
    pub fn dark() -> Self {
        Self {
            conflict: red().dark().step_9(),
            conflict_background: red().dark().step_9(),
            conflict_border: red().dark().step_9(),
            created: grass().dark().step_9(),
            created_background: grass().dark().step_9().opacity(0.25),
            created_border: grass().dark().step_9(),
            deleted: red().dark().step_9(),
            deleted_background: red().dark().step_9().opacity(0.25),
            deleted_border: red().dark().step_9(),
            error: red().dark().step_9(),
            error_background: red().dark().step_9(),
            error_border: red().dark().step_9(),
            hidden: neutral().dark().step_9(),
            hidden_background: neutral().dark().step_9(),
            hidden_border: neutral().dark().step_9(),
            hint: blue().dark().step_9(),
            hint_background: blue().dark().step_9(),
            hint_border: blue().dark().step_9(),
            ignored: neutral().dark().step_9(),
            ignored_background: neutral().dark().step_9(),
            ignored_border: neutral().dark().step_9(),
            info: blue().dark().step_9(),
            info_background: blue().dark().step_9(),
            info_border: blue().dark().step_9(),
            modified: yellow().dark().step_9(),
            modified_background: yellow().dark().step_9().opacity(0.25),
            modified_border: yellow().dark().step_9(),
            predictive: neutral().dark_alpha().step_9(),
            predictive_background: neutral().dark_alpha().step_9(),
            predictive_border: neutral().dark_alpha().step_9(),
            renamed: blue().dark().step_9(),
            renamed_background: blue().dark().step_9(),
            renamed_border: blue().dark().step_9(),
            success: grass().dark().step_9(),
            success_background: grass().dark().step_9(),
            success_border: grass().dark().step_9(),
            unreachable: neutral().dark().step_10(),
            unreachable_background: neutral().dark().step_10(),
            unreachable_border: neutral().dark().step_10(),
            warning: yellow().dark().step_9(),
            warning_background: yellow().dark().step_9(),
            warning_border: yellow().dark().step_9(),
        }
    }

    pub fn light() -> Self {
        Self {
            conflict: red().light().step_9(),
            conflict_background: red().light().step_9(),
            conflict_border: red().light().step_9(),
            created: grass().light().step_9(),
            created_background: grass().light().step_9(),
            created_border: grass().light().step_9(),
            deleted: red().light().step_9(),
            deleted_background: red().light().step_9(),
            deleted_border: red().light().step_9(),
            error: red().light().step_9(),
            error_background: red().light().step_9(),
            error_border: red().light().step_9(),
            hidden: neutral().light().step_9(),
            hidden_background: neutral().light().step_9(),
            hidden_border: neutral().light().step_9(),
            hint: blue().light().step_9(),
            hint_background: blue().light().step_9(),
            hint_border: blue().light().step_9(),
            ignored: neutral().light().step_9(),
            ignored_background: neutral().light().step_9(),
            ignored_border: neutral().light().step_9(),
            info: blue().light().step_9(),
            info_background: blue().light().step_9(),
            info_border: blue().light().step_9(),
            modified: yellow().light().step_9(),
            modified_background: yellow().light().step_9(),
            modified_border: yellow().light().step_9(),
            predictive: neutral().light_alpha().step_9(),
            predictive_background: neutral().light_alpha().step_9(),
            predictive_border: neutral().light_alpha().step_9(),
            renamed: blue().light().step_9(),
            renamed_background: blue().light().step_9(),
            renamed_border: blue().light().step_9(),
            success: grass().light().step_9(),
            success_background: grass().light().step_9(),
            success_border: grass().light().step_9(),
            unreachable: neutral().light().step_10(),
            unreachable_background: neutral().light().step_10(),
            unreachable_border: neutral().light().step_10(),
            warning: yellow().light().step_9(),
            warning_background: yellow().light().step_9(),
            warning_border: yellow().light().step_9(),
        }
    }

    pub fn diagnostic(&self) -> DiagnosticColors {
        DiagnosticColors {
            error: self.error,
            warning: self.warning,
            info: self.info,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AccentColors(pub Arc<[Hsla]>);
impl Default for AccentColors {
    /// Don't use this!
    /// We have to have a default to be `[refineable::Refinable]`.
    /// TODO "Find a way to not need this for Refinable"
    fn default() -> Self {
        Self::dark()
    }
}

impl AccentColors {
    /// Returns the set of dark accent colors.
    pub fn dark() -> Self {
        Self(Arc::from(vec![
            blue().dark().step_9(),
            orange().dark().step_9(),
            pink().dark().step_9(),
            lime().dark().step_9(),
            purple().dark().step_9(),
            amber().dark().step_9(),
            jade().dark().step_9(),
            tomato().dark().step_9(),
            cyan().dark().step_9(),
            gold().dark().step_9(),
            grass().dark().step_9(),
            indigo().dark().step_9(),
            iris().dark().step_9(),
        ]))
    }

    /// Returns the set of light accent colors.
    pub fn light() -> Self {
        Self(Arc::from(vec![
            blue().light().step_9(),
            orange().light().step_9(),
            pink().light().step_9(),
            lime().light().step_9(),
            purple().light().step_9(),
            amber().light().step_9(),
            jade().light().step_9(),
            tomato().light().step_9(),
            cyan().light().step_9(),
            gold().light().step_9(),
            grass().light().step_9(),
            indigo().light().step_9(),
            iris().light().step_9(),
        ]))
    }
}

impl AccentColors {
    /// Returns the color for the given index.
    pub fn color_for_index(&self, index: u32) -> Hsla {
        self.0[index as usize % self.0.len()]
    }
}
