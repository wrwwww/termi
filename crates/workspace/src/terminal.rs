//! `TerminalPane` — the actual terminal viewport.
//!
//! In a real implementation this would be backed by `portable-pty` +
//! `vte::Parser` to translate ANSI escape sequences into styled glyphs.
//! This reference paints a representative static frame.

use std::collections::VecDeque;

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
use terminal::{
    Content, PtyEvent, Terminal, TerminalBackendEvent, TerminalBounds, new_term,
    normalize_terminal_bounds,
};
use terminal_view::TerminalView;
use theme::{ActiveTheme, Theme};
use tokio::task::yield_now;
use vte::ansi::{Processor, StdSyncHandler};
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
