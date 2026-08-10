//! Design tokens — single source of truth for colours, typography, spacing.
//!
//! Mirrors the values described in `design-spec.md §2`. All UI code MUST
//! read from these constants instead of hard-coding hex/px values.

use gpui::{App, Global, Hsla, Rgba, rgb};

/// A complete theme that the entire app can swap to in one motion.
#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: ThemeMode,
    pub surfaces: Surfaces,
    pub text: TextScale,
    pub border: BorderScale,
    pub accent: AccentScale,
    pub semantic: SemanticScale,
    pub terminal: TerminalPalette,
    pub layout: LayoutScale,
    pub font: FontFamilies,
    pub motion: MotionScale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
    System, // resolved at runtime to Dark or Light
}

#[derive(Clone, Debug)]
pub struct Surfaces {
    pub bg: Hsla,
    pub surface: Hsla,
    pub surface_2: Hsla,
    pub surface_3: Hsla,
}

#[derive(Clone, Debug)]
pub struct TextScale {
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_subtle: Hsla,
}

#[derive(Clone, Debug)]
pub struct BorderScale {
    pub border: Hsla,
    pub border_strong: Hsla,
}

#[derive(Clone, Debug)]
pub struct AccentScale {
    pub accent: Hsla,
    pub accent_strong: Hsla,
    pub accent_soft: Hsla,
}

#[derive(Clone, Debug)]
pub struct SemanticScale {
    pub green: Hsla,
    pub amber: Hsla,
    pub red: Hsla,
    pub purple: Hsla,
}

#[derive(Clone, Debug)]
pub struct TerminalPalette {
    pub bg: Hsla,
    pub text: Hsla,
    pub prompt: Hsla,
    pub path: Hsla,
    pub amber: Hsla,
    pub blue: Hsla,
    pub purple: Hsla,
    pub red: Hsla,
    pub gray: Hsla,
}

#[derive(Clone, Debug)]
pub struct LayoutScale {
    pub header_height: f32,
    pub toolbar_height: f32,
    pub tabs_height: f32,
    pub statusbar_height: f32,
    pub sidebar_width: f32,
    pub rightbar_width: f32,
}

#[derive(Clone, Debug)]
pub struct FontFamilies {
    pub ui: &'static str,
    pub mono: &'static str,
}

#[derive(Clone, Debug)]
pub struct MotionScale {
    pub fast_ms: u64,
    pub default_ms: u64,
    pub slow_ms: u64,
}

impl Theme {
    /// Dark theme (Lumen's identity).
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            surfaces: Surfaces {
                bg: rgb(0x0a0a0a).into(),
                surface: rgb(0x0f1115).into(),
                surface_2: rgb(0x16181d).into(),
                surface_3: rgb(0x1c1f26).into(),
            },
            text: TextScale {
                text: rgb(0xe6e8eb).into(),
                text_muted: rgb(0x9aa3af).into(),
                text_subtle: rgb(0x6b7280).into(),
            },
            border: BorderScale {
                border: rgb(0x1f2228).into(),
                border_strong: rgb(0x2a2e36).into(),
            },
            accent: AccentScale {
                accent: rgb(0x7dd3fc).into(),
                accent_strong: rgb(0x38bdf8).into(),
                accent_soft: rgba(125, 211, 252, 0.10).into(),
            },
            semantic: SemanticScale {
                green: rgb(0x86efac).into(),
                amber: rgb(0xfbbf24).into(),
                red: rgb(0xfca5a5).into(),
                purple: rgb(0xc4b5fd).into(),
            },
            terminal: TerminalPalette {
                bg: rgb(0x0c0d10).into(),
                text: rgb(0xd4d4d4).into(),
                prompt: rgb(0x86efac).into(),
                path: rgb(0x7dd3fc).into(),
                amber: rgb(0xfbbf24).into(),
                blue: rgb(0x7dd3fc).into(),
                purple: rgb(0xc4b5fd).into(),
                red: rgb(0xfca5a5).into(),
                gray: rgb(0x6b7280).into(),
            },
            layout: LayoutScale {
                header_height: 38.0,
                toolbar_height: 44.0,
                tabs_height: 36.0,
                statusbar_height: 28.0,
                sidebar_width: 256.0,
                rightbar_width: 280.0,
            },
            font: FontFamilies {
                ui: "Inter",
                mono: "JetBrains Mono",
            },
            motion: MotionScale {
                fast_ms: 120,
                default_ms: 200,
                slow_ms: 300,
            },
        }
    }

    /// Light theme — same hues, different scale.
    pub fn light() -> Self {
        let mut t = Self::dark();
        t.mode = ThemeMode::Light;
        t.surfaces = Surfaces {
            bg: rgb(0xfafafa).into(),
            surface: rgb(0xffffff).into(),
            surface_2: rgb(0xf4f5f7).into(),
            surface_3: rgb(0xeceef2).into(),
        };
        t.text = TextScale {
            text: rgb(0x111827).into(),
            text_muted: rgb(0x4b5563).into(),
            text_subtle: rgb(0x9ca3af).into(),
        };
        t.border = BorderScale {
            border: rgb(0xe5e7eb).into(),
            border_strong: rgb(0xd1d5db).into(),
        };
        t.accent = AccentScale {
            accent: rgb(0x0284c7).into(),
            accent_strong: rgb(0x0369a1).into(),
            accent_soft: rgba(2, 132, 199, 0.08).into(),
        };
        t.terminal = TerminalPalette {
            bg: rgb(0xffffff).into(),
            text: rgb(0x1f2937).into(),
            prompt: rgb(0x15803d).into(),
            path: rgb(0x1d4ed8).into(),
            amber: rgb(0xb45309).into(),
            blue: rgb(0x1d4ed8).into(),
            purple: rgb(0x7e22ce).into(),
            red: rgb(0xb91c1c).into(),
            gray: rgb(0x9ca3af).into(),
        };
        t
    }
}

/// Global handle to the active theme.
pub struct ActiveTheme(pub Theme);
impl Global for ActiveTheme {}

/// Install the default theme with GPUI.
pub fn install(cx: &mut App) {
    cx.set_global(ActiveTheme(Theme::dark()));
}

/// Helper: derive an `Hsla` with custom alpha from RGB(u8) triple + alpha.
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Hsla {
    Hsla::from(Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    })
}

/// Access the active theme from anywhere.
pub fn active(cx: &App) -> &Theme {
    &cx.global::<ActiveTheme>().0
}
