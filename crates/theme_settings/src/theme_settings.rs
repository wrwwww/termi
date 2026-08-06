pub mod schema;
use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use gpui::{App, Font, FontFallbacks, FontStyle, Global, Pixels, SharedString, Window, px};
use serde::{Deserialize, Serialize};
use settings::{Settings, content_into_gpui::IntoGpui, settings_store::SettingsStore};
use settings_content::{
    SettingsContent,
    theme::{
        BufferLineHeight, DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME, FontFamilyName,
        ThemeAppearanceMode, ThemeName, ThemeStyleContent,
    },
};
use settings_macros::RegisterSetting;
use theme::{
    Appearance, AppearanceContent, GlobalTheme, LoadThemes, SystemAppearance, Theme, ThemeFamily,
    colors::{StatusColors, ThemeColors, ThemeStyles},
    default_colors::{SystemColors, default_color_scales},
    registry::ThemeRegistry,
};

use crate::schema::{ThemeContent, ThemeFamilyContent};

/// Initialize the theme system with settings integration.
///
/// This is the full initialization for the application. It calls [`theme::init`]
/// and then wires up settings observation for theme/font changes.
pub fn init(themes_to_load: LoadThemes, cx: &mut App) {
    let load_user_themes = matches!(&themes_to_load, LoadThemes::All(_));

    theme::init(themes_to_load, cx);
    // theme::set_theme_settings_provider(Box::new(ThemeSettingsProviderImpl), cx);

    if load_user_themes {
        let registry = ThemeRegistry::global(cx);
        load_bundled_themes(&registry);
    }

    let theme = configured_theme(cx);
    // let icon_theme = configured_icon_theme(cx);
    GlobalTheme::update_theme(cx, theme);
    // GlobalTheme::update_icon_theme(cx, icon_theme);

    let settings = ThemeSettings::get_global(cx);

    let mut prev_buffer_font_size_settings = settings.buffer_font_size_settings();
    let mut prev_ui_font_size_settings = settings.ui_font_size_settings();
    let mut prev_theme_name = settings.theme.name(SystemAppearance::global(cx).0);
    // let mut prev_icon_theme_name = settings.icon_theme.name(SystemAppearance::global(cx).0);
    let mut prev_theme_overrides = (
        settings.experimental_theme_overrides.clone(),
        settings.theme_overrides.clone(),
    );

    cx.observe_global::<SettingsStore>(move |cx| {
        let settings = ThemeSettings::get_global(cx);

        let buffer_font_size_settings = settings.buffer_font_size_settings();
        let ui_font_size_settings = settings.ui_font_size_settings();
        let theme_name = settings.theme.name(SystemAppearance::global(cx).0);
        // let icon_theme_name = settings.icon_theme.name(SystemAppearance::global(cx).0);
        let theme_overrides = (
            settings.experimental_theme_overrides.clone(),
            settings.theme_overrides.clone(),
        );

        if buffer_font_size_settings != prev_buffer_font_size_settings {
            prev_buffer_font_size_settings = buffer_font_size_settings;
            reset_buffer_font_size(cx);
        }

        if ui_font_size_settings != prev_ui_font_size_settings {
            prev_ui_font_size_settings = ui_font_size_settings;
            reset_ui_font_size(cx);
        }

        if theme_name != prev_theme_name || theme_overrides != prev_theme_overrides {
            prev_theme_name = theme_name;
            prev_theme_overrides = theme_overrides;
            reload_theme(cx);
        }
    })
    .detach();
}

/// Gets the font size, adjusted by the difference between the current buffer font size and the one set in the settings.
pub fn adjusted_font_size(size: Pixels, cx: &App) -> Pixels {
    let adjusted_font_size =
        if let Some(BufferFontSize(adjusted_size)) = cx.try_global::<BufferFontSize>() {
            let buffer_font_size = ThemeSettings::get_global(cx).buffer_font_size;
            let delta = *adjusted_size - buffer_font_size;
            size + delta
        } else {
            size
        };
    clamp_font_size(adjusted_font_size)
}

/// Adjusts the buffer font size, without persisting the result in the settings.
/// This will be effective until the app is restarted.
pub fn adjust_buffer_font_size(cx: &mut App, f: impl FnOnce(Pixels) -> Pixels) {
    let buffer_font_size = ThemeSettings::get_global(cx).buffer_font_size;
    let adjusted_size = cx
        .try_global::<BufferFontSize>()
        .map_or(buffer_font_size, |adjusted_size| adjusted_size.0);
    cx.set_global(BufferFontSize(clamp_font_size(f(adjusted_size))));
    cx.refresh_windows();
}

/// Resets the buffer font size to the default value.
pub fn reset_buffer_font_size(cx: &mut App) {
    if cx.has_global::<BufferFontSize>() {
        cx.remove_global::<BufferFontSize>();
        cx.refresh_windows();
    }
}

#[allow(missing_docs)]
pub fn setup_ui_font(window: &mut Window, cx: &mut App) -> gpui::Font {
    let (ui_font, ui_font_size) = {
        let theme_settings = ThemeSettings::get_global(cx);
        let font = theme_settings.ui_font.clone();
        (font, theme_settings.ui_font_size(cx))
    };

    window.set_rem_size(ui_font_size);
    ui_font
}

/// Sets the adjusted UI font size.
pub fn adjust_ui_font_size(cx: &mut App, f: impl FnOnce(Pixels) -> Pixels) {
    let ui_font_size = ThemeSettings::get_global(cx).ui_font_size(cx);
    let adjusted_size = cx
        .try_global::<UiFontSize>()
        .map_or(ui_font_size, |adjusted_size| adjusted_size.0);
    cx.set_global(UiFontSize(clamp_font_size(f(adjusted_size))));
    cx.refresh_windows();
}

/// Resets the UI font size to the default value.
pub fn reset_ui_font_size(cx: &mut App) {
    if cx.has_global::<UiFontSize>() {
        cx.remove_global::<UiFontSize>();
        cx.refresh_windows();
    }
}

/// Ensures font size is within the valid range.
pub fn clamp_font_size(size: Pixels) -> Pixels {
    size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
}

fn font_fallbacks_from_settings(fallbacks: Option<Vec<FontFamilyName>>) -> Option<FontFallbacks> {
    fallbacks.map(|fallbacks| {
        FontFallbacks::from_fonts(
            fallbacks
                .into_iter()
                .map(|font_family| font_family.0.to_string())
                .collect(),
        )
    })
}

/// Loads the themes bundled with the Zed binary into the registry.
pub fn load_bundled_themes(registry: &ThemeRegistry) {
    let theme_paths = registry
        .assets()
        .list("themes/")
        .expect("failed to list theme assets")
        .into_iter()
        .filter(|path| path.ends_with(".json"));

    for path in theme_paths {
        let Some(theme) = registry.assets().load(&path).unwrap() else {
            continue;
        };

        let Some(theme_family) = serde_json::from_slice(&theme)
            .with_context(|| format!("failed to parse theme at path \"{path}\""))
            .unwrap()
        else {
            continue;
        };

        let refined = refine_theme_family(theme_family);
        registry.insert_theme_families([refined]);
    }
}

/// Loads a user theme from the given bytes into the registry.
pub fn load_user_theme(registry: &ThemeRegistry, bytes: &[u8]) -> Result<()> {
    let theme = deserialize_user_theme(bytes)?;
    let refined = refine_theme_family(theme);
    registry.insert_theme_families([refined]);
    Ok(())
}

/// Deserializes a user theme from the given bytes.
pub fn deserialize_user_theme(bytes: &[u8]) -> Result<ThemeFamilyContent> {
    let theme_family: ThemeFamilyContent = serde_json_lenient::from_slice(bytes)?;

    for theme in &theme_family.themes {
        if theme
            .style
            .colors
            .deprecated_scrollbar_thumb_background
            .is_some()
        {
            log::warn!(
                r#"Theme "{theme_name}" is using a deprecated style property: scrollbar_thumb.background. Use `scrollbar.thumb.background` instead."#,
                theme_name = theme.name
            )
        }
    }

    Ok(theme_family)
}
/// Refines a [`ThemeFamilyContent`] and its [`ThemeContent`]s into a [`ThemeFamily`].
pub fn refine_theme_family(theme_family_content: ThemeFamilyContent) -> ThemeFamily {
    let id = uuid::Uuid::new_v4().to_string();
    let name = theme_family_content.name.clone();
    let author = theme_family_content.author.clone();

    let themes: Vec<Theme> = theme_family_content
        .themes
        .iter()
        .map(|theme_content| refine_theme(theme_content))
        .collect();

    ThemeFamily {
        id,
        name: name.into(),
        author: author.into(),
        themes,
        scales: default_color_scales(),
    }
}

/// Refines a [`ThemeContent`] into a [`Theme`].
pub fn refine_theme(theme: &ThemeContent) -> Theme {
    let appearance = match theme.appearance {
        AppearanceContent::Light => Appearance::Light,
        AppearanceContent::Dark => Appearance::Dark,
    };

    let mut refined_theme_colors = match theme.appearance {
        AppearanceContent::Light => ThemeColors::light(),
        AppearanceContent::Dark => ThemeColors::dark(),
    };

    let window_background_appearance = theme
        .style
        .window_background_appearance
        .map(|w| w.into_gpui())
        .unwrap_or_default();

    Theme {
        id: uuid::Uuid::new_v4().to_string(),
        name: theme.name.clone().into(),
        appearance,
        styles: ThemeStyles {
            system: SystemColors::default(),
            window_background_appearance,

            colors: refined_theme_colors.clone(),
            status: StatusColors::dark(),
        },
    }
}
/// Reloads the current theme from settings.
pub fn reload_theme(cx: &mut App) {
    let theme = configured_theme(cx);
    GlobalTheme::update_theme(cx, theme);
    cx.refresh_windows();
}
fn configured_theme(cx: &mut App) -> Arc<Theme> {
    let themes = ThemeRegistry::default_global(cx);
    let theme_settings = ThemeSettings::get_global(cx);
    let system_appearance = SystemAppearance::global(cx);

    let theme_name = theme_settings.theme.name(*system_appearance);

    let theme = match themes.get(&theme_name.0) {
        Ok(theme) => theme,
        Err(err) => {
            if themes.extensions_loaded() {
                log::error!("{err}");
            }
            themes
                .get(default_theme(*system_appearance))
                .unwrap_or_else(|_| themes.get(DEFAULT_DARK_THEME).unwrap())
        }
    };
    theme_settings.apply_theme_overrides(theme)
}

#[derive(Clone, PartialEq, RegisterSetting)]
pub struct ThemeSettings {
    /// The UI font size. Determines the size of text in the UI,
    /// as well as the size of a [gpui::Rems] unit.
    ///
    /// Changing this will impact the size of all UI elements.
    ui_font_size: Pixels,
    /// The font used for UI elements.
    pub ui_font: Font,
    /// The font size used for buffers, and the terminal.
    ///
    /// The terminal font size can be overridden using it's own setting.
    buffer_font_size: Pixels,
    /// The font used for buffers, and the terminal.
    ///
    /// The terminal font family can be overridden using it's own setting.
    pub buffer_font: Font,
    /// The agent UI font family. Determines the family of response text in the agent panel.
    /// Falls back to the UI font family if unset.
    agent_ui_font_family: Option<SharedString>,
    /// The agent font size. Determines the size of text in the agent panel. Falls back to the UI font size if unset.
    agent_ui_font_size: Option<Pixels>,
    /// The agent buffer font family. Determines the family of user messages in the agent panel.
    /// Falls back to the buffer font family if unset.
    agent_buffer_font_family: Option<SharedString>,
    /// The agent buffer font size. Determines the size of user messages in the agent panel.
    agent_buffer_font_size: Option<Pixels>,
    git_commit_buffer_font_size: Option<Pixels>,
    /// The font family to use for rendering in the markdown preview.
    /// Falls back to the UI font family if unset.
    markdown_preview_font_family: Option<SharedString>,
    /// The font family to use for code in the markdown preview.
    /// Falls back to the buffer font family if unset.
    markdown_preview_code_font_family: Option<SharedString>,
    /// The font size to use for rendering in the markdown preview.
    /// Falls back to the UI font size if unset.
    markdown_preview_font_size: Option<Pixels>,
    /// The theme to use for the markdown preview.
    /// Falls back to the main editor theme if unset.
    pub markdown_preview_theme: Option<ThemeSelection>,
    /// The line height for buffers, and the terminal.
    ///
    /// Changing this may affect the spacing of some UI elements.
    ///
    /// The terminal font family can be overridden using it's own setting.
    pub buffer_line_height: BufferLineHeight,
    /// The current theme selection.
    pub theme: ThemeSelection,
    /// Manual overrides for the active theme.
    ///
    /// Note: This setting is still experimental. See [this tracking issue](https://github.com/zed-industries/zed/issues/18078)
    pub experimental_theme_overrides: Option<ThemeStyleContent>,
    /// Manual overrides per theme
    pub theme_overrides: HashMap<String, ThemeStyleContent>,
}

/// Returns the name of the default theme for the given [`Appearance`].
pub fn default_theme(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::Light => DEFAULT_LIGHT_THEME,
        Appearance::Dark => DEFAULT_DARK_THEME,
    }
}

const MIN_FONT_SIZE: Pixels = px(6.0);
const MAX_FONT_SIZE: Pixels = px(100.0);
const MIN_LINE_HEIGHT: f32 = 1.0;

impl Settings for ThemeSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let content = &content.theme;
        let theme_selection: ThemeSelection = content.theme.clone().unwrap().into();
        // let icon_theme_selection: IconThemeSelection = content.icon_theme.clone().unwrap().into();
        Self {
            ui_font_size: clamp_font_size(content.ui_font_size.unwrap().into_gpui()),
            ui_font: Font {
                family: content.ui_font_family.as_ref().unwrap().0.clone().into(),
                features: content.ui_font_features.clone().unwrap().into_gpui(),
                fallbacks: font_fallbacks_from_settings(content.ui_font_fallbacks.clone()),
                weight: content.ui_font_weight.unwrap().into_gpui(),
                style: Default::default(),
            },
            buffer_font: Font {
                family: content
                    .buffer_font_family
                    .as_ref()
                    .unwrap()
                    .0
                    .clone()
                    .into(),
                features: content.buffer_font_features.clone().unwrap().into_gpui(),
                fallbacks: font_fallbacks_from_settings(content.buffer_font_fallbacks.clone()),
                weight: content.buffer_font_weight.unwrap().into_gpui(),
                style: FontStyle::default(),
            },
            buffer_font_size: clamp_font_size(content.buffer_font_size.unwrap().into_gpui()),
            buffer_line_height: content.buffer_line_height.unwrap().into(),
            agent_ui_font_family: content
                .agent_ui_font_family
                .as_ref()
                .map(|font| font.0.clone().into()),
            agent_ui_font_size: content.agent_ui_font_size.map(|s| s.into_gpui()),
            agent_buffer_font_family: content
                .agent_buffer_font_family
                .as_ref()
                .map(|font| font.0.clone().into()),
            agent_buffer_font_size: content.agent_buffer_font_size.map(|s| s.into_gpui()),
            git_commit_buffer_font_size: content.git_commit_buffer_font_size.map(|s| s.into_gpui()),
            markdown_preview_font_family: content
                .markdown_preview_font_family
                .as_ref()
                .map(|f| f.0.clone().into()),
            markdown_preview_code_font_family: content
                .markdown_preview_code_font_family
                .as_ref()
                .map(|f| f.0.clone().into()),
            markdown_preview_font_size: content.markdown_preview_font_size.map(|s| s.into_gpui()),
            markdown_preview_theme: content
                .markdown_preview_theme
                .clone()
                .map(ThemeSelection::from),
            theme: theme_selection,
            experimental_theme_overrides: content.experimental_theme_overrides.clone(),
            theme_overrides: content.theme_overrides.clone(),
            // icon_theme: icon_theme_selection,
            // ui_density: ui_density_from_settings(content.ui_density.unwrap_or_default()),
            // unnecessary_code_fade: content.unnecessary_code_fade.unwrap().0.clamp(0.0, 0.9),
        }
    }
}

impl ThemeSettings {
    /// Returns the buffer font size.
    pub fn buffer_font_size(&self, cx: &App) -> Pixels {
        let font_size = cx
            .try_global::<BufferFontSize>()
            .map(|size| size.0)
            .unwrap_or(self.buffer_font_size);
        clamp_font_size(font_size)
    }

    /// Returns the UI font size.
    pub fn ui_font_size(&self, cx: &App) -> Pixels {
        let font_size = cx
            .try_global::<UiFontSize>()
            .map(|size| size.0)
            .unwrap_or(self.ui_font_size);
        clamp_font_size(font_size)
    }

    // /// Returns the agent panel font size. Falls back to the UI font size if unset.
    // pub fn agent_ui_font_size(&self, cx: &App) -> Pixels {
    //     cx.try_global::<AgentUiFontSize>()
    //         .map(|size| size.0)
    //         .or(self.agent_ui_font_size)
    //         .map(clamp_font_size)
    //         .unwrap_or_else(|| self.ui_font_size(cx))
    // }

    // pub fn agent_ui_font_family(&self) -> &SharedString {
    //     self.agent_ui_font_family
    //         .as_ref()
    //         .unwrap_or(&self.ui_font.family)
    // }

    // /// Returns the agent panel buffer font size.
    // pub fn agent_buffer_font_size(&self, cx: &App) -> Pixels {
    //     cx.try_global::<AgentBufferFontSize>()
    //         .map(|size| size.0)
    //         .or(self.agent_buffer_font_size)
    //         .map(clamp_font_size)
    //         .unwrap_or_else(|| self.buffer_font_size(cx))
    // }

    // pub fn agent_buffer_font_family(&self) -> &SharedString {
    //     self.agent_buffer_font_family
    //         .as_ref()
    //         .unwrap_or(&self.buffer_font.family)
    // }

    // pub fn git_commit_buffer_font_size(&self, cx: &App) -> Pixels {
    //     cx.try_global::<GitCommitBufferFontSize>()
    //         .map(|size| size.0)
    //         .or(self.git_commit_buffer_font_size)
    //         .map(clamp_font_size)
    //         .unwrap_or_else(|| self.buffer_font_size(cx))
    // }

    /// Returns the font family to use in the markdown preview,
    /// falling back to the UI font family when unset.
    pub fn markdown_preview_font_family(&self) -> &SharedString {
        self.markdown_preview_font_family
            .as_ref()
            .unwrap_or(&self.ui_font.family)
    }

    /// Returns the font family to use for code in the markdown preview,
    /// falling back to the buffer font family when unset.
    pub fn markdown_preview_code_font_family(&self) -> &SharedString {
        self.markdown_preview_code_font_family
            .as_ref()
            .unwrap_or(&self.buffer_font.family)
    }

    /// Returns the markdown preview font size.
    ///
    /// Note: the fallback deliberately uses `self.ui_font_size` instead of `ui_font_size(cx)`,
    /// so that temporary UI zoom does not also resize the markdown preview.
    pub fn markdown_preview_font_size(&self, cx: &App) -> Pixels {
        cx.try_global::<MarkdownPreviewFontSize>()
            .map(|size| size.0)
            .or(self.markdown_preview_font_size)
            .map(clamp_font_size)
            .unwrap_or_else(|| clamp_font_size(self.ui_font_size))
    }

    /// Returns the buffer font size, read from the settings.
    ///
    /// The real buffer font size is stored in-memory, to support temporary font size changes.
    /// Use [`Self::buffer_font_size`] to get the real font size.
    pub fn buffer_font_size_settings(&self) -> Pixels {
        self.buffer_font_size
    }

    /// Returns the UI font size, read from the settings.
    ///
    /// The real UI font size is stored in-memory, to support temporary font size changes.
    /// Use [`Self::ui_font_size`] to get the real font size.
    pub fn ui_font_size_settings(&self) -> Pixels {
        self.ui_font_size
    }

    /// Returns the buffer's line height.
    pub fn line_height(&self) -> f32 {
        f32::max(self.buffer_line_height.value(), MIN_LINE_HEIGHT)
    }

    /// Applies the theme overrides, if there are any, to the current theme.
    pub fn apply_theme_overrides(&self, mut arc_theme: Arc<Theme>) -> Arc<Theme> {
        if let Some(experimental_theme_overrides) = &self.experimental_theme_overrides {
            let mut theme = (*arc_theme).clone();
            ThemeSettings::modify_theme(&mut theme, experimental_theme_overrides);
            arc_theme = Arc::new(theme);
        }

        if let Some(theme_overrides) = self.theme_overrides.get(arc_theme.name.as_ref()) {
            let mut theme = (*arc_theme).clone();
            ThemeSettings::modify_theme(&mut theme, theme_overrides);
            arc_theme = Arc::new(theme);
        }

        arc_theme
    }

    fn modify_theme(base_theme: &mut Theme, theme_overrides: &ThemeStyleContent) {
        if let Some(window_background_appearance) = theme_overrides.window_background_appearance {
            base_theme.styles.window_background_appearance =
                window_background_appearance.into_gpui();
        }
        // let status_color_refinement = status_colors_refinement(&theme_overrides.status);

        // let theme_color_refinement = theme_colors_refinement(
        //     &theme_overrides.colors,
        //     &status_color_refinement,
        //     base_theme.appearance.is_light(),
        // );
        // base_theme.styles.colors.refine(&theme_color_refinement);
        // base_theme.styles.status.refine(&status_color_refinement);
        // merge_player_colors(&mut base_theme.styles.player, &theme_overrides.players);
        // merge_accent_colors(&mut base_theme.styles.accents, &theme_overrides.accents);
        // base_theme.styles.syntax = SyntaxTheme::merge(
        //     base_theme.styles.syntax.clone(),
        //     syntax_overrides(theme_overrides),
        // );
    }
}

/// Represents the selection of a theme, which can be either static or dynamic.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ThemeSelection {
    /// A static theme selection, represented by a single theme name.
    Static(ThemeName),
    /// A dynamic theme selection, which can change based the [ThemeMode].
    Dynamic {
        /// The mode used to determine which theme to use.
        #[serde(default)]
        mode: ThemeAppearanceMode,
        /// The theme to use for light mode.
        light: ThemeName,
        /// The theme to use for dark mode.
        dark: ThemeName,
    },
}

impl From<settings_content::theme::ThemeSelection> for ThemeSelection {
    fn from(selection: settings_content::theme::ThemeSelection) -> Self {
        match selection {
            settings_content::theme::ThemeSelection::Static(theme) => ThemeSelection::Static(theme),
            settings_content::theme::ThemeSelection::Dynamic { mode, light, dark } => {
                ThemeSelection::Dynamic { mode, light, dark }
            }
        }
    }
}

impl ThemeSelection {
    /// Returns the theme name for the selected [ThemeMode].
    pub fn name(&self, system_appearance: Appearance) -> ThemeName {
        match self {
            Self::Static(theme) => theme.clone(),
            Self::Dynamic { mode, light, dark } => match mode {
                ThemeAppearanceMode::Light => light.clone(),
                ThemeAppearanceMode::Dark => dark.clone(),
                ThemeAppearanceMode::System => match system_appearance {
                    Appearance::Light => light.clone(),
                    Appearance::Dark => dark.clone(),
                },
            },
        }
    }

    /// Returns the [ThemeMode] for the [ThemeSelection].
    pub fn mode(&self) -> Option<ThemeAppearanceMode> {
        match self {
            ThemeSelection::Static(_) => None,
            ThemeSelection::Dynamic { mode, .. } => Some(*mode),
        }
    }
}
#[derive(Default)]
struct BufferFontSize(Pixels);

impl Global for BufferFontSize {}

#[derive(Default)]
pub(crate) struct UiFontSize(Pixels);

impl Global for UiFontSize {}

/// In-memory override for the UI font size in the agent panel.
#[derive(Default)]
pub struct AgentUiFontSize(Pixels);

impl Global for AgentUiFontSize {}

/// In-memory override for the buffer font size in the agent panel.
#[derive(Default)]
pub struct AgentBufferFontSize(Pixels);

impl Global for AgentBufferFontSize {}

#[derive(Default)]
pub struct GitCommitBufferFontSize(Pixels);

impl Global for GitCommitBufferFontSize {}

/// In-memory override for the markdown preview font size.
#[derive(Default)]
pub struct MarkdownPreviewFontSize(Pixels);

impl Global for MarkdownPreviewFontSize {}
