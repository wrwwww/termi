pub mod colors;
pub mod default_colors;
pub mod fallback_colors;
pub mod font_family_cache;
pub mod registry;
pub mod scale;
use std::sync::Arc;

use gpui::BorrowAppContext;
use gpui::Global;
use gpui::{
    App, AssetSource, Hsla, Pixels, SharedString, Styled, Tiling, WindowAppearance,
    WindowBackgroundAppearance, px,
};
use log::info;
use serde::Deserialize;
use serde::Serialize;

use crate::colors::StatusColors;
use crate::colors::ThemeColors;
use crate::colors::ThemeStyles;
use crate::default_colors::SystemColors;
use crate::font_family_cache::FontFamilyCache;
use crate::registry::ThemeRegistry;
use crate::scale::ColorScales;

/// The name of the default dark theme.
pub const DEFAULT_DARK_THEME: &str = "Zed Default";

/// Defines window border radius for platforms that use client side decorations.
pub const CLIENT_SIDE_DECORATION_ROUNDING: Pixels = px(10.0);
/// Defines window shadow size for platforms that use client side decorations.
pub const CLIENT_SIDE_DECORATION_SHADOW: Pixels = px(10.0);

/// Styling helpers for elements that follow client-side window decorations.
pub trait ClientDecorationsExt: Styled {
    /// Rounds each corner whose two adjacent edges are both untiled.
    fn rounded_client_corners(mut self, tiling: Tiling) -> Self {
        if !tiling.top && !tiling.left {
            self = self.rounded_tl(CLIENT_SIDE_DECORATION_ROUNDING);
        }
        if !tiling.top && !tiling.right {
            self = self.rounded_tr(CLIENT_SIDE_DECORATION_ROUNDING);
        }
        if !tiling.bottom && !tiling.left {
            self = self.rounded_bl(CLIENT_SIDE_DECORATION_ROUNDING);
        }
        if !tiling.bottom && !tiling.right {
            self = self.rounded_br(CLIENT_SIDE_DECORATION_ROUNDING);
        }
        self
    }
}

impl<T: Styled> ClientDecorationsExt for T {}

/// The appearance of the theme.
#[derive(Debug, PartialEq, Clone, Copy, Deserialize)]
pub enum Appearance {
    /// A light appearance.
    Light,
    /// A dark appearance.
    Dark,
}

impl Appearance {
    /// Returns whether the appearance is light.
    pub fn is_light(&self) -> bool {
        match self {
            Self::Light => true,
            Self::Dark => false,
        }
    }
}

impl From<WindowAppearance> for Appearance {
    fn from(value: WindowAppearance) -> Self {
        match value {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
        }
    }
}
/// The appearance of a theme in serialized content.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppearanceContent {
    Light,
    Dark,
}

/// Which themes should be loaded. This is used primarily for testing.
pub enum LoadThemes {
    /// Only load the base theme.
    /// 仅加载基础主题没提供用户更换主题的功能
    /// No user themes will be loaded.
    JustBase,

    /// Load all of the built-in themes.
    /// 加载所有主题
    All(Box<dyn AssetSource>),
}

/// Initialize the theme system with default themes.
///
/// This sets up the [`ThemeRegistry`], [`FontFamilyCache`], [`SystemAppearance`],
/// and [`GlobalTheme`] with the default dark theme. It does NOT load bundled
/// themes from JSON or integrate with settings — use `theme_settings::init` for that.
pub fn init(themes_to_load: LoadThemes, cx: &mut App) {
    SystemAppearance::init(cx);
    let assets = match themes_to_load {
        LoadThemes::JustBase => Box::new(()) as Box<dyn AssetSource>,
        LoadThemes::All(assets) => assets,
    };
    // 初始化全局的主题注册中心
    ThemeRegistry::set_global(assets, cx);
    // 初始化全局字体管理中心
    FontFamilyCache::init_global(cx);

    let themes = ThemeRegistry::default_global(cx);
    // 加载默认主题，并且使用主题中心第一个做兜底
    let theme = themes.get(DEFAULT_DARK_THEME).unwrap_or_else(|_| {
        themes
            .list()
            .into_iter()
            .next()
            .map(|m| themes.get(&m.name).unwrap())
            .unwrap()
    });

    // let icon_theme = themes.default_icon_theme().unwrap();
    cx.set_global(GlobalTheme { theme });
}

/// Implementing this trait allows accessing the active theme.
pub trait ActiveTheme {
    /// Returns the active theme.
    fn theme(&self) -> &Arc<Theme>;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Arc<Theme> {
        GlobalTheme::theme(self)
    }
}

/// The appearance of the system.
#[derive(Debug, Clone, Copy)]
pub struct SystemAppearance(pub Appearance);

impl std::ops::Deref for SystemAppearance {
    type Target = Appearance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for SystemAppearance {
    fn default() -> Self {
        Self(Appearance::Dark)
    }
}

#[derive(Default)]
struct GlobalSystemAppearance(SystemAppearance);

impl std::ops::DerefMut for GlobalSystemAppearance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for GlobalSystemAppearance {
    type Target = SystemAppearance;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Global for GlobalSystemAppearance {}

impl SystemAppearance {
    /// Initializes the [`SystemAppearance`] for the application.
    pub fn init(cx: &mut App) {
        *cx.default_global::<GlobalSystemAppearance>() =
            GlobalSystemAppearance(SystemAppearance(cx.window_appearance().into()));
    }

    /// Returns the global [`SystemAppearance`].
    pub fn global(cx: &App) -> Self {
        cx.global::<GlobalSystemAppearance>().0
    }

    /// Returns a mutable reference to the global [`SystemAppearance`].
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<GlobalSystemAppearance>()
    }
}

/// A theme family is a grouping of themes under a single name.
///
/// For example, the "One" theme family contains the "One Light" and "One Dark" themes.
///
/// It can also be used to package themes with many variants.
///
/// For example, the "Atelier" theme family contains "Cave", "Dune", "Estuary", "Forest", "Heath", etc.
pub struct ThemeFamily {
    /// The unique identifier for the theme family.
    pub id: String,
    /// The name of the theme family. This will be displayed in the UI, such as when adding or removing a theme family.
    pub name: SharedString,
    /// The author of the theme family.
    pub author: SharedString,
    /// The [Theme]s in the family.
    pub themes: Vec<Theme>,
    ///
    pub scales: ColorScales,
}

/// A theme is the primary mechanism for defining the appearance of the UI.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// The unique identifier for the theme.
    pub id: String,
    /// The name of the theme.
    pub name: SharedString,
    /// The appearance of the theme (light or dark).
    pub appearance: Appearance,
    /// The colors and other styles for the theme.
    pub styles: ThemeStyles,
}

impl Theme {
    #[inline(always)]
    ///
    pub fn system(&self) -> &SystemColors {
        &self.styles.system
    }

    /// Returns the [`ThemeColors`] for the theme.
    #[inline(always)]
    pub fn colors(&self) -> &ThemeColors {
        &self.styles.colors
    }

    /// Returns the [`StatusColors`] for the theme.
    #[inline(always)]
    pub fn status(&self) -> &StatusColors {
        &self.styles.status
    }

    /// Returns the [`Appearance`] for the theme.
    #[inline(always)]
    pub fn appearance(&self) -> Appearance {
        self.appearance
    }

    /// Returns the [`WindowBackgroundAppearance`] for the theme.
    #[inline(always)]
    pub fn window_background_appearance(&self) -> WindowBackgroundAppearance {
        self.styles.window_background_appearance
    }

    /// Darkens the color by reducing its lightness.
    /// The resulting lightness is clamped to ensure it doesn't go below 0.0.
    ///
    /// The first value darkens light appearance mode, the second darkens appearance dark mode.
    ///
    /// Note: This is a tentative solution and may be replaced with a more robust color system.
    pub fn darken(&self, color: Hsla, light_amount: f32, dark_amount: f32) -> Hsla {
        let amount = match self.appearance {
            Appearance::Light => light_amount,
            Appearance::Dark => dark_amount,
        };
        let mut hsla = color;
        hsla.l = (hsla.l - amount).max(0.0);
        hsla
    }
}

/// Deserializes an icon theme from the given bytes.
// pub fn deserialize_icon_theme(bytes: &[u8]) -> anyhow::Result<IconThemeFamilyContent> {
//     let icon_theme_family: IconThemeFamilyContent = serde_json_lenient::from_slice(bytes)?;

//     Ok(icon_theme_family)
// }

/// The active theme.
pub struct GlobalTheme {
    theme: Arc<Theme>,
}
impl Global for GlobalTheme {}

impl GlobalTheme {
    /// Creates a new [`GlobalTheme`] with the given theme and icon theme.
    pub fn new(theme: Arc<Theme>) -> Self {
        Self { theme }
    }

    /// Updates the active theme.
    pub fn update_theme(cx: &mut App, theme: Arc<Theme>) {
        cx.update_global::<Self, _>(|this, _| this.theme = theme);
    }

    /// Updates the active icon theme.
    // pub fn update_icon_theme(cx: &mut App, icon_theme: Arc<IconTheme>) {
    //     cx.update_global::<Self, _>(|this, _| this.icon_theme = icon_theme);
    // }

    /// Returns the active theme.
    pub fn theme(cx: &App) -> &Arc<Theme> {
        &cx.global::<Self>().theme
    }

    // pub fn icon_theme(cx: &App) -> &Arc<IconTheme> {
    //     &cx.global::<Self>().icon_theme
    // }
}
