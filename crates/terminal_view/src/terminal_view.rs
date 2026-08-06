use std::ops::Range as StdRange;
// pub mod blink_manager;
pub mod Blink_manager;
pub mod terminal_element;
use gpui::*;
use gpui_rsx::rsx;
use terminal::{CursorShape, Terminal, TerminalBounds, terminal_settings::TerminalSettings};

use crate::{Blink_manager::BlinkManager, terminal_element::TerminalElement};

pub struct ImeState {
    pub marked_text: String,
}
pub struct TerminalView {
    // hostname: Entity<InputState>,
    // port: Entity<InputState>,
    terminal: Entity<Terminal>,
    // lable: Entity<InputState>,
    focus_handle: FocusHandle,
    // hover: Option<HoverTarget>,
    // config: Entity<AppState>,
    pub ime_state: Option<ImeState>,
    pub scroll_top: Pixels,
}
impl TerminalView {
    pub fn new(
        terminal: Entity<Terminal>,
        window: &mut Window,
        cx: &mut Context<Self>,
        // config_manager: Entity<AppState>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        Self {
            // hostname: cx.new(|cx| InputState::new(window, cx)),
            // port: cx.new(|cx| InputState::new(window, cx)),
            // lable: cx.new(|cx| InputState::new(window, cx)),
            scroll_top: Pixels::ZERO,
            // config: config_manager,
            terminal,
            ime_state: None,
            focus_handle,
        }
    }
    pub(crate) fn marked_text_range(&self) -> Option<StdRange<usize>> {
        self.ime_state
            .as_ref()
            .map(|state| 0..state.marked_text.encode_utf16().count())
    }
    pub(crate) fn terminal_bounds(&self, cx: &App) -> TerminalBounds {
        self.terminal.read(cx).last_content().terminal_bounds
    }

    pub(crate) fn set_marked_text(&mut self, marked_text: String, cx: &mut Context<Self>) {
        self.ime_state = (!marked_text.is_empty()).then_some(ImeState { marked_text });
        cx.notify();
    }

    pub(crate) fn clear_marked_text(&mut self, cx: &mut Context<Self>) {
        self.ime_state = None;
        cx.notify();
    }

    pub(crate) fn commit_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.clear_marked_text(cx);
        if !text.is_empty() {
            self.terminal.update(cx, |terminal, cx| {
                terminal.write_input(text.as_bytes().to_vec());
                cx.notify();
            });
        }
    }
    pub fn clear_bell(&mut self, cx: &mut Context<TerminalView>) {
        self.has_bell = false;
        // cx.emit(Event::Wakeup);
    }
    pub fn pause_cursor_blinking(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // self.blink_manager.update(cx, BlinkManager::pause_blinking);
    }

    pub fn terminal(&self) -> &Entity<Terminal> {
        &self.terminal
    }

    /// Attempts to process a keystroke in the terminal. Returns true if handled.
    ///
    /// In vi mode, explicitly triggers a re-render because vi navigation (like j/k)
    /// updates the cursor locally without sending data to the shell, so there's no
    /// shell output to automatically trigger a re-render.
    fn process_keystroke(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        let (handled, vi_mode_enabled) = self.terminal.update(cx, |term, cx| {
            (
                term.try_keystroke(keystroke, TerminalSettings::get_global(cx).option_as_meta),
                term.vi_mode_enabled(),
            )
        });

        if handled && vi_mode_enabled {
            cx.notify();
        }

        handled
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_bell(cx);
        self.pause_cursor_blinking(window, cx);

        if self.process_keystroke(&event.keystroke, cx) {
            cx.stop_propagation();
        }
    }

    fn focus_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.terminal.update(cx, |terminal, _| {
            terminal.set_cursor_shape(self.cursor_shape);
            terminal.focus_in();
        });

        let should_blink = match TerminalSettings::get_global(cx).blinking {
            TerminalBlink::Off => false,
            TerminalBlink::On => true,
            TerminalBlink::TerminalControlled => self.blinking_terminal_enabled,
        };

        if should_blink {
            self.blink_manager.update(cx, BlinkManager::enable);
        }

        window.invalidate_character_coordinates();
        cx.notify();
    }

    fn focus_out(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.blink_manager.update(cx, BlinkManager::disable);
        self.terminal.update(cx, |terminal, _| {
            terminal.focus_out();
            terminal.set_cursor_shape(CursorShape::Hollow);
        });
        cx.notify();
    }
}
impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let terminal_handle = self.terminal.clone();
        let terminal_view_handle = cx.entity();
        let focused = self.focus_handle.is_focused(window);
        // let view = cx.entity();
        // let config = self.config.read(cx).config_manager.current.theme.font_size;
        // let list = self.terminal_manager.read(cx).session_manager.read(cx);

        rsx! {
            <div id="terminal_view" class="bg-black" h_full w_full>
                <div id="terminal_container" class="" h_full w_full>
                    {
                        TerminalElement::new(
                        terminal_handle,
                        terminal_view_handle,
                        self.focus_handle.clone(),
                        focused,
                        true,
                        None,
                        )
                    }
                </div>
            </div>
        }
    }
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
// #[serde(rename_all = "snake_case")]
pub enum TerminalBlink {
    /// Never blink the cursor, ignoring the terminal mode.
    Off,
    /// Default the cursor blink to off, but allow the terminal to
    /// set blinking.
    TerminalControlled,
    /// Always blink the cursor, ignoring the terminal mode.
    On,
}
