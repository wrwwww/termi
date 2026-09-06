//! Application state — sessions, settings, active session selection.
//!
//! Held inside a single `Entity<AppState>` that views observe and mutate
//! via GPUI's standard model notification API.

use futures::channel::mpsc::UnboundedReceiver;
use gpui::{Context, Entity};
use protocol::{
    AuthMethod, RuntimeEvent, Session, SessionId, SessionStatus, TabId, monitor::MonitorStore,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum::{Display, EnumIter, EnumString};
use utils::collections::HashMap;

use crate::{
    monitor_store::{self},
    runtime_manager::{self, RuntimeManager},
    session_store::{self, SessionStore},
    terminal_store::TerminalStore,
    transfer_store::{self, TransferStore},
};

pub struct AppState {
    pub runtime_manager: RuntimeManager,
    pub session_store: Entity<SessionStore>,
    pub terminal_store: Entity<TerminalStore>,
    pub monitor_store: Entity<MonitorStore>,
    pub transfer_store: Entity<TransferStore>,
    pub groups: Vec<String>,
}
impl AppState {
    pub fn new(
        runtime_manager: RuntimeManager,
        session_store: Entity<SessionStore>,
        terminal_store: Entity<TerminalStore>,
        monitor_store: Entity<MonitorStore>,
        transfer_store: Entity<TransferStore>,
    ) -> Self {
        Self {
            runtime_manager,
            session_store,
            terminal_store,
            monitor_store,
            transfer_store,
            groups: vec![],
        }
    }
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
    pub fn start_event_dispatcher(
        this: Entity<Self>,
        mut rx: UnboundedReceiver<RuntimeEvent>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |_, cx| {
            while let Ok(event) = rx.recv().await {
                this.update(cx, |app, cx| {
                    app.dispatch_event(event, cx);
                });
            }

            anyhow::Ok(())
        })
        .detach();
    }
    fn dispatch_event(&mut self, event: RuntimeEvent, cx: &mut Context<Self>) {
        match event {
            RuntimeEvent::Connected { session_id } => {
                log::info!("RuntimeManager: Connected: {:?}", session_id);
                self.session_store.update(cx, |store, cx| {
                    if let Some(session) = store.query_mut(session_id) {
                        session.status = SessionStatus::Connected;
                    }
                    cx.notify();
                });
            }
            RuntimeEvent::Disconnected => {
                log::info!("RuntimeManager: Disconnected");
            }
            RuntimeEvent::Error { message } => todo!(),
            RuntimeEvent::TerminalOutput { tab_id, bytes } => {
                self.terminal_store.update(cx, |this, cx| {
                    if let Some(terminal) = this.get(&tab_id) {
                        terminal.runtime.update(cx, |this, cx| {
                            this.write_output(&bytes);
                        })
                    }
                });
            }
            RuntimeEvent::TerminalExit { tab_id } => todo!(),
            RuntimeEvent::MetricsUpdated { metrics } => todo!(),
            RuntimeEvent::DirectoryListed { path, entries } => todo!(),
            RuntimeEvent::TransferStarted { transfer_id } => todo!(),
            RuntimeEvent::TransferProgress {
                transfer_id,
                transferred,
                total,
            } => todo!(),
            RuntimeEvent::TransferCompleted { transfer_id } => todo!(),
            RuntimeEvent::TransferFailed {
                transfer_id,
                message,
            } => todo!(),
        }
    }
    fn state_file() -> Option<std::path::PathBuf> {
        std::env::var_os("APPDATA")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(|base| {
                std::path::PathBuf::from(base)
                    .join("termi")
                    .join("state.json")
            })
    }
}
