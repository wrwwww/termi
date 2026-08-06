pub mod settings_store;
use anyhow::{Context as _, Result};
use gpui::{App, Font, FontFallbacks, FontStyle, Pixels, px};
use rust_embed::RustEmbed;
use serde::de::DeserializeOwned;
use settings_content::{ParseStatus, SettingsContent};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, btree_map, hash_map},
};
use utils::asset_str;
pub mod content_into_gpui;
use gpui::{AsyncApp, Global, SharedString, UpdateGlobal};

#[doc(hidden)]
pub mod private {
    pub use crate::{RegisteredSetting, SettingValue};
    pub use inventory;
}
use std::{
    any::{Any, TypeId, type_name},
    fmt::Debug,
    path::PathBuf,
    sync::Arc,
};

use crate::settings_store::SettingsStore;

// use crate::setting_content::{ParseStatus, SettingsContent, UserSettingsContent};
#[derive(RustEmbed)]
#[folder = "../../assets"]
#[include = "settings/*"]
#[include = "keymaps/*"]
#[exclude = "*.DS_Store"]
pub struct SettingsAssets;

pub fn init(cx: &mut App) {
    let settings = SettingsStore::new(cx, &default_settings());
    cx.set_global(settings);
    // SettingsStore::observe_active_settings_profile_name(cx).detach();
}

pub fn default_settings() -> Cow<'static, str> {
    asset_str::<SettingsAssets>("settings/default.json")
}
pub trait SettingsKey: 'static + Send + Sync {
    /// The name of a key within the JSON file from which this setting should
    /// be deserialized. If this is `None`, then the setting will be deserialized
    /// from the root object.
    const KEY: Option<&'static str>;

    const FALLBACK_KEY: Option<&'static str> = None;
}

/// A value that can be defined as a user setting.
///
/// Settings can be loaded from a combination of multiple JSON files.
pub trait Settings: 'static + Send + Sync + Sized {
    /// The name of the keys in the [`SettingsContent`] that should
    /// always be written to a settings file, even if their value matches the default
    /// value.
    ///
    /// This is useful for tagged [`SettingsContent`]s where the tag
    /// is a "version" field that should always be persisted, even if the current
    /// user settings match the current version of the settings.
    const PRESERVED_KEYS: Option<&'static [&'static str]> = None;

    /// Read the value from default.json.
    ///
    /// This function *should* panic if default values are missing,
    /// and you should add a default to default.json for documentation.
    fn from_settings(content: &SettingsContent) -> Self;

    #[track_caller]
    fn register(cx: &mut App)
    where
        Self: Sized,
    {
        SettingsStore::update_global(cx, |store, _| {
            store.register_setting::<Self>();
        });
    }

    #[track_caller]
    fn get<'a>(path: Option<String>, cx: &'a App) -> &'a Self
    where
        Self: Sized,
    {
        cx.global::<SettingsStore>().get(path)
    }

    #[track_caller]
    fn get_global(cx: &App) -> &Self
    where
        Self: Sized,
    {
        cx.global::<SettingsStore>().get(None)
    }

    #[track_caller]
    fn try_get(cx: &App) -> Option<&Self>
    where
        Self: Sized,
    {
        if cx.has_global::<SettingsStore>() {
            cx.global::<SettingsStore>().try_get(None)
        } else {
            None
        }
    }

    #[track_caller]
    fn try_read_global<R>(cx: &AsyncApp, f: impl FnOnce(&Self) -> R) -> Option<R>
    where
        Self: Sized,
    {
        cx.try_read_global(|s: &SettingsStore, _| f(s.get(None)))
    }

    #[track_caller]
    fn override_global(settings: Self, cx: &mut App)
    where
        Self: Sized,
    {
        cx.global_mut::<SettingsStore>().override_global(settings)
    }
}

pub struct RegisteredSetting {
    pub settings_value: fn() -> Box<dyn AnySettingValue>,
    pub from_settings: fn(&SettingsContent) -> Box<dyn Any>,
    pub id: fn() -> TypeId,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SettingsFile {
    Default,
    User,
}

impl PartialOrd for SettingsFile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Sorted in order of precedence
impl Ord for SettingsFile {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use SettingsFile::*;
        use std::cmp::Ordering;
        match (self, other) {
            (User, User) => Ordering::Equal,

            (Default, Default) => Ordering::Equal,
            (User, _) => Ordering::Less,
            (_, User) => Ordering::Greater,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LocalSettingsKind {
    Settings,
    Tasks,
    Editorconfig,
    Debug,
}

impl Global for SettingsStore {}

#[doc(hidden)]
#[derive(Debug)]
pub struct SettingValue<T> {
    #[doc(hidden)]
    pub global_value: Option<T>,
}

#[doc(hidden)]
pub trait AnySettingValue: 'static + Send + Sync {
    fn setting_type_name(&self) -> &'static str;

    fn from_settings(&self, s: &SettingsContent) -> Box<dyn Any>;

    // fn value_for_path(&self, path: Option<String>) -> &dyn Any;
    // fn all_local_values(&self) -> Vec<(Arc<String>, &dyn Any)>;
    fn set_global_value(&mut self, value: Box<dyn Any>);
    // fn set_local_value(&mut self, path: Arc<String>, value: Box<dyn Any>);
    // fn clear_local_values(&mut self);
}

/// Parameters that are used when generating some JSON schemas at runtime.
pub struct SettingsJsonSchemaParams<'a> {
    pub language_names: &'a [String],
    pub font_names: &'a [String],
    pub theme_names: &'a [SharedString],
    pub icon_theme_names: &'a [SharedString],
    pub lsp_adapter_names: &'a [String],
    pub action_names: &'a [&'a str],
    pub action_documentation: &'a HashMap<&'a str, &'a str>,
    pub deprecations: &'a HashMap<&'a str, &'a str>,
    pub deprecation_messages: &'a HashMap<&'a str, &'a str>,
}

pub fn parse_json_with_comments<T: DeserializeOwned>(content: &str) -> Result<T> {
    let mut deserializer = serde_json_lenient::Deserializer::from_str(content);
    Ok(T::deserialize(&mut deserializer)?)
}

/// The result of parsing settings, including any migration attempts
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsParseResult {
    /// The result of parsing the settings file (possibly after migration)
    pub parse_status: ParseStatus,
    /// The result of attempting to migrate the settings file
    pub migration_status: MigrationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationStatus {
    /// No migration was needed - settings are up to date
    NotNeeded,
    /// Settings were automatically migrated in memory, but the file needs to be updated
    Succeeded,
    /// Migration was attempted but failed. Original settings were parsed instead.
    Failed { error: String },
}

impl Default for SettingsParseResult {
    fn default() -> Self {
        Self {
            parse_status: ParseStatus::Success,
            migration_status: MigrationStatus::NotNeeded,
        }
    }
}

impl SettingsParseResult {
    pub fn unwrap(self) -> bool {
        self.result().unwrap()
    }

    pub fn expect(self, message: &str) -> bool {
        self.result().expect(message)
    }

    /// Formats the ParseResult as a Result type. This is a lossy conversion
    pub fn result(self) -> Result<bool> {
        let migration_result = match self.migration_status {
            MigrationStatus::NotNeeded => Ok(false),
            MigrationStatus::Succeeded => Ok(true),
            MigrationStatus::Failed { error } => {
                Err(anyhow::format_err!(error)).context("Failed to migrate settings")
            }
        };

        let parse_result = match self.parse_status {
            ParseStatus::Success | ParseStatus::Unchanged => Ok(()),
            ParseStatus::Failed { error } => {
                Err(anyhow::format_err!(error)).context("Failed to parse settings")
            }
        };

        match (migration_result, parse_result) {
            (migration_result @ Ok(_), Ok(())) => migration_result,
            (Err(migration_err), Ok(())) => Err(migration_err),
            (_, Err(parse_err)) => Err(parse_err),
        }
    }

    /// Returns true if there were any errors migrating and parsing the settings content or if migration was required but there were no errors
    pub fn requires_user_action(&self) -> bool {
        matches!(self.parse_status, ParseStatus::Failed { .. })
            || matches!(
                self.migration_status,
                MigrationStatus::Succeeded | MigrationStatus::Failed { .. }
            )
    }

    pub fn ok(self) -> Option<bool> {
        self.result().ok()
    }

    pub fn parse_error(&self) -> Option<String> {
        match &self.parse_status {
            ParseStatus::Failed { error } => Some(error.clone()),
            ParseStatus::Success | ParseStatus::Unchanged => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InvalidSettingsError {
    LocalSettings { path: Arc<String>, message: String },
    UserSettings { message: String },
    ServerSettings { message: String },
    DefaultSettings { message: String },
    Editorconfig { path: String, message: String },
    Tasks { path: PathBuf, message: String },
    Debug { path: PathBuf, message: String },
}

impl std::fmt::Display for InvalidSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidSettingsError::LocalSettings { message, .. }
            | InvalidSettingsError::UserSettings { message }
            | InvalidSettingsError::ServerSettings { message }
            | InvalidSettingsError::DefaultSettings { message }
            | InvalidSettingsError::Tasks { message, .. }
            | InvalidSettingsError::Editorconfig { message, .. }
            | InvalidSettingsError::Debug { message, .. } => write!(f, "{message}"),
        }
    }
}
impl std::error::Error for InvalidSettingsError {}

impl<T: Settings> AnySettingValue for SettingValue<T> {
    fn from_settings(&self, s: &SettingsContent) -> Box<dyn Any> {
        Box::new(T::from_settings(s)) as _
    }

    fn setting_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn set_global_value(&mut self, value: Box<dyn Any>) {
        self.global_value = Some(*value.downcast().unwrap());
    }

    // fn set_local_value(&mut self, root_id: WorktreeId, path: Arc<RelPath>, value: Box<dyn Any>) {
    //     let value = *value.downcast().unwrap();
    //     match self
    //         .local_values
    //         .binary_search_by_key(&(root_id, &path), |e| (e.0, &e.1))
    //     {
    //         Ok(ix) => self.local_values[ix].2 = value,
    //         Err(ix) => self.local_values.insert(ix, (root_id, path, value)),
    //     }
    // }

    // fn clear_local_values(&mut self, root_id: WorktreeId) {
    //     self.local_values
    //         .retain(|(worktree_id, _, _)| *worktree_id != root_id);
    // }
}
