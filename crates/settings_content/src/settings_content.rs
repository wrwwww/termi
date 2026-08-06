use serde::de::DeserializeOwned;
pub mod serde_helper;
use utils::collections::{HashMap, IndexMap, IndexSet};
pub mod merge_from;
pub mod terminal;
pub mod theme;
use serde::{Deserialize, Serialize};
// use settings_macros::{ with_fallible_options};

/// Defines a settings override struct where each field is
/// `Option<Box<SettingsContent>>`, along with:
/// - `OVERRIDE_KEYS`: a `&[&str]` of the field names (the JSON keys)
/// - `get_by_key(&self, key) -> Option<&SettingsContent>`: accessor by key
///
/// The field list is the single source of truth for the override key strings.
// macro_rules! settings_overrides {
//     (
//         $(#[$attr:meta])*
//         pub struct $name:ident { $($field:ident),* $(,)? }
//     ) => {
//         $(#[$attr])*
//         pub struct $name {
//             $(pub $field: Option<Box<SettingsContent>>,)*
//         }

//         impl $name {
//             /// The JSON override keys, derived from the field names on this struct.
//             pub const OVERRIDE_KEYS: &[&str] = &[$(stringify!($field)),*];

//             /// Look up an override by its JSON key name.
//             pub fn get_by_key(&self, key: &str) -> Option<&SettingsContent> {
//                 match key {
//                     $(stringify!($field) => self.$field.as_deref(),)*
//                     _ => None,
//                 }
//             }
//         }
//     }
// }
use std::collections::{BTreeMap, BTreeSet};
use std::hash::Hash;
use std::sync::Arc;

use crate::terminal::{CursorShape, TerminalSettingsContent};
use crate::theme::ThemeSettingsContent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseStatus {
    /// Settings were parsed successfully
    Success,
    /// Settings file was not changed, so no parsing was performed
    Unchanged,
    /// Settings failed to parse
    Failed { error: String },
}

/// Determines when the mouse cursor should be hidden in response to keyboard
/// input.
///
/// Default: on_typing_and_action
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum HideMouseMode {
    /// Never hide the mouse cursor
    Never,
    /// Hide only when typing
    OnTyping,
    /// Hide on typing and on key bindings that resolve to an action
    #[default]
    OnTypingAndAction,
}

/// Determines whether to reduce non-essential motion in the UI, such as
/// loading spinners and pulsating labels, by rendering them in a static state.
///
/// Default: off
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum ReduceMotionMode {
    /// Always reduce motion
    On,
    /// Never reduce motion
    #[default]
    Off,
}

//#[with_fallible_options]
#[derive(Debug, PartialEq, Default, Clone, Serialize, Deserialize)]
pub struct SettingsContent {
    // #[serde(flatten)]
    // pub project: ProjectSettingsContent,
    #[serde(flatten)]
    pub theme: Box<ThemeSettingsContent>,

    /// Configuration of the terminal in Zed.
    pub terminal: Option<TerminalSettingsContent>,
    // #[serde(flatten)]
    // pub extension: ExtensionSettingsContent,

    // #[serde(flatten)]
    // pub workspace: WorkspaceSettingsContent,

    // #[serde(flatten)]
    // pub editor: EditorSettingsContent,
    // #[serde(flatten)]
    // pub remote: RemoteSettingsContent,
    /// Settings related to the file finder.
    // pub file_finder: Option<FileFinderSettingsContent>,

    // pub git_panel: Option<GitPanelSettingsContent>,

    // pub tabs: Option<ItemSettingsContent>,
    // pub tab_bar: Option<TabBarSettingsContent>,
    // pub status_bar: Option<StatusBarSettingsContent>,

    // pub preview_tabs: Option<PreviewTabsSettingsContent>,

    // pub agent: Option<AgentSettingsContent>,
    // pub agent_servers: Option<AllAgentServersSettings>,
    /// Configuration of audio in Zed.
    // pub audio: Option<AudioSettingsContent>,

    /// Whether or not to automatically check for updates.
    ///
    /// Default: true
    pub auto_update: Option<bool>,

    /// Configuration for the collab panel visual settings.
    pub collaboration_panel: Option<PanelSettingsContent>,
}

impl SettingsContent {
    // pub fn languages_mut(&mut self) -> &mut HashMap<String, LanguageSettingsContent> {
    //     &mut self.project.all_languages.languages.0
    // }
}

// These impls are there to optimize builds by avoiding monomorphization downstream. Yes, they're repetitive, but using default impls
// break the optimization, for whatever reason.
pub trait RootUserSettings: Sized + DeserializeOwned {
    fn parse_json(json: &str) -> (Option<Self>, ParseStatus);
    fn parse_json_with_comments(json: &str) -> anyhow::Result<Self>;
}

// impl RootUserSettings for SettingsContent {
//     fn parse_json(json: &str) -> (Option<Self>, ParseStatus) {
//         fallible_options::parse_json(json)
//     }
//     fn parse_json_with_comments(json: &str) -> anyhow::Result<Self> {
//         parse_json_with_comments(json)
//     }
// }
// // Explicit opt-in instead of blanket impl to avoid monomorphizing downstream. Just a hunch though.
// impl RootUserSettings for Option<SettingsContent> {
//     fn parse_json(json: &str) -> (Option<Self>, ParseStatus) {
//         fallible_options::parse_json(json)
//     }
//     fn parse_json_with_comments(json: &str) -> anyhow::Result<Self> {
//         parse_json_with_comments(json)
//     }
// }
// impl RootUserSettings for UserSettingsContent {
//     fn parse_json(json: &str) -> (Option<Self>, ParseStatus) {
//         fallible_options::parse_json(json)
//     }
//     fn parse_json_with_comments(json: &str) -> anyhow::Result<Self> {
//         parse_json_with_comments(json)
//     }
// }

// settings_overrides! {
//     //#[with_fallible_options]
//     #[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, ]
//     pub struct ReleaseChannelOverrides { dev, nightly, preview, stable }
// }

// settings_overrides! {
//     //#[with_fallible_options]
//     #[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, ]
//     pub struct PlatformOverrides { macos, linux, windows }
// }

/// Determines what settings a profile starts from before applying its overrides.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileBase {
    /// Apply profile settings on top of the user's current settings.
    #[default]
    User,
    /// Apply profile settings on top of Zed's default settings, ignoring user customizations.
    Default,
}

/// A named settings profile that can temporarily override settings.
//#[with_fallible_options]
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
pub struct SettingsProfile {
    /// What base settings to start from before applying this profile's overrides.
    ///
    /// - `user`: Apply on top of user's settings (default)
    /// - `default`: Apply on top of Zed's default settings, ignoring user customizations
    #[serde(default)]
    pub base: ProfileBase,

    /// The settings overrides for this profile.
    #[serde(default)]
    pub settings: Box<SettingsContent>,
}

//#[with_fallible_options]
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
pub struct UserSettingsContent {
    #[serde(flatten)]
    pub content: Box<SettingsContent>,

    // #[serde(flatten)]
    // pub release_channel_overrides: ReleaseChannelOverrides,

    // #[serde(flatten)]
    // pub platform_overrides: PlatformOverrides,
    #[serde(default)]
    pub profiles: IndexMap<String, SettingsProfile>,
}

/// Configuration of audio in Zed.
//#[with_fallible_options]
#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub struct AudioSettingsContent {
    /// Select specific output audio device.
    #[serde(rename = "experimental.output_audio_device")]
    pub output_audio_device: Option<AudioOutputDeviceName>,
    /// Select specific input audio device.
    #[serde(rename = "experimental.input_audio_device")]
    pub input_audio_device: Option<AudioInputDeviceName>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct AudioOutputDeviceName(pub Option<String>);

impl AsRef<Option<String>> for AudioInputDeviceName {
    fn as_ref(&self) -> &Option<String> {
        &self.0
    }
}

impl From<Option<String>> for AudioInputDeviceName {
    fn from(value: Option<String>) -> Self {
        Self(value)
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct AudioInputDeviceName(pub Option<String>);

impl AsRef<Option<String>> for AudioOutputDeviceName {
    fn as_ref(&self) -> &Option<String> {
        &self.0
    }
}

impl From<Option<String>> for AudioOutputDeviceName {
    fn from(value: Option<String>) -> Self {
        Self(value)
    }
}

/// Control what info is collected by Zed.
//#[with_fallible_options]
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct TelemetrySettingsContent {
    /// Send debug info like crash reports.
    ///
    /// Default: true
    pub diagnostics: Option<bool>,
    /// Send anonymized usage data like what languages you're using Zed with.
    ///
    /// Default: true
    pub metrics: Option<bool>,
    /// Allow sending requests to Anthropic models that cannot be offered with
    /// Zero Data Retention.
    ///
    /// Default: false
    pub anthropic_retention: Option<bool>,
}

impl Default for TelemetrySettingsContent {
    fn default() -> Self {
        Self {
            diagnostics: Some(true),
            metrics: Some(true),
            anthropic_retention: Some(false),
        }
    }
}

//#[with_fallible_options]
#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct DebuggerSettingsContent {
    /// Determines the stepping granularity.
    ///
    /// Default: line
    pub stepping_granularity: Option<SteppingGranularity>,
    /// Whether the breakpoints should be reused across Zed sessions.
    ///
    /// Default: true
    pub save_breakpoints: Option<bool>,
    /// Whether to show the debug button in the status bar.
    ///
    /// Default: true
    pub button: Option<bool>,
    /// Time in milliseconds until timeout error when connecting to a TCP debug adapter
    ///
    /// Default: 2000ms
    pub timeout: Option<u64>,
    /// Whether to log messages between active debug adapters and Zed
    ///
    /// Default: true
    pub log_dap_communications: Option<bool>,
    /// Whether to format dap messages in when adding them to debug adapter logger
    ///
    /// Default: true
    pub format_dap_log_messages: Option<bool>,
    /// The dock position of the debug panel
    ///
    /// Default: Bottom
    pub dock: Option<DockPosition>,
}

/// The granularity of one 'step' in the stepping requests `next`, `stepIn`, `stepOut`, and `stepBack`.
#[derive(
    PartialEq,
    Eq,
    Debug,
    Hash,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum SteppingGranularity {
    /// The step should allow the program to run until the current statement has finished executing.
    /// The meaning of a statement is determined by the adapter and it may be considered equivalent to a line.
    /// For example 'for(int i = 0; i < 10; i++)' could be considered to have 3 statements 'int i = 0', 'i < 10', and 'i++'.
    Statement,
    /// The step should allow the program to run until the current source line has executed.
    Line,
    /// The step should allow one instruction to execute (e.g. one x86 instruction).
    Instruction,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum DockPosition {
    Left,
    Bottom,
    Right,
}

/// Configuration of voice calls in Zed.
//#[with_fallible_options]
#[derive(Clone, PartialEq, Default, Serialize, Deserialize, Debug)]
pub struct CallSettingsContent {
    /// Whether the microphone should be muted when joining a channel or a call.
    ///
    /// Default: false
    pub mute_on_join: Option<bool>,

    /// Whether your current project should be shared when joining an empty channel.
    ///
    /// Default: false
    pub share_on_join: Option<bool>,
}

#[derive(
    Default,
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum StatusStyle {
    #[default]
    Icon,
    LabelColor,
}

//#[with_fallible_options]
#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScrollbarSettings {
    // pub show: Option<ShowScrollbar>,
}

//#[with_fallible_options]
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
pub struct PanelSettingsContent {
    /// Whether to show the panel button in the status bar.
    ///
    /// Default: true
    pub button: Option<bool>,
    /// Where to dock the panel.
    ///
    /// Default: right (Agentic layout), left (Classic layout)
    pub dock: Option<DockPosition>,
    /// Default width of the panel in pixels.
    ///
    /// Default: 240
    #[serde(
        serialize_with = "crate::serde_helper::serialize_optional_f32_with_two_decimal_places"
    )]
    pub default_width: Option<f32>,
}

//#[with_fallible_options]
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
pub struct FileFinderSettingsContent {
    /// Whether to show file icons in the file finder.
    ///
    /// Default: true
    pub file_icons: Option<bool>,
    /// Determines how much space the file finder can take up in relation to the available window width.
    ///
    /// Default: small
    pub modal_max_width: Option<FileFinderWidthContent>,
    /// Determines whether the file finder should skip focus for the active file in search results.
    ///
    /// Default: true
    pub skip_focus_for_active_in_search: Option<bool>,
    /// Whether to use gitignored files when searching.
    /// Only the file Zed had indexed will be used, not necessary all the gitignored files.
    ///
    /// Default: Smart
    pub include_ignored: Option<IncludeIgnoredContent>,
    /// Whether to include text channels in file finder results.
    ///
    /// Default: false
    pub include_channels: Option<bool>,
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum IncludeIgnoredContent {
    /// Use all gitignored files
    All,
    /// Use only the files Zed had indexed
    Indexed,
    /// Be smart and search for ignored when called from a gitignored worktree
    #[default]
    Smart,
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "lowercase")]
pub enum FileFinderWidthContent {
    #[default]
    Small,
    Medium,
    Large,
    XLarge,
    Full,
}

//#[with_fallible_options]
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub struct VimSettingsContent {
    pub default_mode: Option<ModeContent>,
    pub toggle_relative_line_numbers: Option<bool>,
    pub use_system_clipboard: Option<UseSystemClipboard>,
    pub use_smartcase_find: Option<bool>,
    pub use_regex_search: Option<bool>,
    /// When enabled, the `:substitute` command replaces all matches in a line
    /// by default. The 'g' flag then toggles this behavior.,
    pub gdefault: Option<bool>,
    pub custom_digraphs: Option<HashMap<String, Arc<str>>>,
    pub highlight_on_yank_duration: Option<u64>,
    pub cursor_shape: Option<CursorShapeSettings>,
    /// When enabled, edit predictions are shown in Vim normal mode.
    /// By default, edit predictions are only shown in insert and replace modes.
    pub show_edit_predictions_in_normal_mode: Option<bool>,
}

#[derive(
    Copy,
    Clone,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Debug,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum ModeContent {
    #[default]
    Normal,
    Insert,
}

/// Controls when to use system clipboard.
#[derive(
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum UseSystemClipboard {
    /// Don't use system clipboard.
    Never,
    /// Use system clipboard.
    Always,
    /// Use system clipboard for yank operations.
    OnYank,
}

/// Cursor shape configuration for insert mode in Vim.
#[derive(
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum VimInsertModeCursorShape {
    /// Inherit cursor shape from the editor's base cursor_shape setting.
    Inherit,
    /// Vertical bar cursor.
    Bar,
    /// Block cursor that surrounds the character.
    Block,
    /// Underline cursor.
    Underline,
    /// Hollow box cursor.
    Hollow,
}

/// The settings for cursor shape.
//#[with_fallible_options]
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CursorShapeSettings {
    /// Cursor shape for the normal mode.
    ///
    /// Default: block
    pub normal: Option<CursorShape>,
    /// Cursor shape for the replace mode.
    ///
    /// Default: underline
    pub replace: Option<CursorShape>,
    /// Cursor shape for the visual mode.
    ///
    /// Default: block
    pub visual: Option<CursorShape>,
    /// Cursor shape for the insert mode.
    ///
    /// The default value follows the primary cursor_shape.
    pub insert: Option<VimInsertModeCursorShape>,
}

/// Settings specific to journaling
//#[with_fallible_options]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JournalSettingsContent {
    /// The path of the directory where journal entries are stored.
    ///
    /// Default: `~`
    pub path: Option<String>,
    /// What format to display the hours in.
    ///
    /// Default: hour12
    pub hour_format: Option<HourFormat>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HourFormat {
    #[default]
    Hour12,
    Hour24,
}

//#[with_fallible_options]
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
pub struct OutlinePanelSettingsContent {
    /// Whether to show the outline panel button in the status bar.
    ///
    /// Default: true
    pub button: Option<bool>,
    /// Customize default width (in pixels) taken by outline panel
    ///
    /// Default: 240
    #[serde(
        serialize_with = "crate::serde_helper::serialize_optional_f32_with_two_decimal_places"
    )]
    pub default_width: Option<f32>,
    /// The position of outline panel
    ///
    /// Default: right (Agentic layout), left (Classic layout)
    pub dock: Option<DockSide>,
    /// Whether to show file icons in the outline panel.
    ///
    /// Default: true
    pub file_icons: Option<bool>,
    /// Whether to show folder icons or chevrons for directories in the outline panel.
    ///
    /// Default: true
    pub folder_icons: Option<bool>,
    /// Whether to show the git status in the outline panel.
    ///
    /// Default: true
    pub git_status: Option<bool>,
    /// Amount of indentation (in pixels) for nested items.
    ///
    /// Default: 20
    #[serde(
        serialize_with = "crate::serde_helper::serialize_optional_f32_with_two_decimal_places"
    )]
    pub indent_size: Option<f32>,
    /// Whether to reveal it in the outline panel automatically,
    /// when a corresponding project entry becomes active.
    /// Gitignored entries are never auto revealed.
    ///
    /// Default: true
    pub auto_reveal_entries: Option<bool>,
    /// Whether to fold directories automatically
    /// when directory has only one directory inside.
    ///
    /// Default: true
    pub auto_fold_dirs: Option<bool>,
    /// Settings related to indent guides in the outline panel.
    pub indent_guides: Option<IndentGuidesSettingsContent>,
    /// Scrollbar-related settings
    // pub scrollbar: Option<ScrollbarSettingsContent>,
    /// Default depth to expand outline items in the current file.
    /// The default depth to which outline entries are expanded on reveal.
    /// - Set to 0 to collapse all items that have children
    /// - Set to 1 or higher to collapse items at that depth or deeper
    ///
    /// Default: 100
    pub expand_outlines_with_depth: Option<usize>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum DockSide {
    Left,
    Right,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum ShowIndentGuides {
    Always,
    Never,
}

//#[with_fallible_options]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IndentGuidesSettingsContent {
    /// When to show the scrollbar in the outline panel.
    pub show: Option<ShowIndentGuides>,
}

#[derive(Clone, Copy, Default, PartialEq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LineIndicatorFormat {
    Short,
    #[default]
    Long,
}

/// The settings for the markdown preview.
//#[with_fallible_options]
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct MarkdownPreviewSettingsContent {
    /// Whether to limit the width of the rendered markdown content. When
    /// enabled, content is constrained to `max_width` and centered
    /// horizontally within the preview pane, for optimal readability.
    ///
    /// Default: true
    pub limit_content_width: Option<bool>,
    /// The maximum width, in pixels, of the rendered markdown content when
    /// `limit_content_width` is enabled.
    ///
    /// Default: 800
    pub max_width: Option<f32>,
}

/// The settings for the image viewer.
//#[with_fallible_options]
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ImageViewerSettingsContent {
    /// The unit to use for displaying image file sizes.
    ///
    /// Default: "binary"
    pub unit: Option<ImageFileSizeUnit>,
}

//#[with_fallible_options]
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    strum::VariantArray,
    strum::VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum ImageFileSizeUnit {
    /// Displays file size in binary units (e.g., KiB, MiB).
    #[default]
    Binary,
    /// Displays file size in decimal units (e.g., KB, MB).
    Decimal,
}

//#[with_fallible_options]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RemoteSettingsContent {
    pub ssh_connections: Option<Vec<SshConnection>>,
    pub wsl_connections: Option<Vec<WslConnection>>,
    pub dev_container_connections: Option<Vec<DevContainerConnection>>,
    pub read_ssh_config: Option<bool>,
    pub use_podman: Option<bool>,
    /// Whether to build dev container images with BuildKit.
    ///
    /// When unset, Zed auto-detects BuildKit by probing for the `buildx` CLI
    /// plugin. Set to `false` to force the classic Docker builder, which is
    /// required for Docker-compatible engines that lack an integrated BuildKit
    /// (e.g. Apple Container via a Docker-API bridge), where BuildKit builds
    /// cannot resolve locally-built images.
    ///
    /// Default: null (auto-detect)
    pub dev_container_use_buildkit: Option<bool>,
}

//#[with_fallible_options]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DevContainerConnection {
    pub name: String,
    pub remote_user: String,
    pub container_id: String,
    pub use_podman: bool,
    pub extension_ids: Vec<String>,
    pub remote_env: BTreeMap<String, String>,
}

//#[with_fallible_options]
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SshConnection {
    pub host: String,
    pub username: Option<String>,
    pub port: Option<u16>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub projects: utils::collections::BTreeSet<RemoteProject>,
    /// Name to use for this server in UI.
    pub nickname: Option<String>,
    // By default Zed will download the binary to the host directly.
    // If this is set to true, Zed will download the binary to your local machine,
    // and then upload it over the SSH connection. Useful if your SSH server has
    // limited outbound internet access.
    pub upload_binary_over_ssh: Option<bool>,

    pub port_forwards: Option<Vec<SshPortForwardOption>>,
    /// Timeout in seconds for SSH connection and downloading the remote server binary.
    /// Defaults to 10 seconds if not specified.
    pub connection_timeout: Option<u16>,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
pub struct WslConnection {
    pub distro_name: String,
    pub user: Option<String>,
    #[serde(default)]
    pub projects: BTreeSet<RemoteProject>,
}

//#[with_fallible_options]
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct RemoteProject {
    pub paths: Vec<String>,
}

//#[with_fallible_options]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SshPortForwardOption {
    pub local_host: Option<String>,
    pub local_port: u16,
    pub remote_host: Option<String>,
    pub remote_port: u16,
}

/// Settings for configuring REPL display and behavior.
//#[with_fallible_options]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplSettingsContent {
    /// Maximum number of lines to keep in REPL's scrollback buffer.
    /// Clamped with [4, 256] range.
    ///
    /// Default: 32
    pub max_lines: Option<usize>,
    /// Maximum number of columns to keep in REPL's scrollback buffer.
    /// Clamped with [20, 512] range.
    ///
    /// Default: 128
    pub max_columns: Option<usize>,
    /// Whether to show small single-line outputs inline instead of in a block.
    ///
    /// Default: true
    pub inline_output: Option<bool>,
    /// Maximum number of characters for an output to be shown inline.
    /// Only applies when `inline_output` is true.
    ///
    /// Default: 50
    pub inline_output_max_length: Option<usize>,
    /// Maximum number of lines of output to display before scrolling.
    /// Set to 0 to disable output height limits.
    ///
    /// Default: 0
    pub output_max_height_lines: Option<usize>,
}

/// Settings for configuring the which-key popup behaviour.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WhichKeySettingsContent {
    /// Whether to show the which-key popup when holding down key combinations
    ///
    /// Default: false
    pub enabled: Option<bool>,
    /// Delay in milliseconds before showing the which-key popup.
    ///
    /// Default: 700
    pub delay_ms: Option<u64>,
}

// An ExtendingVec in the settings can only accumulate new values.
//
// This is useful for things like private files where you only want
// to allow new values to be added.
//
// Consider using a HashMap<String, bool> instead of this type
// (like auto_install_extensions) so that user settings files can both add
// and remove values from the set.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtendingVec<T>(pub Vec<T>);

impl<T> Into<Vec<T>> for ExtendingVec<T> {
    fn into(self) -> Vec<T> {
        self.0
    }
}
impl<T> From<Vec<T>> for ExtendingVec<T> {
    fn from(vec: Vec<T>) -> Self {
        ExtendingVec(vec)
    }
}

impl<T: Clone> crate::merge_from::MergeFrom for ExtendingVec<T> {
    fn merge_from(&mut self, other: &Self) {
        self.0.extend_from_slice(other.0.as_slice());
    }
}

// An ExtendingSet in the settings can only accumulate new values, and ignores
// values that are already present, so merging the same source more than once
// (e.g. re-importing VS Code settings) is idempotent.
//
// Insertion order is preserved, so it round-trips through the user's settings
// file without reordering their entries.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtendingSet<T: std::hash::Hash + Eq>(pub IndexSet<T>);

impl<T: std::hash::Hash + Eq> From<Vec<T>> for ExtendingSet<T> {
    fn from(vec: Vec<T>) -> Self {
        ExtendingSet(vec.into_iter().collect())
    }
}

impl<T: Clone + std::hash::Hash + Eq> crate::merge_from::MergeFrom for ExtendingSet<T> {
    fn merge_from(&mut self, other: &Self) {
        self.0.extend(other.0.iter().cloned());
    }
}

// A SaturatingBool in the settings can only ever be set to true,
// later attempts to set it to false will be ignored.
//
// Used by `disable_ai`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaturatingBool(pub bool);

impl From<bool> for SaturatingBool {
    fn from(value: bool) -> Self {
        SaturatingBool(value)
    }
}

impl From<SaturatingBool> for bool {
    fn from(value: SaturatingBool) -> bool {
        value.0
    }
}

impl merge_from::MergeFrom for SaturatingBool {
    fn merge_from(&mut self, other: &Self) {
        self.0 |= other.0
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DelayMs(pub u64);

impl From<u64> for DelayMs {
    fn from(n: u64) -> Self {
        Self(n)
    }
}

impl std::fmt::Display for DelayMs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ms", self.0)
    }
}
