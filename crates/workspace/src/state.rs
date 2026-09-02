//! Application state — sessions, settings, active session selection.
//!
//! Held inside a single `Entity<AppState>` that views observe and mutate
//! via GPUI's standard model notification API.

use protocol::{AuthMethod, Session, SessionId, SessionStatus, TabId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum::{Display, EnumIter, EnumString};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppState {
    pub sessions: Vec<Session>,
    pub groups: Vec<String>,
    /// Session ids that currently have a terminal tab. This is deliberately
    /// runtime-only: reopening the app must not reconnect to hosts silently.
    #[serde(skip)]
    pub open_session_ids: Vec<TabId>,
    #[serde(skip)]
    pub pending_open_session_id: Option<SessionId>,
    pub active_tab_id: Option<TabId>,
    pub active_view: ActiveView,
    pub settings: Settings,
    /// Live server metrics for the currently active session. `None` while
    /// no session is connected or before the first sample arrives.
    pub monitor: Option<MonitorSnapshot>,
    /// Which tab inside the monitor panel is active.
    pub monitor_tab: MonitorTab,
    /// Time window the user has selected for chart history (1m / 5m / 15m).
    pub monitor_window: MonitorWindow,
    /// True when the monitor panel is collapsed (header-only).
    pub monitor_collapsed: bool,
    /// True when live data sampling is paused by the user.
    pub monitor_paused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveView {
    Workspace,
    NewConnection,
    Settings,
}

impl Default for ActiveView {
    fn default() -> Self {
        ActiveView::Workspace
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub theme_mode: ThemeMode,
    pub accent_name: String,
    pub ui_font: String,
    pub mono_font: String,
    pub mono_font_size: f32,
    pub line_height: f32,
    pub cursor_style: CursorStyle,
    pub cursor_blink: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    Block,
    Beam,
    Underline,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::Dark,
            accent_name: "Sky".into(),
            ui_font: "Inter".into(),
            mono_font: "JetBrains Mono".to_string(),
            mono_font_size: 13.0,
            line_height: 1.55,
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Monitor panel
// ---------------------------------------------------------------------------

/// One numeric metric tracked over time (e.g. CPU %, memory %, KB/s).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metric {
    pub current: f32,
    pub unit: String,
    pub threshold_warn: f32,
    pub threshold_danger: f32,
    pub status: MetricStatus,
    /// Rolling window — newest at the end. Length = sample_count.
    pub samples: Vec<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetMetric {
    pub up_kbps: f32,
    pub down_kbps: f32,
    pub interface: String,
    pub status: MetricStatus,
    pub samples_up: Vec<f32>,
    pub samples_down: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricStatus {
    Healthy,
    Warn,
    Danger,
}

impl Metric {
    pub fn severity(&self) -> MetricStatus {
        if self.current >= self.threshold_danger {
            MetricStatus::Danger
        } else if self.current >= self.threshold_warn {
            MetricStatus::Warn
        } else {
            MetricStatus::Healthy
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub cpu: Metric,
    pub memory: Metric,
    pub disk: Metric,
    pub network: NetMetric,
}

impl Default for MonitorSnapshot {
    fn default() -> Self {
        let samples = || vec![30.0; 60];
        Self {
            cpu: Metric {
                current: 42.0,
                unit: "%".into(),
                threshold_warn: 70.0,
                threshold_danger: 90.0,
                status: MetricStatus::Healthy,
                samples: samples(),
            },
            memory: Metric {
                current: 40.0,
                unit: "%".into(),
                threshold_warn: 75.0,
                threshold_danger: 92.0,
                status: MetricStatus::Healthy,
                samples: samples(),
            },
            disk: Metric {
                current: 73.0,
                unit: "%".into(),
                threshold_warn: 80.0,
                threshold_danger: 95.0,
                status: MetricStatus::Warn,
                samples: samples(),
            },
            network: NetMetric {
                up_kbps: 128.0,
                down_kbps: 432.0,
                interface: "eth0".into(),
                status: MetricStatus::Healthy,
                samples_up: vec![128.0; 60],
                samples_down: vec![432.0; 60],
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonitorTab {
    Metrics,
    Logs,
    Tasks,
}

impl Default for MonitorTab {
    fn default() -> Self {
        MonitorTab::Metrics
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonitorWindow {
    OneMin,
    FiveMin,
    FifteenMin,
}

impl Default for MonitorWindow {
    fn default() -> Self {
        MonitorWindow::FiveMin
    }
}

impl AppState {
    fn state_file() -> Option<std::path::PathBuf> {
        std::env::var_os("APPDATA")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(|base| {
                std::path::PathBuf::from(base)
                    .join("termi")
                    .join("state.json")
            })
    }

    /// Load sessions + settings from disk; fall back to a demo dataset on first run.
    pub fn load() -> Self {
        // let path = directories::ProjectDirs::from("dev", "lumen", "Lumen")
        //     .map(|d| d.config_dir().join("state.json"));

        if let Some(path) = Self::state_file()
            && let Ok(text) = std::fs::read_to_string(path)
            && let Ok(mut parsed) = serde_json::from_str::<AppState>(&text)
        {
            // Runtime tabs are never restored: reopening an app must not make
            // background connections to remote hosts.
            parsed.open_session_ids.clear();
            parsed.pending_open_session_id = None;
            parsed.active_tab_id = None;
            return parsed;
        }

        // Demo data so the first run isn't empty.
        let mut groups: Vec<String> =
            vec!["Production".into(), "Staging".into(), "Personal".into()];
        groups.sort();
        let sessions = vec![];
        let active_tab_id = None;

        Self {
            sessions,
            groups,
            open_session_ids: Vec::new(),
            pending_open_session_id: None,
            active_tab_id,
            active_view: ActiveView::Workspace,
            settings: Settings::default(),
            monitor: Some(MonitorSnapshot::default()),
            monitor_tab: MonitorTab::Metrics,
            monitor_window: MonitorWindow::FiveMin,
            monitor_collapsed: false,
            monitor_paused: false,
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::state_file() else {
            return;
        };
        let Some(parent) = path.parent() else {
            return;
        };

        // Keep endpoint metadata between launches, but never write secrets in
        // clear text. A password or key passphrase is intentionally requested
        // again after an application restart.
        let mut persisted = self.clone();
        for session in &mut persisted.sessions {
            match &mut session.auth {
                AuthMethod::Password { password } => password.clear(),
                AuthMethod::PublicKey { passphrase, .. } => *passphrase = None,
                AuthMethod::Agent | AuthMethod::KeyboardInteractive => {}
            }
        }

        if std::fs::create_dir_all(parent).is_ok()
            && let Ok(text) = serde_json::to_string_pretty(&persisted)
        {
            let _ = std::fs::write(path, text);
        }
    }

    pub fn set_active_view(&mut self, v: ActiveView) {
        self.active_view = v;
    }

    pub fn set_active_tab(&mut self, id: &TabId) {
        self.active_tab_id = Some(*id);
    }
}
