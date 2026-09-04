//! `TerminalPane` — the actual terminal viewport.
//!
//! In a real implementation this would be backed by `portable-pty` +
//! `vte::Parser` to translate ANSI escape sequences into styled glyphs.
//! This reference paints a representative static frame.

use crate::EditAction;
use crate::{session_store::SessionStore, state::AppState, welcome::WelcomePage};
use anyhow::Ok;
use futures::StreamExt;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gpui::{prelude::FluentBuilder, *};
use log::{error, info};
use protocol::{SessionId, SystemEvent, TabId};
use schemars::JsonSchema;
use serde::Deserialize;
use terminal::{TerminalBounds, TerminalBuilder};
use terminal_view::TerminalView;
use theme::{ActiveTheme, Theme};
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = workspace)]
pub struct OpenTerminalAction {
    pub session_id: SessionId,
}
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = workspace)]
pub struct ActivateTerminalAction {
    pub tab_id: TabId,
}
#[derive(Clone, Deserialize, PartialEq, JsonSchema, Action)]
#[action(namespace = workspace)]
pub struct CloseTerminalAction {
    pub tab_id: TabId,
}

pub struct TerminalPane {
    state: Entity<AppState>,
    session_manager: Entity<SessionStore>,
    focus_handle: FocusHandle,
    // 接受从backend返回的事件
    events_rx: Option<UnboundedReceiver<SystemEvent>>,
    events_tx: UnboundedSender<SystemEvent>,
    event_loop_task: Task<Result<(), anyhow::Error>>,
}

impl TerminalPane {
    pub fn open_terminal(
        &mut self,
        action: &OpenTerminalAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab_id = TabId::new();
        let session_id = action.session_id.clone();

        let session = self
            .session_manager
            .read(cx)
            .query(action.session_id)
            .unwrap()
            .clone();
        let title = session.name.clone().into();
        let builder = TerminalBuilder::new_terminal(TerminalBounds::default());

        let terminal = cx.new(|cx| builder.subscribe(cx));

        let terminal_view = cx.new(|cx| TerminalView::new(terminal.clone(), window, cx));
    }
}
