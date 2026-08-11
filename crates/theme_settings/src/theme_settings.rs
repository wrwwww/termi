pub mod schema;
use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use gpui::{App, Font, FontFallbacks, FontStyle, Global, Hsla, Pixels, SharedString, Window, px};
use log::info;
use palette::convert::FromColorUnclamped;
use refineable::Refineable;
use serde::{Deserialize, Serialize};
use settings::{Settings, content_into_gpui::IntoGpui, settings_store::SettingsStore};
use settings_content::{
    SettingsContent,
    theme::{
        AccentContent, BufferLineHeight, DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME, FontFamilyName,
        StatusColorsContent, ThemeAppearanceMode, ThemeColorsContent, ThemeName, ThemeStyleContent,
    },
};
use settings_macros::RegisterSetting;
use theme::{
    Appearance, AppearanceContent, GlobalTheme, LoadThemes, SystemAppearance, Theme, ThemeFamily,
    colors::{
        AccentColors, StatusColors, StatusColorsRefinement, ThemeColors, ThemeColorsRefinement,
        ThemeStyles,
    },
    default_colors::{SystemColors, default_color_scales},
    registry::ThemeRegistry,
};

use crate::schema::{ThemeContent, ThemeFamilyContent};
pub trait ThemeSettingsProvider: Send + Sync + 'static {
    /// Returns the font used for UI elements.
    fn ui_font<'a>(&'a self, cx: &'a App) -> &'a Font;

    /// Returns the font used for buffers and the terminal.
    fn buffer_font<'a>(&'a self, cx: &'a App) -> &'a Font;

    /// Returns the UI font size in pixels.
    fn ui_font_size(&self, cx: &App) -> Pixels;

    /// Returns the buffer font size in pixels.
    fn buffer_font_size(&self, cx: &App) -> Pixels;

    // Returns the current UI density setting.
    // fn ui_density(&self, cx: &App) -> UiDensity;
}

struct GlobalThemeSettingsProvider(Box<dyn ThemeSettingsProvider>);

impl Global for GlobalThemeSettingsProvider {}

/// Registers the global [`ThemeSettingsProvider`] implementation.
///
/// This should be called during application initialization by the crate
/// that owns the concrete theme settings (e.g. `theme_settings`).
pub fn set_theme_settings_provider(provider: Box<dyn ThemeSettingsProvider>, cx: &mut App) {
    cx.set_global(GlobalThemeSettingsProvider(provider));
}

/// Returns the global [`ThemeSettingsProvider`].
///
/// Panics if no provider has been registered via [`set_theme_settings_provider`].
pub fn theme_settings(cx: &App) -> &dyn ThemeSettingsProvider {
    &*cx.global::<GlobalThemeSettingsProvider>().0
}

struct ThemeSettingsProviderImpl;

impl ThemeSettingsProvider for ThemeSettingsProviderImpl {
    fn ui_font<'a>(&'a self, cx: &'a App) -> &'a Font {
        &ThemeSettings::get_global(cx).ui_font
    }

    fn buffer_font<'a>(&'a self, cx: &'a App) -> &'a Font {
        &ThemeSettings::get_global(cx).buffer_font
    }

    fn ui_font_size(&self, cx: &App) -> Pixels {
        ThemeSettings::get_global(cx).ui_font_size(cx)
    }

    fn buffer_font_size(&self, cx: &App) -> Pixels {
        ThemeSettings::get_global(cx).buffer_font_size(cx)
    }
}
/// Initialize the theme system with settings integration.
///
/// This is the full initialization for the application. It calls [`theme::init`]
/// and then wires up settings observation for theme/font changes.
pub fn init(themes_to_load: LoadThemes, cx: &mut App) {
    let load_user_themes = matches!(&themes_to_load, LoadThemes::All(_));

    theme::init(themes_to_load, cx);
    set_theme_settings_provider(Box::new(ThemeSettingsProviderImpl), cx);

    if load_user_themes {
        let registry = ThemeRegistry::global(cx);
        load_bundled_themes(&registry);
    }

    let theme = configured_theme(cx);
    // let icon_theme = configured_icon_theme(cx);
    GlobalTheme::update_theme(cx, theme);
    // GlobalTheme::update_icon_theme(cx, icon_theme);

    let settings = ThemeSettings::get_global(cx);
    info!("ui fonts {:#?}", settings);
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
/// 从二进制文件中加载内置主题
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

    let mut refined_status_colors = match theme.appearance {
        AppearanceContent::Light => StatusColors::light(),
        AppearanceContent::Dark => StatusColors::dark(),
    };
    let mut status_colors_refinement = status_colors_refinement(&theme.style.status);
    apply_status_color_defaults(&mut status_colors_refinement);
    refined_status_colors.refine(&status_colors_refinement);

    let mut refined_theme_colors = match theme.appearance {
        AppearanceContent::Light => ThemeColors::light(),
        AppearanceContent::Dark => ThemeColors::dark(),
    };
    let mut theme_colors_refinement = theme_colors_refinement(
        &theme.style.colors,
        &status_colors_refinement,
        theme.appearance == AppearanceContent::Light,
    );
    // theme::apply_theme_color_defaults(&mut theme_colors_refinement, &refined_player_colors);
    refined_theme_colors.refine(&theme_colors_refinement);

    let mut refined_accent_colors = match theme.appearance {
        AppearanceContent::Light => AccentColors::light(),
        AppearanceContent::Dark => AccentColors::dark(),
    };
    merge_accent_colors(&mut refined_accent_colors, &theme.style.accents);

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
            accents: refined_accent_colors,
            colors: refined_theme_colors,
            status: refined_status_colors,
            // player: refined_player_colors,
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
    // theme
    theme_settings.apply_theme_overrides(theme)
}

#[derive(Clone, PartialEq, RegisterSetting, Debug)]
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

/// In-memory override for the markdown preview font size.
#[derive(Default)]
pub struct MarkdownPreviewFontSize(Pixels);

impl Global for MarkdownPreviewFontSize {}

mod test {

    use std::fs;

    use settings_content::theme::{
        AccentContent, PlayerColorContent, StatusColorsContent, ThemeColorsContent,
        ThemeStyleContent,
    };

    use crate::schema::{ThemeContent, ThemeFamilyContent};

    fn build_theme(name: &str, author: &str, themes: Vec<ThemeContent>) -> ThemeFamilyContent {
        let theme_family = ThemeFamilyContent {
            name: name.into(),
            author: author.into(),
            themes,
        };
        return theme_family;
    }
    #[test]
    fn foo() {
        let dark = ThemeContent {
            name: "dark".into(),
            appearance: theme::AppearanceContent::Dark,
            style: ThemeStyleContent {
                window_background_appearance: Some(
                    settings_content::theme::WindowBackgroundContent::Transparent,
                ),
                accents: vec![AccentContent(Some(
                    settings_content::theme::ThemeColor::from("#0a0a0a"),
                ))],
                colors: ThemeColorsContent {
                    // ============ 背景和表面颜色 ============
                    background: Some(settings_content::theme::ThemeColor::from("#0a0a0a")),
                    surface_background: Some(settings_content::theme::ThemeColor::from("#0f1115")),
                    elevated_surface_background: Some(settings_content::theme::ThemeColor::from(
                        "#16181d",
                    )),
                    element_background: Some(settings_content::theme::ThemeColor::from("#1c1f26")),

                    // ============ 文本颜色 ============
                    text: Some(settings_content::theme::ThemeColor::from("#e6e8eb")),
                    text_muted: Some(settings_content::theme::ThemeColor::from("#9aa3af")),
                    // text_subtle: Some(settings_content::theme::ThemeColor::from("#6b7280")),
                    text_placeholder: Some(settings_content::theme::ThemeColor::from("#6b7280")),
                    text_disabled: Some(settings_content::theme::ThemeColor::from("#6b728080")),

                    // ============ 边框颜色 ============
                    border: Some(settings_content::theme::ThemeColor::from("#1f2228")),
                    // border_strong: Some(settings_content::theme::ThemeColor::from("#2a2e36")),
                    border_variant: Some(settings_content::theme::ThemeColor::from("#1f2228")),
                    border_focused: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    border_selected: Some(settings_content::theme::ThemeColor::from("#38bdf8")),
                    border_transparent: Some(settings_content::theme::ThemeColor::from(
                        "#1f222880",
                    )),
                    border_disabled: Some(settings_content::theme::ThemeColor::from("#1f22284d")),

                    // ============ 强调色 ============
                    text_accent: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    icon_accent: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),

                    // ============ 元素颜色 ============
                    element_hover: Some(settings_content::theme::ThemeColor::from("#1c1f26")),
                    element_active: Some(settings_content::theme::ThemeColor::from("#2a2e36")),
                    element_selected: Some(settings_content::theme::ThemeColor::from("#2a2e36")),
                    element_disabled: Some(settings_content::theme::ThemeColor::from("#1c1f2680")),
                    element_selection_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc26",
                    )),

                    // ============ Ghost 元素 ============
                    ghost_element_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc0d",
                    )),
                    ghost_element_hover: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc1a",
                    )),
                    ghost_element_active: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc26",
                    )),
                    ghost_element_selected: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc33",
                    )),
                    ghost_element_disabled: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc0d",
                    )),

                    // ============ 图标颜色 ============
                    icon: Some(settings_content::theme::ThemeColor::from("#e6e8eb")),
                    icon_muted: Some(settings_content::theme::ThemeColor::from("#9aa3af")),
                    icon_disabled: Some(settings_content::theme::ThemeColor::from("#9aa3af4d")),
                    icon_placeholder: Some(settings_content::theme::ThemeColor::from("#6b7280")),

                    // ============ 编辑器颜色 ============
                    editor_background: Some(settings_content::theme::ThemeColor::from("#0a0a0a")),
                    editor_foreground: Some(settings_content::theme::ThemeColor::from("#e6e8eb")),
                    editor_line_number: Some(settings_content::theme::ThemeColor::from("#6b7280")),
                    editor_active_line_number: Some(settings_content::theme::ThemeColor::from(
                        "#e6e8eb",
                    )),
                    editor_hover_line_number: Some(settings_content::theme::ThemeColor::from(
                        "#9aa3af",
                    )),
                    editor_active_line_background: Some(settings_content::theme::ThemeColor::from(
                        "#1c1f26",
                    )),
                    editor_gutter_background: Some(settings_content::theme::ThemeColor::from(
                        "#0f1115",
                    )),
                    editor_subheader_background: Some(settings_content::theme::ThemeColor::from(
                        "#0f1115",
                    )),
                    editor_highlighted_line_background: Some(
                        settings_content::theme::ThemeColor::from("#7dd3fc0d"),
                    ),
                    editor_debugger_active_line_background: Some(
                        settings_content::theme::ThemeColor::from("#7dd3fc14"),
                    ),
                    editor_invisible: Some(settings_content::theme::ThemeColor::from("#1f222880")),

                    // ============ 编辑器缩进和换行引导 ============
                    editor_indent_guide: Some(settings_content::theme::ThemeColor::from("#1f2228")),
                    editor_indent_guide_active: Some(settings_content::theme::ThemeColor::from(
                        "#2a2e36",
                    )),
                    editor_wrap_guide: Some(settings_content::theme::ThemeColor::from("#1f2228")),
                    editor_active_wrap_guide: Some(settings_content::theme::ThemeColor::from(
                        "#2a2e36",
                    )),

                    // ============ 编辑器高亮 ============
                    editor_document_highlight_read_background: Some(
                        settings_content::theme::ThemeColor::from("#7dd3fc1a"),
                    ),
                    editor_document_highlight_write_background: Some(
                        settings_content::theme::ThemeColor::from("#7dd3fc26"),
                    ),
                    editor_document_highlight_bracket_background: Some(
                        settings_content::theme::ThemeColor::from("#7dd3fc1f"),
                    ),

                    // ============ Diff 颜色 ============
                    editor_diff_hunk_added_background: Some(
                        settings_content::theme::ThemeColor::from("#86efac1a"),
                    ),
                    editor_diff_hunk_added_hollow_background: Some(
                        settings_content::theme::ThemeColor::from("#86efac0d"),
                    ),
                    editor_diff_hunk_added_hollow_border: Some(
                        settings_content::theme::ThemeColor::from("#86efac4d"),
                    ),
                    editor_diff_hunk_deleted_background: Some(
                        settings_content::theme::ThemeColor::from("#fca5a51a"),
                    ),
                    editor_diff_hunk_deleted_hollow_background: Some(
                        settings_content::theme::ThemeColor::from("#fca5a50d"),
                    ),
                    editor_diff_hunk_deleted_hollow_border: Some(
                        settings_content::theme::ThemeColor::from("#fca5a54d"),
                    ),

                    // ============ 面板颜色 ============
                    panel_background: Some(settings_content::theme::ThemeColor::from("#0f1115")),
                    panel_focused_border: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc",
                    )),
                    panel_indent_guide: Some(settings_content::theme::ThemeColor::from("#1f2228")),
                    panel_indent_guide_hover: Some(settings_content::theme::ThemeColor::from(
                        "#2a2e36",
                    )),
                    panel_indent_guide_active: Some(settings_content::theme::ThemeColor::from(
                        "#2a2e36",
                    )),
                    panel_overlay_background: Some(settings_content::theme::ThemeColor::from(
                        "#0f1115e6",
                    )),
                    panel_overlay_hover: Some(settings_content::theme::ThemeColor::from(
                        "#1c1f26e6",
                    )),

                    // ============ 窗口/面板边框 ============
                    pane_focused_border: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    pane_group_border: Some(settings_content::theme::ThemeColor::from("#1f2228")),

                    // ============ 状态栏/标题栏/工具栏 ============
                    status_bar_background: Some(settings_content::theme::ThemeColor::from(
                        "#0f1115",
                    )),
                    title_bar_background: Some(settings_content::theme::ThemeColor::from(
                        "#0a0a0a",
                    )),
                    title_bar_inactive_background: Some(settings_content::theme::ThemeColor::from(
                        "#0a0a0a",
                    )),
                    toolbar_background: Some(settings_content::theme::ThemeColor::from("#0f1115")),

                    // ============ 标签栏 ============
                    tab_bar_background: Some(settings_content::theme::ThemeColor::from("#0f1115")),
                    tab_active_background: Some(settings_content::theme::ThemeColor::from(
                        "#1c1f26",
                    )),
                    tab_inactive_background: Some(settings_content::theme::ThemeColor::from(
                        "#0a0a0a",
                    )),

                    // ============ 搜索匹配 ============
                    search_match_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc26",
                    )),
                    search_active_match_background: Some(
                        settings_content::theme::ThemeColor::from("#7dd3fc4d"),
                    ),

                    // ============ 滚动条 ============
                    deprecated_scrollbar_thumb_background: Some(
                        settings_content::theme::ThemeColor::from("#2a2e36"),
                    ),
                    scrollbar_thumb_background: Some(settings_content::theme::ThemeColor::from(
                        "#2a2e36",
                    )),
                    scrollbar_thumb_hover_background: Some(
                        settings_content::theme::ThemeColor::from("#3a3f4a"),
                    ),
                    scrollbar_thumb_active_background: Some(
                        settings_content::theme::ThemeColor::from("#4a4f5a"),
                    ),
                    scrollbar_thumb_border: Some(settings_content::theme::ThemeColor::from(
                        "#1f2228",
                    )),
                    scrollbar_track_background: Some(settings_content::theme::ThemeColor::from(
                        "#0f1115",
                    )),
                    scrollbar_track_border: Some(settings_content::theme::ThemeColor::from(
                        "#1f2228",
                    )),

                    // ============ 小地图 ============
                    minimap_thumb_background: Some(settings_content::theme::ThemeColor::from(
                        "#2a2e3699",
                    )),
                    minimap_thumb_hover_background: Some(
                        settings_content::theme::ThemeColor::from("#2a2e36cc"),
                    ),
                    minimap_thumb_active_background: Some(
                        settings_content::theme::ThemeColor::from("#2a2e36e6"),
                    ),
                    minimap_thumb_border: Some(settings_content::theme::ThemeColor::from(
                        "#1f2228",
                    )),

                    // ============ Drop Target ============
                    drop_target_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc1a",
                    )),
                    drop_target_border: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),

                    // ============ Debugger ============
                    debugger_accent: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),

                    // ============ 链接 ============
                    link_text_hover: Some(settings_content::theme::ThemeColor::from("#38bdf8")),

                    // ============ 版本控制 ============
                    version_control_added: Some(settings_content::theme::ThemeColor::from(
                        "#86efac",
                    )),
                    version_control_deleted: Some(settings_content::theme::ThemeColor::from(
                        "#fca5a5",
                    )),
                    version_control_modified: Some(settings_content::theme::ThemeColor::from(
                        "#fbbf24",
                    )),
                    version_control_renamed: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc",
                    )),
                    version_control_conflict: Some(settings_content::theme::ThemeColor::from(
                        "#c4b5fd",
                    )),
                    version_control_ignored: Some(settings_content::theme::ThemeColor::from(
                        "#6b7280",
                    )),
                    version_control_word_added: Some(settings_content::theme::ThemeColor::from(
                        "#86efac4d",
                    )),
                    version_control_word_deleted: Some(settings_content::theme::ThemeColor::from(
                        "#fca5a54d",
                    )),
                    version_control_conflict_marker_ours: Some(
                        settings_content::theme::ThemeColor::from("#7dd3fc"),
                    ),
                    version_control_conflict_marker_theirs: Some(
                        settings_content::theme::ThemeColor::from("#c4b5fd"),
                    ),

                    // ============ Vim 模式 ============
                    vim_normal_background: Some(settings_content::theme::ThemeColor::from(
                        "#1c1f26",
                    )),
                    vim_insert_background: Some(settings_content::theme::ThemeColor::from(
                        "#0f1115",
                    )),
                    vim_replace_background: Some(settings_content::theme::ThemeColor::from(
                        "#2a2e36",
                    )),
                    vim_visual_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc26",
                    )),
                    vim_visual_line_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc26",
                    )),
                    vim_visual_block_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc1a",
                    )),
                    vim_yank_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc33",
                    )),
                    vim_helix_jump_label_foreground: Some(
                        settings_content::theme::ThemeColor::from("#0a0a0a"),
                    ),
                    vim_helix_normal_background: Some(settings_content::theme::ThemeColor::from(
                        "#1c1f26",
                    )),
                    vim_helix_select_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc",
                    )),
                    vim_normal_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#e6e8eb",
                    )),
                    vim_insert_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#e6e8eb",
                    )),
                    vim_replace_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#e6e8eb",
                    )),
                    vim_visual_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#0a0a0a",
                    )),
                    vim_visual_line_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#0a0a0a",
                    )),
                    vim_visual_block_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#0a0a0a",
                    )),
                    vim_helix_normal_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#e6e8eb",
                    )),
                    vim_helix_select_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#0a0a0a",
                    )),

                    // ============ 终端颜色 ============
                    terminal_background: Some(settings_content::theme::ThemeColor::from("#0c0d10")),
                    terminal_foreground: Some(settings_content::theme::ThemeColor::from("#d4d4d4")),
                    terminal_bright_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#e6e8eb",
                    )),
                    terminal_dim_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#6b7280",
                    )),
                    terminal_ansi_background: Some(settings_content::theme::ThemeColor::from(
                        "#0c0d10",
                    )),

                    // ANSI 标准色
                    terminal_ansi_black: Some(settings_content::theme::ThemeColor::from("#0c0d10")),
                    terminal_ansi_red: Some(settings_content::theme::ThemeColor::from("#fca5a5")),
                    terminal_ansi_green: Some(settings_content::theme::ThemeColor::from("#86efac")),
                    terminal_ansi_yellow: Some(settings_content::theme::ThemeColor::from(
                        "#fbbf24",
                    )),
                    terminal_ansi_blue: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    terminal_ansi_magenta: Some(settings_content::theme::ThemeColor::from(
                        "#c4b5fd",
                    )),
                    terminal_ansi_cyan: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    terminal_ansi_white: Some(settings_content::theme::ThemeColor::from("#d4d4d4")),

                    // ANSI 亮色
                    terminal_ansi_bright_black: Some(settings_content::theme::ThemeColor::from(
                        "#6b7280",
                    )),
                    terminal_ansi_bright_red: Some(settings_content::theme::ThemeColor::from(
                        "#fca5a5",
                    )),
                    terminal_ansi_bright_green: Some(settings_content::theme::ThemeColor::from(
                        "#86efac",
                    )),
                    terminal_ansi_bright_yellow: Some(settings_content::theme::ThemeColor::from(
                        "#fbbf24",
                    )),
                    terminal_ansi_bright_blue: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc",
                    )),
                    terminal_ansi_bright_magenta: Some(settings_content::theme::ThemeColor::from(
                        "#c4b5fd",
                    )),
                    terminal_ansi_bright_cyan: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc",
                    )),
                    terminal_ansi_bright_white: Some(settings_content::theme::ThemeColor::from(
                        "#e6e8eb",
                    )),

                    // ANSI 暗色（半透明）
                    terminal_ansi_dim_black: Some(settings_content::theme::ThemeColor::from(
                        "#0c0d1080",
                    )),
                    terminal_ansi_dim_red: Some(settings_content::theme::ThemeColor::from(
                        "#fca5a580",
                    )),
                    terminal_ansi_dim_green: Some(settings_content::theme::ThemeColor::from(
                        "#86efac80",
                    )),
                    terminal_ansi_dim_yellow: Some(settings_content::theme::ThemeColor::from(
                        "#fbbf2480",
                    )),
                    terminal_ansi_dim_blue: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc80",
                    )),
                    terminal_ansi_dim_magenta: Some(settings_content::theme::ThemeColor::from(
                        "#c4b5fd80",
                    )),
                    terminal_ansi_dim_cyan: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc80",
                    )),
                    terminal_ansi_dim_white: Some(settings_content::theme::ThemeColor::from(
                        "#d4d4d480",
                    )),
                },
                status: StatusColorsContent {
                    // ============ 冲突 ============
                    conflict: Some(settings_content::theme::ThemeColor::from("#c4b5fd")),
                    conflict_background: Some(settings_content::theme::ThemeColor::from(
                        "#c4b5fd1a",
                    )),
                    conflict_border: Some(settings_content::theme::ThemeColor::from("#c4b5fd4d")),

                    // ============ 创建 ============
                    created: Some(settings_content::theme::ThemeColor::from("#86efac")),
                    created_background: Some(settings_content::theme::ThemeColor::from(
                        "#86efac1a",
                    )),
                    created_border: Some(settings_content::theme::ThemeColor::from("#86efac4d")),

                    // ============ 删除 ============
                    deleted: Some(settings_content::theme::ThemeColor::from("#fca5a5")),
                    deleted_background: Some(settings_content::theme::ThemeColor::from(
                        "#fca5a51a",
                    )),
                    deleted_border: Some(settings_content::theme::ThemeColor::from("#fca5a54d")),

                    // ============ 错误 ============
                    error: Some(settings_content::theme::ThemeColor::from("#fca5a5")),
                    error_background: Some(settings_content::theme::ThemeColor::from("#fca5a51a")),
                    error_border: Some(settings_content::theme::ThemeColor::from("#fca5a54d")),

                    // ============ 隐藏 ============
                    hidden: Some(settings_content::theme::ThemeColor::from("#6b7280")),
                    hidden_background: Some(settings_content::theme::ThemeColor::from("#6b72801a")),
                    hidden_border: Some(settings_content::theme::ThemeColor::from("#6b72804d")),

                    // ============ 提示 ============
                    hint: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    hint_background: Some(settings_content::theme::ThemeColor::from("#7dd3fc1a")),
                    hint_border: Some(settings_content::theme::ThemeColor::from("#7dd3fc4d")),

                    // ============ 忽略 ============
                    ignored: Some(settings_content::theme::ThemeColor::from("#6b7280")),
                    ignored_background: Some(settings_content::theme::ThemeColor::from(
                        "#6b72801a",
                    )),
                    ignored_border: Some(settings_content::theme::ThemeColor::from("#6b72804d")),

                    // ============ 信息 ============
                    info: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    info_background: Some(settings_content::theme::ThemeColor::from("#7dd3fc1a")),
                    info_border: Some(settings_content::theme::ThemeColor::from("#7dd3fc4d")),

                    // ============ 修改 ============
                    modified: Some(settings_content::theme::ThemeColor::from("#fbbf24")),
                    modified_background: Some(settings_content::theme::ThemeColor::from(
                        "#fbbf241a",
                    )),
                    modified_border: Some(settings_content::theme::ThemeColor::from("#fbbf244d")),

                    // ============ 预测 ============
                    predictive: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    predictive_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc0d",
                    )),
                    predictive_border: Some(settings_content::theme::ThemeColor::from("#7dd3fc33")),

                    // ============ 重命名 ============
                    renamed: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    renamed_background: Some(settings_content::theme::ThemeColor::from(
                        "#7dd3fc1a",
                    )),
                    renamed_border: Some(settings_content::theme::ThemeColor::from("#7dd3fc4d")),

                    // ============ 成功 ============
                    success: Some(settings_content::theme::ThemeColor::from("#86efac")),
                    success_background: Some(settings_content::theme::ThemeColor::from(
                        "#86efac1a",
                    )),
                    success_border: Some(settings_content::theme::ThemeColor::from("#86efac4d")),

                    // ============ 不可达 ============
                    unreachable: Some(settings_content::theme::ThemeColor::from("#6b7280")),
                    unreachable_background: Some(settings_content::theme::ThemeColor::from(
                        "#6b72801a",
                    )),
                    unreachable_border: Some(settings_content::theme::ThemeColor::from(
                        "#6b72804d",
                    )),

                    // ============ 警告 ============
                    warning: Some(settings_content::theme::ThemeColor::from("#fbbf24")),
                    warning_background: Some(settings_content::theme::ThemeColor::from(
                        "#fbbf241a",
                    )),
                    warning_border: Some(settings_content::theme::ThemeColor::from("#fbbf244d")),
                },
                players: vec![PlayerColorContent {
                    cursor: Some(settings_content::theme::ThemeColor::from("#7dd3fc")),
                    background: Some(settings_content::theme::ThemeColor::from("#7dd3fc1a")),
                    selection: Some(settings_content::theme::ThemeColor::from("#7dd3fc26")),
                }],
                // syntax: IndexMap::new(),
            },
        };
        let light = ThemeContent {
            name: "light".into(),
            appearance: theme::AppearanceContent::Light,
            style: ThemeStyleContent {
                window_background_appearance: Some(
                    settings_content::theme::WindowBackgroundContent::Transparent,
                ),
                accents: vec![AccentContent(Some(
                    settings_content::theme::ThemeColor::from("#0284c7"),
                ))],
                colors: ThemeColorsContent {
                    // ============ 背景和表面颜色 ============
                    background: Some(settings_content::theme::ThemeColor::from("#fafafa")),
                    surface_background: Some(settings_content::theme::ThemeColor::from("#ffffff")),
                    elevated_surface_background: Some(settings_content::theme::ThemeColor::from(
                        "#f4f5f7",
                    )),
                    element_background: Some(settings_content::theme::ThemeColor::from("#eceef2")),

                    // ============ 文本颜色 ============
                    text: Some(settings_content::theme::ThemeColor::from("#111827")),
                    text_muted: Some(settings_content::theme::ThemeColor::from("#4b5563")),
                    // text_subtle: Some(settings_content::theme::ThemeColor::from("#9ca3af")),
                    text_placeholder: Some(settings_content::theme::ThemeColor::from("#9ca3af")),
                    text_disabled: Some(settings_content::theme::ThemeColor::from("#9ca3af80")),

                    // ============ 边框颜色 ============
                    border: Some(settings_content::theme::ThemeColor::from("#e5e7eb")),
                    // border_strong: Some(settings_content::theme::ThemeColor::from("#d1d5db")),
                    border_variant: Some(settings_content::theme::ThemeColor::from("#e5e7eb")),
                    border_focused: Some(settings_content::theme::ThemeColor::from("#0284c7")),
                    border_selected: Some(settings_content::theme::ThemeColor::from("#0369a1")),
                    border_transparent: Some(settings_content::theme::ThemeColor::from(
                        "#e5e7eb80",
                    )),
                    border_disabled: Some(settings_content::theme::ThemeColor::from("#e5e7eb4d")),

                    // ============ 强调色 ============
                    text_accent: Some(settings_content::theme::ThemeColor::from("#0284c7")),
                    icon_accent: Some(settings_content::theme::ThemeColor::from("#0284c7")),

                    // ============ 元素颜色 ============
                    element_hover: Some(settings_content::theme::ThemeColor::from("#eceef2")),
                    element_active: Some(settings_content::theme::ThemeColor::from("#d1d5db")),
                    element_selected: Some(settings_content::theme::ThemeColor::from("#d1d5db")),
                    element_disabled: Some(settings_content::theme::ThemeColor::from("#eceef280")),
                    element_selection_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c714",
                    )),

                    // ============ Ghost 元素 ============
                    ghost_element_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c70d",
                    )),
                    ghost_element_hover: Some(settings_content::theme::ThemeColor::from(
                        "#0284c71a",
                    )),
                    ghost_element_active: Some(settings_content::theme::ThemeColor::from(
                        "#0284c726",
                    )),
                    ghost_element_selected: Some(settings_content::theme::ThemeColor::from(
                        "#0284c733",
                    )),
                    ghost_element_disabled: Some(settings_content::theme::ThemeColor::from(
                        "#0284c70d",
                    )),

                    // ============ 图标颜色 ============
                    icon: Some(settings_content::theme::ThemeColor::from("#111827")),
                    icon_muted: Some(settings_content::theme::ThemeColor::from("#4b5563")),
                    icon_disabled: Some(settings_content::theme::ThemeColor::from("#4b55634d")),
                    icon_placeholder: Some(settings_content::theme::ThemeColor::from("#9ca3af")),

                    // ============ 编辑器颜色 ============
                    editor_background: Some(settings_content::theme::ThemeColor::from("#fafafa")),
                    editor_foreground: Some(settings_content::theme::ThemeColor::from("#111827")),
                    editor_line_number: Some(settings_content::theme::ThemeColor::from("#9ca3af")),
                    editor_active_line_number: Some(settings_content::theme::ThemeColor::from(
                        "#111827",
                    )),
                    editor_hover_line_number: Some(settings_content::theme::ThemeColor::from(
                        "#4b5563",
                    )),
                    editor_active_line_background: Some(settings_content::theme::ThemeColor::from(
                        "#eceef2",
                    )),
                    editor_gutter_background: Some(settings_content::theme::ThemeColor::from(
                        "#f4f5f7",
                    )),
                    editor_subheader_background: Some(settings_content::theme::ThemeColor::from(
                        "#f4f5f7",
                    )),
                    editor_highlighted_line_background: Some(
                        settings_content::theme::ThemeColor::from("#0284c70d"),
                    ),
                    editor_debugger_active_line_background: Some(
                        settings_content::theme::ThemeColor::from("#0284c714"),
                    ),
                    editor_invisible: Some(settings_content::theme::ThemeColor::from("#e5e7eb80")),

                    // ============ 编辑器缩进和换行引导 ============
                    editor_indent_guide: Some(settings_content::theme::ThemeColor::from("#e5e7eb")),
                    editor_indent_guide_active: Some(settings_content::theme::ThemeColor::from(
                        "#d1d5db",
                    )),
                    editor_wrap_guide: Some(settings_content::theme::ThemeColor::from("#e5e7eb")),
                    editor_active_wrap_guide: Some(settings_content::theme::ThemeColor::from(
                        "#d1d5db",
                    )),

                    // ============ 编辑器高亮 ============
                    editor_document_highlight_read_background: Some(
                        settings_content::theme::ThemeColor::from("#0284c71a"),
                    ),
                    editor_document_highlight_write_background: Some(
                        settings_content::theme::ThemeColor::from("#0284c726"),
                    ),
                    editor_document_highlight_bracket_background: Some(
                        settings_content::theme::ThemeColor::from("#0284c71f"),
                    ),

                    // ============ Diff 颜色 ============
                    editor_diff_hunk_added_background: Some(
                        settings_content::theme::ThemeColor::from("#15803d1a"),
                    ),
                    editor_diff_hunk_added_hollow_background: Some(
                        settings_content::theme::ThemeColor::from("#15803d0d"),
                    ),
                    editor_diff_hunk_added_hollow_border: Some(
                        settings_content::theme::ThemeColor::from("#15803d4d"),
                    ),
                    editor_diff_hunk_deleted_background: Some(
                        settings_content::theme::ThemeColor::from("#b91c1c1a"),
                    ),
                    editor_diff_hunk_deleted_hollow_background: Some(
                        settings_content::theme::ThemeColor::from("#b91c1c0d"),
                    ),
                    editor_diff_hunk_deleted_hollow_border: Some(
                        settings_content::theme::ThemeColor::from("#b91c1c4d"),
                    ),

                    // ============ 面板颜色 ============
                    panel_background: Some(settings_content::theme::ThemeColor::from("#f4f5f7")),
                    panel_focused_border: Some(settings_content::theme::ThemeColor::from(
                        "#0284c7",
                    )),
                    panel_indent_guide: Some(settings_content::theme::ThemeColor::from("#e5e7eb")),
                    panel_indent_guide_hover: Some(settings_content::theme::ThemeColor::from(
                        "#d1d5db",
                    )),
                    panel_indent_guide_active: Some(settings_content::theme::ThemeColor::from(
                        "#d1d5db",
                    )),
                    panel_overlay_background: Some(settings_content::theme::ThemeColor::from(
                        "#f4f5f7e6",
                    )),
                    panel_overlay_hover: Some(settings_content::theme::ThemeColor::from(
                        "#eceef2e6",
                    )),

                    // ============ 窗口/面板边框 ============
                    pane_focused_border: Some(settings_content::theme::ThemeColor::from("#0284c7")),
                    pane_group_border: Some(settings_content::theme::ThemeColor::from("#e5e7eb")),

                    // ============ 状态栏/标题栏/工具栏 ============
                    status_bar_background: Some(settings_content::theme::ThemeColor::from(
                        "#f4f5f7",
                    )),
                    title_bar_background: Some(settings_content::theme::ThemeColor::from(
                        "#fafafa",
                    )),
                    title_bar_inactive_background: Some(settings_content::theme::ThemeColor::from(
                        "#fafafa",
                    )),
                    toolbar_background: Some(settings_content::theme::ThemeColor::from("#f4f5f7")),

                    // ============ 标签栏 ============
                    tab_bar_background: Some(settings_content::theme::ThemeColor::from("#f4f5f7")),
                    tab_active_background: Some(settings_content::theme::ThemeColor::from(
                        "#ffffff",
                    )),
                    tab_inactive_background: Some(settings_content::theme::ThemeColor::from(
                        "#fafafa",
                    )),

                    // ============ 搜索匹配 ============
                    search_match_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c726",
                    )),
                    search_active_match_background: Some(
                        settings_content::theme::ThemeColor::from("#0284c74d"),
                    ),

                    // ============ 滚动条 ============
                    deprecated_scrollbar_thumb_background: Some(
                        settings_content::theme::ThemeColor::from("#d1d5db"),
                    ),
                    scrollbar_thumb_background: Some(settings_content::theme::ThemeColor::from(
                        "#d1d5db",
                    )),
                    scrollbar_thumb_hover_background: Some(
                        settings_content::theme::ThemeColor::from("#b0b5c0"),
                    ),
                    scrollbar_thumb_active_background: Some(
                        settings_content::theme::ThemeColor::from("#9ca3af"),
                    ),
                    scrollbar_thumb_border: Some(settings_content::theme::ThemeColor::from(
                        "#e5e7eb",
                    )),
                    scrollbar_track_background: Some(settings_content::theme::ThemeColor::from(
                        "#f4f5f7",
                    )),
                    scrollbar_track_border: Some(settings_content::theme::ThemeColor::from(
                        "#e5e7eb",
                    )),

                    // ============ 小地图 ============
                    minimap_thumb_background: Some(settings_content::theme::ThemeColor::from(
                        "#d1d5db99",
                    )),
                    minimap_thumb_hover_background: Some(
                        settings_content::theme::ThemeColor::from("#d1d5dbcc"),
                    ),
                    minimap_thumb_active_background: Some(
                        settings_content::theme::ThemeColor::from("#d1d5dbe6"),
                    ),
                    minimap_thumb_border: Some(settings_content::theme::ThemeColor::from(
                        "#e5e7eb",
                    )),

                    // ============ Drop Target ============
                    drop_target_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c71a",
                    )),
                    drop_target_border: Some(settings_content::theme::ThemeColor::from("#0284c7")),

                    // ============ Debugger ============
                    debugger_accent: Some(settings_content::theme::ThemeColor::from("#0284c7")),

                    // ============ 链接 ============
                    link_text_hover: Some(settings_content::theme::ThemeColor::from("#0369a1")),

                    // ============ 版本控制 ============
                    version_control_added: Some(settings_content::theme::ThemeColor::from(
                        "#15803d",
                    )),
                    version_control_deleted: Some(settings_content::theme::ThemeColor::from(
                        "#b91c1c",
                    )),
                    version_control_modified: Some(settings_content::theme::ThemeColor::from(
                        "#b45309",
                    )),
                    version_control_renamed: Some(settings_content::theme::ThemeColor::from(
                        "#0284c7",
                    )),
                    version_control_conflict: Some(settings_content::theme::ThemeColor::from(
                        "#7e22ce",
                    )),
                    version_control_ignored: Some(settings_content::theme::ThemeColor::from(
                        "#9ca3af",
                    )),
                    version_control_word_added: Some(settings_content::theme::ThemeColor::from(
                        "#15803d4d",
                    )),
                    version_control_word_deleted: Some(settings_content::theme::ThemeColor::from(
                        "#b91c1c4d",
                    )),
                    version_control_conflict_marker_ours: Some(
                        settings_content::theme::ThemeColor::from("#0284c7"),
                    ),
                    version_control_conflict_marker_theirs: Some(
                        settings_content::theme::ThemeColor::from("#7e22ce"),
                    ),

                    // ============ Vim 模式 ============
                    vim_normal_background: Some(settings_content::theme::ThemeColor::from(
                        "#eceef2",
                    )),
                    vim_insert_background: Some(settings_content::theme::ThemeColor::from(
                        "#f4f5f7",
                    )),
                    vim_replace_background: Some(settings_content::theme::ThemeColor::from(
                        "#d1d5db",
                    )),
                    vim_visual_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c726",
                    )),
                    vim_visual_line_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c726",
                    )),
                    vim_visual_block_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c71a",
                    )),
                    vim_yank_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c733",
                    )),
                    vim_helix_jump_label_foreground: Some(
                        settings_content::theme::ThemeColor::from("#fafafa"),
                    ),
                    vim_helix_normal_background: Some(settings_content::theme::ThemeColor::from(
                        "#eceef2",
                    )),
                    vim_helix_select_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c7",
                    )),
                    vim_normal_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#111827",
                    )),
                    vim_insert_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#111827",
                    )),
                    vim_replace_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#111827",
                    )),
                    vim_visual_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#fafafa",
                    )),
                    vim_visual_line_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#fafafa",
                    )),
                    vim_visual_block_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#fafafa",
                    )),
                    vim_helix_normal_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#111827",
                    )),
                    vim_helix_select_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#fafafa",
                    )),

                    // ============ 终端颜色 ============
                    terminal_background: Some(settings_content::theme::ThemeColor::from("#ffffff")),
                    terminal_foreground: Some(settings_content::theme::ThemeColor::from("#1f2937")),
                    terminal_bright_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#111827",
                    )),
                    terminal_dim_foreground: Some(settings_content::theme::ThemeColor::from(
                        "#9ca3af",
                    )),
                    terminal_ansi_background: Some(settings_content::theme::ThemeColor::from(
                        "#ffffff",
                    )),

                    // ANSI 标准色
                    terminal_ansi_black: Some(settings_content::theme::ThemeColor::from("#ffffff")),
                    terminal_ansi_red: Some(settings_content::theme::ThemeColor::from("#b91c1c")),
                    terminal_ansi_green: Some(settings_content::theme::ThemeColor::from("#15803d")),
                    terminal_ansi_yellow: Some(settings_content::theme::ThemeColor::from(
                        "#b45309",
                    )),
                    terminal_ansi_blue: Some(settings_content::theme::ThemeColor::from("#1d4ed8")),
                    terminal_ansi_magenta: Some(settings_content::theme::ThemeColor::from(
                        "#7e22ce",
                    )),
                    terminal_ansi_cyan: Some(settings_content::theme::ThemeColor::from("#1d4ed8")),
                    terminal_ansi_white: Some(settings_content::theme::ThemeColor::from("#1f2937")),

                    // ANSI 亮色
                    terminal_ansi_bright_black: Some(settings_content::theme::ThemeColor::from(
                        "#9ca3af",
                    )),
                    terminal_ansi_bright_red: Some(settings_content::theme::ThemeColor::from(
                        "#b91c1c",
                    )),
                    terminal_ansi_bright_green: Some(settings_content::theme::ThemeColor::from(
                        "#15803d",
                    )),
                    terminal_ansi_bright_yellow: Some(settings_content::theme::ThemeColor::from(
                        "#b45309",
                    )),
                    terminal_ansi_bright_blue: Some(settings_content::theme::ThemeColor::from(
                        "#1d4ed8",
                    )),
                    terminal_ansi_bright_magenta: Some(settings_content::theme::ThemeColor::from(
                        "#7e22ce",
                    )),
                    terminal_ansi_bright_cyan: Some(settings_content::theme::ThemeColor::from(
                        "#1d4ed8",
                    )),
                    terminal_ansi_bright_white: Some(settings_content::theme::ThemeColor::from(
                        "#111827",
                    )),

                    // ANSI 暗色（半透明）
                    terminal_ansi_dim_black: Some(settings_content::theme::ThemeColor::from(
                        "#ffffff80",
                    )),
                    terminal_ansi_dim_red: Some(settings_content::theme::ThemeColor::from(
                        "#b91c1c80",
                    )),
                    terminal_ansi_dim_green: Some(settings_content::theme::ThemeColor::from(
                        "#15803d80",
                    )),
                    terminal_ansi_dim_yellow: Some(settings_content::theme::ThemeColor::from(
                        "#b4530980",
                    )),
                    terminal_ansi_dim_blue: Some(settings_content::theme::ThemeColor::from(
                        "#1d4ed880",
                    )),
                    terminal_ansi_dim_magenta: Some(settings_content::theme::ThemeColor::from(
                        "#7e22ce80",
                    )),
                    terminal_ansi_dim_cyan: Some(settings_content::theme::ThemeColor::from(
                        "#1d4ed880",
                    )),
                    terminal_ansi_dim_white: Some(settings_content::theme::ThemeColor::from(
                        "#1f293780",
                    )),
                },
                status: StatusColorsContent {
                    conflict: Some(settings_content::theme::ThemeColor::from("#7e22ce")),
                    conflict_background: Some(settings_content::theme::ThemeColor::from(
                        "#7e22ce1a",
                    )),
                    conflict_border: Some(settings_content::theme::ThemeColor::from("#7e22ce4d")),
                    created: Some(settings_content::theme::ThemeColor::from("#15803d")),
                    created_background: Some(settings_content::theme::ThemeColor::from(
                        "#15803d1a",
                    )),
                    created_border: Some(settings_content::theme::ThemeColor::from("#15803d4d")),
                    deleted: Some(settings_content::theme::ThemeColor::from("#b91c1c")),
                    deleted_background: Some(settings_content::theme::ThemeColor::from(
                        "#b91c1c1a",
                    )),
                    deleted_border: Some(settings_content::theme::ThemeColor::from("#b91c1c4d")),
                    error: Some(settings_content::theme::ThemeColor::from("#b91c1c")),
                    error_background: Some(settings_content::theme::ThemeColor::from("#b91c1c1a")),
                    error_border: Some(settings_content::theme::ThemeColor::from("#b91c1c4d")),
                    hidden: Some(settings_content::theme::ThemeColor::from("#9ca3af")),
                    hidden_background: Some(settings_content::theme::ThemeColor::from("#9ca3af1a")),
                    hidden_border: Some(settings_content::theme::ThemeColor::from("#9ca3af4d")),
                    hint: Some(settings_content::theme::ThemeColor::from("#0284c7")),
                    hint_background: Some(settings_content::theme::ThemeColor::from("#0284c71a")),
                    hint_border: Some(settings_content::theme::ThemeColor::from("#0284c74d")),
                    ignored: Some(settings_content::theme::ThemeColor::from("#9ca3af")),
                    ignored_background: Some(settings_content::theme::ThemeColor::from(
                        "#9ca3af1a",
                    )),
                    ignored_border: Some(settings_content::theme::ThemeColor::from("#9ca3af4d")),
                    info: Some(settings_content::theme::ThemeColor::from("#0284c7")),
                    info_background: Some(settings_content::theme::ThemeColor::from("#0284c71a")),
                    info_border: Some(settings_content::theme::ThemeColor::from("#0284c74d")),
                    modified: Some(settings_content::theme::ThemeColor::from("#b45309")),
                    modified_background: Some(settings_content::theme::ThemeColor::from(
                        "#b453091a",
                    )),
                    modified_border: Some(settings_content::theme::ThemeColor::from("#b453094d")),
                    predictive: Some(settings_content::theme::ThemeColor::from("#0284c7")),
                    predictive_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c70d",
                    )),
                    predictive_border: Some(settings_content::theme::ThemeColor::from("#0284c733")),
                    renamed: Some(settings_content::theme::ThemeColor::from("#0284c7")),
                    renamed_background: Some(settings_content::theme::ThemeColor::from(
                        "#0284c71a",
                    )),
                    renamed_border: Some(settings_content::theme::ThemeColor::from("#0284c74d")),
                    success: Some(settings_content::theme::ThemeColor::from("#15803d")),
                    success_background: Some(settings_content::theme::ThemeColor::from(
                        "#15803d1a",
                    )),
                    success_border: Some(settings_content::theme::ThemeColor::from("#15803d4d")),
                    unreachable: Some(settings_content::theme::ThemeColor::from("#9ca3af")),
                    unreachable_background: Some(settings_content::theme::ThemeColor::from(
                        "#9ca3af1a",
                    )),
                    unreachable_border: Some(settings_content::theme::ThemeColor::from(
                        "#9ca3af4d",
                    )),
                    warning: Some(settings_content::theme::ThemeColor::from("#b45309")),
                    warning_background: Some(settings_content::theme::ThemeColor::from(
                        "#b453091a",
                    )),
                    warning_border: Some(settings_content::theme::ThemeColor::from("#b453094d")),
                },
                players: vec![PlayerColorContent {
                    cursor: Some(settings_content::theme::ThemeColor::from("#0284c7")),
                    background: Some(settings_content::theme::ThemeColor::from("#0284c71a")),
                    selection: Some(settings_content::theme::ThemeColor::from("#0284c726")),
                }],
                // syntax: IndexMap::new(),
            },
        };
        let theme = build_theme("default", "default", vec![light, dark]);
        let v = serde_json::to_string_pretty(&theme).unwrap();
        fs::write("../../assets/default.json", v).unwrap();
    }
}

pub fn status_colors_refinement(colors: &StatusColorsContent) -> StatusColorsRefinement {
    StatusColorsRefinement {
        conflict: colors
            .conflict
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        conflict_background: colors
            .conflict_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        conflict_border: colors
            .conflict_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        created: colors
            .created
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        created_background: colors
            .created_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        created_border: colors
            .created_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        deleted: colors
            .deleted
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        deleted_background: colors
            .deleted_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        deleted_border: colors
            .deleted_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        error: colors
            .error
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        error_background: colors
            .error_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        error_border: colors
            .error_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        hidden: colors
            .hidden
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        hidden_background: colors
            .hidden_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        hidden_border: colors
            .hidden_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        hint: colors
            .hint
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        hint_background: colors
            .hint_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        hint_border: colors
            .hint_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        ignored: colors
            .ignored
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        ignored_background: colors
            .ignored_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        ignored_border: colors
            .ignored_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        info: colors
            .info
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        info_background: colors
            .info_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        info_border: colors
            .info_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        modified: colors
            .modified
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        modified_background: colors
            .modified_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        modified_border: colors
            .modified_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        predictive: colors
            .predictive
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        predictive_background: colors
            .predictive_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        predictive_border: colors
            .predictive_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        renamed: colors
            .renamed
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        renamed_background: colors
            .renamed_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        renamed_border: colors
            .renamed_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        success: colors
            .success
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        success_background: colors
            .success_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        success_border: colors
            .success_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        unreachable: colors
            .unreachable
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        unreachable_background: colors
            .unreachable_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        unreachable_border: colors
            .unreachable_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        warning: colors
            .warning
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        warning_background: colors
            .warning_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        warning_border: colors
            .warning_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
    }
}

pub fn theme_colors_refinement(
    this: &ThemeColorsContent,
    status_colors: &StatusColorsRefinement,
    is_light: bool,
) -> ThemeColorsRefinement {
    let border = this
        .border
        .as_ref()
        .and_then(|color| try_parse_color(color).ok());
    let editor_document_highlight_read_background = this
        .editor_document_highlight_read_background
        .as_ref()
        .and_then(|color| try_parse_color(color).ok());
    let scrollbar_thumb_background = this
        .scrollbar_thumb_background
        .as_ref()
        .and_then(|color| try_parse_color(color).ok())
        .or_else(|| {
            this.deprecated_scrollbar_thumb_background
                .as_ref()
                .and_then(|color| try_parse_color(color).ok())
        });
    let scrollbar_thumb_hover_background = this
        .scrollbar_thumb_hover_background
        .as_ref()
        .and_then(|color| try_parse_color(color).ok());
    let scrollbar_thumb_active_background = this
        .scrollbar_thumb_active_background
        .as_ref()
        .and_then(|color| try_parse_color(color).ok())
        .or(scrollbar_thumb_background);
    let scrollbar_thumb_border = this
        .scrollbar_thumb_border
        .as_ref()
        .and_then(|color| try_parse_color(color).ok());
    let element_hover = this
        .element_hover
        .as_ref()
        .and_then(|color| try_parse_color(color).ok());
    let panel_background = this
        .panel_background
        .as_ref()
        .and_then(|color| try_parse_color(color).ok());
    let search_match_background = this
        .search_match_background
        .as_ref()
        .and_then(|color| try_parse_color(color).ok());
    let search_active_match_background = this
        .search_active_match_background
        .as_ref()
        .and_then(|color| try_parse_color(color).ok())
        .or(search_match_background);
    let version_control_added = this
        .version_control_added
        .as_ref()
        .and_then(|color| try_parse_color(color).ok())
        .or(status_colors.created);
    let version_control_deleted = this
        .version_control_deleted
        .as_ref()
        .and_then(|color| try_parse_color(color).ok())
        .or(status_colors.deleted);
    let (hunk_fill, hunk_hollow_bg, hunk_hollow_border) = if is_light {
        (
            LIGHT_DIFF_HUNK_FILLED_OPACITY,
            LIGHT_DIFF_HUNK_HOLLOW_BACKGROUND_OPACITY,
            LIGHT_DIFF_HUNK_HOLLOW_BORDER_OPACITY,
        )
    } else {
        (
            DARK_DIFF_HUNK_FILLED_OPACITY,
            DARK_DIFF_HUNK_HOLLOW_BACKGROUND_OPACITY,
            DARK_DIFF_HUNK_HOLLOW_BORDER_OPACITY,
        )
    };
    ThemeColorsRefinement {
        border,
        border_variant: this
            .border_variant
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        border_focused: this
            .border_focused
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        border_selected: this
            .border_selected
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        border_transparent: this
            .border_transparent
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        border_disabled: this
            .border_disabled
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        elevated_surface_background: this
            .elevated_surface_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        surface_background: this
            .surface_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        background: this
            .background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        element_background: this
            .element_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        element_hover,
        element_active: this
            .element_active
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        element_selected: this
            .element_selected
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        element_disabled: this
            .element_disabled
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        element_selection_background: this
            .element_selection_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        drop_target_background: this
            .drop_target_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        drop_target_border: this
            .drop_target_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        ghost_element_background: this
            .ghost_element_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        ghost_element_hover: this
            .ghost_element_hover
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        ghost_element_active: this
            .ghost_element_active
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        ghost_element_selected: this
            .ghost_element_selected
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        ghost_element_disabled: this
            .ghost_element_disabled
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        text: this
            .text
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        text_muted: this
            .text_muted
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        text_placeholder: this
            .text_placeholder
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        text_disabled: this
            .text_disabled
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        text_accent: this
            .text_accent
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        icon: this
            .icon
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        icon_muted: this
            .icon_muted
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        icon_disabled: this
            .icon_disabled
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        icon_placeholder: this
            .icon_placeholder
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        icon_accent: this
            .icon_accent
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        debugger_accent: this
            .debugger_accent
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        status_bar_background: this
            .status_bar_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        title_bar_background: this
            .title_bar_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        title_bar_inactive_background: this
            .title_bar_inactive_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        toolbar_background: this
            .toolbar_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        tab_bar_background: this
            .tab_bar_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        tab_inactive_background: this
            .tab_inactive_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        tab_active_background: this
            .tab_active_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        search_match_background,
        search_active_match_background,
        panel_background,
        panel_focused_border: this
            .panel_focused_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        panel_indent_guide: this
            .panel_indent_guide
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        panel_indent_guide_hover: this
            .panel_indent_guide_hover
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        panel_indent_guide_active: this
            .panel_indent_guide_active
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        panel_overlay_background: this
            .panel_overlay_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(panel_background.map(ensure_opaque)),
        panel_overlay_hover: this
            .panel_overlay_hover
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(panel_background
                .zip(element_hover)
                .map(|(panel_bg, hover_bg)| panel_bg.blend(hover_bg))
                .map(ensure_opaque)),
        pane_focused_border: this
            .pane_focused_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        pane_group_border: this
            .pane_group_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(border),
        scrollbar_thumb_background,
        scrollbar_thumb_hover_background,
        scrollbar_thumb_active_background,
        scrollbar_thumb_border,
        scrollbar_track_background: this
            .scrollbar_track_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        scrollbar_track_border: this
            .scrollbar_track_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        minimap_thumb_background: this
            .minimap_thumb_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(scrollbar_thumb_background.map(ensure_non_opaque)),
        minimap_thumb_hover_background: this
            .minimap_thumb_hover_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(scrollbar_thumb_hover_background.map(ensure_non_opaque)),
        minimap_thumb_active_background: this
            .minimap_thumb_active_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(scrollbar_thumb_active_background.map(ensure_non_opaque)),
        minimap_thumb_border: this
            .minimap_thumb_border
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(scrollbar_thumb_border),

        terminal_background: this
            .terminal_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_background: this
            .terminal_ansi_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_foreground: this
            .terminal_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_bright_foreground: this
            .terminal_bright_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_dim_foreground: this
            .terminal_dim_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_black: this
            .terminal_ansi_black
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_bright_black: this
            .terminal_ansi_bright_black
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_dim_black: this
            .terminal_ansi_dim_black
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_red: this
            .terminal_ansi_red
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_bright_red: this
            .terminal_ansi_bright_red
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_dim_red: this
            .terminal_ansi_dim_red
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_green: this
            .terminal_ansi_green
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_bright_green: this
            .terminal_ansi_bright_green
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_dim_green: this
            .terminal_ansi_dim_green
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_yellow: this
            .terminal_ansi_yellow
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_bright_yellow: this
            .terminal_ansi_bright_yellow
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_dim_yellow: this
            .terminal_ansi_dim_yellow
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_blue: this
            .terminal_ansi_blue
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_bright_blue: this
            .terminal_ansi_bright_blue
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_dim_blue: this
            .terminal_ansi_dim_blue
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_magenta: this
            .terminal_ansi_magenta
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_bright_magenta: this
            .terminal_ansi_bright_magenta
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_dim_magenta: this
            .terminal_ansi_dim_magenta
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_cyan: this
            .terminal_ansi_cyan
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_bright_cyan: this
            .terminal_ansi_bright_cyan
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_dim_cyan: this
            .terminal_ansi_dim_cyan
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_white: this
            .terminal_ansi_white
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_bright_white: this
            .terminal_ansi_bright_white
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        terminal_ansi_dim_white: this
            .terminal_ansi_dim_white
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        link_text_hover: this
            .link_text_hover
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_normal_background: this
            .vim_normal_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_insert_background: this
            .vim_insert_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_replace_background: this
            .vim_replace_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_visual_background: this
            .vim_visual_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_visual_line_background: this
            .vim_visual_line_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_visual_block_background: this
            .vim_visual_block_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_yank_background: this
            .vim_yank_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(editor_document_highlight_read_background),
        vim_helix_jump_label_foreground: this
            .vim_helix_jump_label_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok())
            .or(status_colors.error),
        vim_helix_normal_background: this
            .vim_helix_normal_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_helix_select_background: this
            .vim_helix_select_background
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_normal_foreground: this
            .vim_normal_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_insert_foreground: this
            .vim_insert_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_replace_foreground: this
            .vim_replace_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_visual_foreground: this
            .vim_visual_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_visual_line_foreground: this
            .vim_visual_line_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_visual_block_foreground: this
            .vim_visual_block_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_helix_normal_foreground: this
            .vim_helix_normal_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
        vim_helix_select_foreground: this
            .vim_helix_select_foreground
            .as_ref()
            .and_then(|color| try_parse_color(color).ok()),
    }
}
fn try_parse_color(color: &str) -> anyhow::Result<Hsla> {
    let rgba = gpui::Rgba::try_from(color)?;
    let rgba = palette::rgb::Srgba::from_components((rgba.r, rgba.g, rgba.b, rgba.a));
    let hsla = palette::Hsla::from_color_unclamped(rgba);

    let hsla = gpui::hsla(
        hsla.hue.into_positive_degrees() / 360.,
        hsla.saturation,
        hsla.lightness,
        hsla.alpha,
    );

    Ok(hsla)
}

pub fn merge_accent_colors(accent_colors: &mut AccentColors, user_accent_colors: &[AccentContent]) {
    if user_accent_colors.is_empty() {
        return;
    }

    let colors = user_accent_colors
        .iter()
        .filter_map(|accent_color| {
            accent_color
                .0
                .as_ref()
                .and_then(|color| try_parse_color(color).ok())
        })
        .collect::<Vec<_>>();

    if !colors.is_empty() {
        accent_colors.0 = Arc::from(colors);
    }
}
pub fn apply_status_color_defaults(status: &mut StatusColorsRefinement) {
    for (fg_color, bg_color) in [
        (&status.deleted, &mut status.deleted_background),
        (&status.created, &mut status.created_background),
        (&status.modified, &mut status.modified_background),
        (&status.conflict, &mut status.conflict_background),
        (&status.error, &mut status.error_background),
        (&status.hidden, &mut status.hidden_background),
    ] {
        if bg_color.is_none()
            && let Some(fg_color) = fg_color
        {
            *bg_color = Some(fg_color.opacity(0.25));
        }
    }
}
const LIGHT_DIFF_HUNK_FILLED_OPACITY: f32 = 0.16;
const LIGHT_DIFF_HUNK_HOLLOW_BACKGROUND_OPACITY: f32 = 0.08;
const LIGHT_DIFF_HUNK_HOLLOW_BORDER_OPACITY: f32 = 0.48;
const DARK_DIFF_HUNK_FILLED_OPACITY: f32 = 0.12;
const DARK_DIFF_HUNK_HOLLOW_BACKGROUND_OPACITY: f32 = 0.06;
const DARK_DIFF_HUNK_HOLLOW_BORDER_OPACITY: f32 = 0.36;
fn ensure_opaque(color: Hsla) -> Hsla {
    Hsla { a: 1.0, ..color }
}
fn ensure_non_opaque(color: Hsla) -> Hsla {
    const MAXIMUM_OPACITY: f32 = 0.7;
    if color.a <= MAXIMUM_OPACITY {
        color
    } else {
        Hsla {
            a: MAXIMUM_OPACITY,
            ..color
        }
    }
}
/// Sets the mode for the theme.
pub fn set_mode(content: &mut SettingsContent, mode: ThemeAppearanceMode) {
    let theme = content.theme.as_mut();

    if let Some(selection) = theme.theme.as_mut() {
        match selection {
            settings_content::theme::ThemeSelection::Static(_) => {
                *selection = settings_content::theme::ThemeSelection::Dynamic {
                    mode: ThemeAppearanceMode::System,
                    light: ThemeName(settings_content::theme::DEFAULT_LIGHT_THEME.into()),
                    dark: ThemeName(settings_content::theme::DEFAULT_DARK_THEME.into()),
                };
            }
            settings_content::theme::ThemeSelection::Dynamic {
                mode: mode_to_update,
                ..
            } => *mode_to_update = mode,
        }
    } else {
        theme.theme = Some(settings_content::theme::ThemeSelection::Dynamic {
            mode,
            light: ThemeName(settings_content::theme::DEFAULT_LIGHT_THEME.into()),
            dark: ThemeName(settings_content::theme::DEFAULT_DARK_THEME.into()),
        });
    }
}
