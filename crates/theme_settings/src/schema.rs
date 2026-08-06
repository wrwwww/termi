use serde::{Deserialize, Serialize};
use settings_content::theme::ThemeStyleContent;
use theme::AppearanceContent;
/// The content of a serialized theme family.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeFamilyContent {
    pub name: String,
    pub author: String,
    pub themes: Vec<ThemeContent>,
}

/// The content of a serialized theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeContent {
    pub name: String,
    pub appearance: AppearanceContent,
    pub style: ThemeStyleContent,
}
