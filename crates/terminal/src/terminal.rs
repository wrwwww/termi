pub mod terminal_settings;
use std::{
    cmp,
    collections::{VecDeque, vec_deque},
    mem,
    ops::{BitOr, BitOrAssign, Deref, Range as StdRange},
    rc::Rc,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use alacritty_terminal::{
    Term,
    event::{Event, EventListener},
    grid::{Dimensions, GridIterator},
    index::Column,
    sync::FairMutex,
    term::{Config, cell::Flags},
};

use gpui::{
    AbsoluteLength, AnyElement, App, AvailableSpace, Background, BorderStyle, Bounds, ContentMask,
    Context, Corners, DefiniteLength, DispatchPhase, Edges, Element, Entity, FocusHandle, Font,
    FontFeatures, FontStyle, FontWeight, HighlightStyle, Hitbox, HitboxBehavior, Hsla,
    InputHandler, InteractiveElement, Interactivity, IntoElement, KeyDownEvent, Keystroke,
    Modifiers, ModifiersChangedEvent, MouseButton, MouseMoveEvent, PaintQuad, ParentElement,
    Pixels, Point as GpuiPoint, ShapedLine, Size, StrikethroughStyle, TextAlign, TextRun,
    TextStyle, UTF16Selection, UnderlineStyle, WeakEntity, WhiteSpace, Window, accesskit::Uuid,
    div, fill, font, hsla, point, px, relative, rgba, size,
};
use itertools::Itertools;
use protocol::{BackendTx, SshMessage, SystemEvent};
use serde::{Deserialize, Serialize};
use vte::ansi::{Attr, Color, Handler, NamedColor, Processor, StdSyncHandler};
pub struct Terminal {
    pub id: String,
    pub title: String,
    pub dynamic_title: String,
    // pub kind: TabKind,
    pub status: String,
    pub connected: bool,
    pub last_content: Content,
    pub disconnected_reason: Option<String>,
    /// Incremented each time the tab is reconnected. Used to ignore stale
    /// `BackendEvent::Closed` from the previous backend after a retry.
    pub backend_generation: u32,
    /// Set to `true` when the current backend sends its first `Output` or
    /// `Connected` event. Used to skip stale `Closed` events that arrive
    /// before the new backend has started producing output.
    pub backend_initialized: bool,
    // pub session: Option<Session>,
    output_processor: Processor,
    events: VecDeque<InternalEvent>,
    term: Arc<FairMutex<Term<TerminalListener>>>,

    pub cols: u16,
    pub rows: u16,
    // pub backend: std::sync::Arc<std::sync::Mutex<BackendTx>>,
    pub scroll_pixel_y: f32,
    backend: std::sync::Arc<std::sync::Mutex<BackendTx>>,
    // pub(crate) highlight_cache: std::cell::RefCell<
    //     Option<(
    //         Vec<RenderCell>,
    //         std::collections::HashMap<(i32, i32), gpui::Hsla>,
    //     )>,
    // >,
}
#[derive(Clone)]
enum InternalEvent {
    Resize(TerminalBounds),
    Clear,
    // FocusNextMatch,
    Scroll(Scroll),
    // ScrollToPoint(Point),
    SetSelection(Option<Selection>),
    UpdateSelection(GpuiPoint<Pixels>),
    FindHyperlink(GpuiPoint<Pixels>, bool),
    // ProcessHyperlink(HyperlinkMatch, bool),
    // Whether keep selection when copy
    Copy(Option<bool>),
    // Vi mode events
    ToggleViMode,
    ViMotion(ViMotion),
    // MoveViCursorToPoint(Point),
}
impl Terminal {
    pub fn try_keystroke(&mut self, keystroke: &Keystroke, option_as_meta: bool) -> bool {
        if self.vi_mode_enabled {
            self.vi_motion(keystroke);
            return true;
        }

        // Keep default terminal behavior
        let esc = to_esc_str(keystroke, self.last_content.mode, option_as_meta);
        if let Some(esc) = esc {
            match esc {
                Cow::Borrowed(string) => self.input(string.as_bytes()),
                Cow::Owned(string) => self.input(string.into_bytes()),
            };
            true
        } else {
            false
        }
    }
    // fn process_terminal_event(
    //     &mut self,
    //     event: &InternalEvent,
    //     term: &mut AlacrittyTerm,
    //     window: &mut Window,
    //     cx: &mut Context<Self>,
    // ) {
    //     match event {
    //         &InternalEvent::Resize(new_bounds) => {
    //             let new_bounds = normalize_terminal_bounds(new_bounds);
    //             trace!("Resizing: new_bounds={new_bounds:?}");

    //             self.last_content.terminal_bounds = new_bounds;

    //             if let TerminalType::Pty { pty_tx, .. } = &self.terminal_type {
    //                 pty_tx.resize(new_bounds);
    //             }

    //             resize(term, new_bounds);
    //             // If there are matches we need to emit a wake up event to
    //             // invalidate the matches and recalculate their locations
    //             // in the new terminal layout
    //             if !self.matches.is_empty() {
    //                 cx.emit(Event::Wakeup);
    //             }
    //         }
    //         InternalEvent::Clear => {
    //             trace!("Clearing");
    //             clear_saved_screen(term);
    //             cx.emit(Event::Wakeup);
    //         }
    //         InternalEvent::Scroll(scroll) => {
    //             trace!("Scrolling: scroll={scroll:?}");
    //             scroll_display(term, *scroll);
    //             self.refresh_hovered_word(window);

    //             if self.vi_mode_enabled {
    //                 update_vi_cursor_for_scroll(term, *scroll);
    //                 if let Some(selection_head) = update_selection_to_vi_cursor(term) {
    //                     #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    //                     if let Some(selection_text) = selection_text(term) {
    //                         cx.write_to_primary(ClipboardItem::new_string(selection_text));
    //                     }

    //                     self.selection_head = Some(selection_head);
    //                     cx.emit(Event::SelectionsChanged)
    //                 }
    //             }
    //         }
    //         InternalEvent::SetSelection(selection) => {
    //             trace!("Setting selection: selection={selection:?}");
    //             set_term_selection(term, selection.as_ref());

    //             #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    //             if let Some(selection_text) = selection_text(term) {
    //                 cx.write_to_primary(ClipboardItem::new_string(selection_text));
    //             }

    //             if let Some(selection) = selection {
    //                 self.selection_head = Some(selection.head);
    //             }
    //             cx.emit(Event::SelectionsChanged)
    //         }
    //         InternalEvent::UpdateSelection(position) => {
    //             trace!("Updating selection: position={position:?}");
    //             let (point, side) = grid_point_and_side(
    //                 *position,
    //                 self.last_content.terminal_bounds,
    //                 display_offset(term),
    //             );

    //             if update_term_selection(term, point, side) {
    //                 #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    //                 if let Some(selection_text) = selection_text(term) {
    //                     cx.write_to_primary(ClipboardItem::new_string(selection_text));
    //                 }

    //                 self.selection_head = Some(point);
    //                 cx.emit(Event::SelectionsChanged)
    //             }
    //         }

    //         InternalEvent::Copy(keep_selection) => {
    //             trace!("Copying selection: keep_selection={keep_selection:?}");
    //             if let Some(txt) = selection_text(term) {
    //                 cx.write_to_clipboard(ClipboardItem::new_string(txt));
    //                 if !keep_selection.unwrap_or_else(|| {
    //                     let settings = TerminalSettings::get_global(cx);
    //                     settings.keep_selection_on_copy
    //                 }) {
    //                     self.events.push_back(InternalEvent::SetSelection(None));
    //                 }
    //             }
    //         }
    //         InternalEvent::ScrollToPoint(point) => {
    //             trace!("Scrolling to point: point={point:?}");
    //             scroll_to_point(term, *point);
    //             self.refresh_hovered_word(window);
    //         }
    //         InternalEvent::MoveViCursorToPoint(point) => {
    //             trace!("Move vi cursor to point: point={point:?}");
    //             vi_goto_point(term, *point);
    //             self.refresh_hovered_word(window);
    //         }
    //         InternalEvent::ToggleViMode => {
    //             trace!("Toggling vi mode");
    //             self.vi_mode_enabled = !self.vi_mode_enabled;
    //             toggle_term_vi_mode(term);
    //         }
    //         InternalEvent::ViMotion(motion) => {
    //             trace!("Performing vi motion: motion={motion:?}");
    //             vi_motion(term, *motion);
    //         }
    //         InternalEvent::FindHyperlink(position, open) => {
    //             trace!("Finding hyperlink at position: position={position:?}, open={open:?}");

    //             let point = grid_point(
    //                 *position,
    //                 self.last_content.terminal_bounds,
    //                 display_offset(term),
    //             );

    //             match find_from_terminal_point(
    //                 term,
    //                 point,
    //                 &mut self.hyperlink_regex_searches,
    //                 self.path_style,
    //             ) {
    //                 Some(hyperlink) => {
    //                     self.process_hyperlink(hyperlink, *open, cx);
    //                 }
    //                 None => {
    //                     self.last_content.last_hovered_word = None;
    //                     cx.emit(Event::NewNavigationTarget(None));
    //                 }
    //             }
    //         }
    //         InternalEvent::ProcessHyperlink(hyperlink, open) => {
    //             self.process_hyperlink(hyperlink.clone(), *open, cx);
    //         }
    //     }
    // }
    pub fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let term = self.term.clone();
        let terminal = term.lock();

        //Note that the ordering of events matters for event processing
        while let Some(e) = self.events.pop_front() {
            // self.process_terminal_event(&e, &mut terminal, window, cx)
        }

        self.last_content = make_content(&terminal, &self.last_content);
    }
    pub fn try_modifiers_change(
        &mut self,
        modifiers: &Modifiers,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .last_content
            .terminal_bounds
            .bounds
            .contains(&window.mouse_position())
            && modifiers.secondary()
        {
            // self.refresh_hovered_word(window);
        }
        cx.notify();
    }
    pub fn new(
        id: String,
        title: String,
        // kind: TabKind,
        status: String,
        backend: BackendTx,
        events: std::sync::mpsc::Sender<SystemEvent>,
    ) -> Self {
        let shared_backend = std::sync::Arc::new(std::sync::Mutex::new(backend));
        Self {
            id: id.clone(),
            title: title.clone(),
            dynamic_title: title,
            status,
            connected: false,
            disconnected_reason: None,
            backend_generation: 0,
            backend_initialized: true,
            output_processor: Processor::new(),
            term: Arc::new(FairMutex::new(new_term(
                60,
                30,
                shared_backend.clone(),
                id,
                events,
            ))),
            cols: 100,
            rows: 30,
            scroll_pixel_y: 0.0,
            backend: shared_backend.clone(),
            last_content: Content::default(),
            events: vec_deque::VecDeque::new(),
        }
    }
    pub fn last_content(&self) -> &Content {
        &self.last_content
    }

    // pub fn feed(&mut self, bytes: &[u8]) {
    //     self.output_processor.advance(&mut self.term, bytes);
    // }

    pub fn write_output(&mut self, bytes: &[u8]) {
        // Inject bytes directly into the terminal emulator and refresh the UI.
        // This bypasses the PTY/event loop for display-only terminals.
        let mut previous_byte_was_cr = false;
        let converted = convert_lf_to_crlf(bytes, &mut previous_byte_was_cr);
        log::info!("将后端返回的字符串，写入到term中");
        let mut term = self.term.lock();
        self.output_processor.advance(&mut *term, &converted);
        drop(term);
        // self.detect_init_command_startup_marker();
        // cx.emit(Event::Wakeup);
    }

    pub fn write_input(&mut self, bytes: impl Into<Vec<u8>>) {
        if let Ok(backend) = self.backend.lock() {
            backend.send(SshMessage::Input(bytes.into()));
        }
    }
    // fn detect_init_command_startup_marker(&mut self) {
    //     let Some(marker) = self.init_command_startup_marker.as_deref() else {
    //         return;
    //     };

    //     let has_marker = {
    //         let term = self.term.lock_unfair();
    //         last_non_empty_lines(&term, INIT_COMMAND_STARTUP_MARKER_SEARCH_LINES)
    //             .iter()
    //             .any(|line| line.contains(marker))
    //     };

    //     if has_marker {
    //         self.complete_init_command_startup_handshake();
    //     }
    // }
}
pub type AlacPoint = alacritty_terminal::index::Point;
pub type AlacCell = alacritty_terminal::term::cell::Cell;
fn terminal_point_from_alacritty(point: AlacPoint) -> Point {
    Point {
        line: point.line.0,
        column: point.column.0,
    }
}
fn terminal_cell_from_alacritty(cell: &AlacCell) -> Cell {
    Cell { cell: cell.clone() }
}
fn terminal_cursor_from_alacritty(cursor: alacritty_terminal::term::RenderableCursor) -> Cursor {
    let shape = match cursor.shape {
        vte::ansi::CursorShape::Block => CursorShape::Block,
        vte::ansi::CursorShape::Underline => CursorShape::Underline,
        vte::ansi::CursorShape::Beam => CursorShape::Bar,
        vte::ansi::CursorShape::HollowBlock => CursorShape::HollowBlock,
        vte::ansi::CursorShape::Hidden => CursorShape::Hidden,
    };

    Cursor {
        shape,
        point: Point::new(cursor.point.line.0, cursor.point.column.0),
    }
}
pub fn make_content(term: &Term<TerminalListener>, last_content: &Content) -> Content {
    let content = term.renderable_content();

    let estimated_size = content.display_iter.size_hint().0;
    let mut cells = Vec::with_capacity(estimated_size);

    cells.extend(content.display_iter.map(|ic| IndexedCell {
        point: terminal_point_from_alacritty(ic.point),
        cell: terminal_cell_from_alacritty(ic.cell),
    }));

    let selection_text = if content.selection.is_some() {
        term.selection_to_string()
    } else {
        None
    };

    let bottom_line = term.screen_lines() as i32 - 1 - content.display_offset as i32;
    let bottom_row_occupied = content.cursor.point.line.0 >= bottom_line
        || cells
            .iter()
            .rev()
            .take_while(|cell| cell.point.line >= bottom_line)
            .any(|cell| cell.cell.character() != ' ');

    Content {
        cells,
        // mode: terminal_modes_from_alacritty(content.mode),
        display_offset: content.display_offset,
        // selection_text,
        // selection: content
        //     .selection
        //     .map(terminal_selection_range_from_alacritty),
        cursor: terminal_cursor_from_alacritty(content.cursor),
        cursor_char: term.grid()[content.cursor.point].c,
        terminal_bounds: last_content.terminal_bounds,
        last_hovered_word: last_content.last_hovered_word.clone(),
        scrolled_to_top: content.display_offset == term.history_size(),
        scrolled_to_bottom: content.display_offset == 0,
        bottom_row_occupied,
    }
}

pub fn content_text(term: &Term<TerminalListener>) -> String {
    let start = AlacPoint::new(term.topmost_line(), Column(0));
    let end = AlacPoint::new(term.bottommost_line(), term.last_column());
    term.bounds_to_string(start, end)
}

pub fn total_lines(term: &Term<TerminalListener>) -> usize {
    term.total_lines()
}

pub fn screen_lines(term: &Term<TerminalListener>) -> usize {
    term.screen_lines()
}

// pub(super) fn full_content_range(term: &Term<TerminalListener>) -> Range {
//     let start = AlacPoint::new(term.topmost_line(), Column(0));
//     let end = AlacPoint::new(term.bottommost_line(), term.last_column());
//     Range::from_alacritty(start..=end)
// }

fn convert_lf_to_crlf(bytes: &[u8], previous_byte_was_cr: &mut bool) -> Vec<u8> {
    let mut converted = Vec::with_capacity(bytes.len());
    for &byte in bytes {
        if byte == b'\n' && !*previous_byte_was_cr {
            converted.push(b'\r');
        }
        converted.push(byte);
        *previous_byte_was_cr = byte == b'\r';
    }
    converted
}
#[derive(Clone)]
struct TerminalListener {
    tab_id: String,
    backend: std::sync::Arc<std::sync::Mutex<BackendTx>>,
    events: std::sync::mpsc::Sender<SystemEvent>,
}

impl EventListener for TerminalListener {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(output) => {
                if let Ok(backend) = self.backend.lock() {
                    backend.send(SshMessage::Input(output.into_bytes()));
                }
            }
            Event::TextAreaSizeRequest(format) => {
                let size = alacritty_terminal::event::WindowSize {
                    num_lines: 30,
                    num_cols: 100,
                    cell_width: 8,
                    cell_height: 16,
                };
                if let Ok(backend) = self.backend.lock() {
                    backend.send(SshMessage::Input(format(size).into_bytes()));
                }
            }
            Event::Title(title) => {
                let _ = self.events.send(SystemEvent::TitleUpdate {
                    tab_id: self.tab_id.clone(),
                    title,
                });
            }
            _ => {}
        }
    }
}

fn new_term(
    cols: u16,
    rows: u16,
    backend: std::sync::Arc<std::sync::Mutex<BackendTx>>,
    tab_id: String,
    events: std::sync::mpsc::Sender<SystemEvent>,
) -> Term<TerminalListener> {
    Term::new(
        Config {
            scrolling_history: 2000,
            ..Config::default()
        },
        &TerminalSize::new(cols, rows),
        TerminalListener {
            tab_id,
            backend,
            events,
        },
    )
}
pub struct TerminalSize {
    cols: usize,
    rows: usize,
}

impl TerminalSize {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: cols.max(1) as usize,
            rows: rows.max(1) as usize,
        }
    }
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct BlockContext<'a, 'b> {
    pub window: &'a mut Window,
    pub context: &'b mut App,
    pub dimensions: TerminalBounds,
}
pub struct BlockProperties {
    pub height: u8,
    pub render: Box<dyn Send + Fn(&mut BlockContext) -> AnyElement>,
}
pub const BASE_REM_SIZE_IN_PX: f32 = 16.;

pub fn is_blank(cell: &Cell) -> bool {
    if cell.character() != ' ' {
        return false;
    }

    if !is_default_background_color(cell.background()) {
        return false;
    }

    if cell.hyperlink().is_some() {
        return false;
    }

    if cell.has_visible_style_modifier() {
        return false;
    }

    true
}

/// Helper struct for converting terminal cursor points to displayed cursor points.
#[derive(Copy, Clone)]
pub struct DisplayCursor {
    line: i32,
    col: usize,
}

impl DisplayCursor {
    fn from(cursor_point: Point, display_offset: usize) -> Self {
        Self {
            line: cursor_point.line + display_offset as i32,
            col: cursor_point.column,
        }
    }

    pub fn line(&self) -> i32 {
        self.line
    }

    pub fn col(&self) -> usize {
        self.col
    }
}

fn special_input_for_key(key: &str) -> Option<&'static str> {
    match key {
        "backspace" => Some("\x7f"),
        "delete" => Some("\x1b[3~"),
        "enter" => Some("\r"),
        "escape" => Some("\x1b"),
        "tab" => Some("\t"),
        "up" => Some("\x1b[A"),
        "down" => Some("\x1b[B"),
        "right" => Some("\x1b[C"),
        "left" => Some("\x1b[D"),
        "home" => Some("\x1b[H"),
        "end" => Some("\x1b[F"),
        "pageup" => Some("\x1b[5~"),
        "pagedown" => Some("\x1b[6~"),
        _ => None,
    }
}

fn control_input_for_key(key: &str) -> Option<Vec<u8>> {
    let byte = match key {
        "space" | "@" => 0x00,
        "[" | "escape" => 0x1b,
        "\\" => 0x1c,
        "]" => 0x1d,
        "^" => 0x1e,
        "_" => 0x1f,
        "?" | "backspace" => 0x7f,
        "enter" => b'\n',
        key if key.len() == 1 => {
            let byte = key.as_bytes()[0].to_ascii_lowercase();
            if byte.is_ascii_lowercase() {
                byte - b'a' + 1
            } else {
                return special_input_for_key(key)
                    .map(str::as_bytes)
                    .map(ToOwned::to_owned);
            }
        }
        _ => {
            return special_input_for_key(key)
                .map(str::as_bytes)
                .map(ToOwned::to_owned);
        }
    };

    Some(vec![byte])
}
// 定义你的自定义元素结构体

fn slice_utf16(text: &str, range: std::ops::Range<usize>) -> String {
    let mut start_byte = text.len();
    let mut end_byte = text.len();
    let mut utf16_index = 0;

    for (byte_index, character) in text.char_indices() {
        if utf16_index == range.start {
            start_byte = byte_index;
        }
        if utf16_index == range.end {
            end_byte = byte_index;
            break;
        }
        utf16_index += character.len_utf16();
    }

    if range.start == utf16_index {
        start_byte = text.len();
    }
    if range.end == utf16_index {
        end_byte = text.len();
    }

    text[start_byte..end_byte].to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalBounds {
    pub cell_width: Pixels,
    pub line_height: Pixels,
    pub bounds: Bounds<Pixels>,
}

impl TerminalBounds {
    pub fn new(line_height: Pixels, cell_width: Pixels, bounds: Bounds<Pixels>) -> Self {
        TerminalBounds {
            cell_width,
            line_height,
            bounds,
        }
    }

    pub fn num_lines(&self) -> usize {
        // Tolerance to prevent f32 precision from losing a row:
        // `N * line_height / line_height` can be N-epsilon, which floor()
        // would round down, pushing the first line into invisible scrollback.
        let raw = self.bounds.size.height / self.line_height;
        raw.next_up().floor() as usize
    }

    pub fn num_columns(&self) -> usize {
        let raw = self.bounds.size.width / self.cell_width;
        raw.next_up().floor() as usize
    }

    pub fn height(&self) -> Pixels {
        self.bounds.size.height
    }

    pub fn width(&self) -> Pixels {
        self.bounds.size.width
    }

    pub fn cell_width(&self) -> Pixels {
        self.cell_width
    }

    pub fn line_height(&self) -> Pixels {
        self.line_height
    }
}

impl Default for TerminalBounds {
    fn default() -> Self {
        TerminalBounds::new(
            DEBUG_LINE_HEIGHT,
            DEBUG_CELL_WIDTH,
            Bounds {
                origin: GpuiPoint::default(),
                size: Size {
                    width: DEBUG_TERMINAL_WIDTH,
                    height: DEBUG_TERMINAL_HEIGHT,
                },
            },
        )
    }
}

fn normalize_terminal_bounds(mut bounds: TerminalBounds) -> TerminalBounds {
    bounds.bounds.size.height = cmp::max(bounds.line_height, bounds.height());
    bounds.bounds.size.width = cmp::max(bounds.cell_width, bounds.width());
    bounds
}
const DEBUG_TERMINAL_WIDTH: Pixels = px(500.);
const DEBUG_TERMINAL_HEIGHT: Pixels = px(30.);
const DEBUG_CELL_WIDTH: Pixels = px(5.);
const DEBUG_LINE_HEIGHT: Pixels = px(5.);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Range {
    start: Point,
    end: Point,
}

impl Range {
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> Point {
        self.start
    }

    pub fn end(&self) -> Point {
        self.end
    }

    pub fn contains(&self, point: Point) -> bool {
        self.start <= point && point <= self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectionRange {
    pub start: Point,
    pub end: Point,
    pub is_block: bool,
}

impl SelectionRange {
    pub fn point_range(self) -> Range {
        Range::new(self.start, self.end)
    }
}

// TODO: Un-pub
#[derive(Clone)]
pub struct Content {
    pub cells: Vec<IndexedCell>,
    // pub mode: Modes,
    pub display_offset: usize,
    // pub selection_text: Option<String>,
    // pub selection: Option<SelectionRange>,
    pub cursor: Cursor,
    pub cursor_char: char,
    pub terminal_bounds: TerminalBounds,
    pub last_hovered_word: Option<HoveredWord>,
    pub scrolled_to_top: bool,
    pub scrolled_to_bottom: bool,
    pub bottom_row_occupied: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HoveredWord {
    pub word: String,
    pub word_match: Range,
    pub id: usize,
}

impl Default for Content {
    fn default() -> Self {
        Content {
            cells: Default::default(),
            // mode: Default::default(),
            display_offset: Default::default(),
            // selection_text: Default::default(),
            // selection: Default::default(),
            cursor: Cursor {
                shape: CursorShape::Block,
                point: Point::new(0, 0),
            },
            cursor_char: Default::default(),
            terminal_bounds: Default::default(),
            last_hovered_word: None,
            scrolled_to_top: false,
            scrolled_to_bottom: false,
            bottom_row_occupied: false,
        }
    }
}

#[derive(PartialEq, Eq)]
enum SelectionPhase {
    Selecting,
    Ended,
}

#[derive(Clone, Copy, Debug)]
enum Scroll {
    Delta(i32),
    PageUp,
    PageDown,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
enum ViMotion {
    Up,
    Down,
    Left,
    Right,
    First,
    Last,
    FirstOccupied,
    High,
    Middle,
    Low,
    WordLeft,
    WordRight,
    WordRightEnd,
    Bracket,
    ParagraphUp,
    ParagraphDown,
}

// #[derive(Clone, Debug)]
// pub struct Search {
//     search: AlacrittySearch,
// }

#[derive(Clone, Debug)]
struct Selection {
    ty: SelectionType,
    start: SelectionAnchor,
    end: SelectionAnchor,
    head: Point,
}

#[derive(Clone, Copy, Debug)]
struct SelectionAnchor {
    point: Point,
    side: SelectionSide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionType {
    Simple,
    Semantic,
    Lines,
}

impl Selection {
    fn new(selection_type: SelectionType, point: Point, side: SelectionSide) -> Self {
        let anchor = SelectionAnchor { point, side };
        Self {
            ty: selection_type,
            start: anchor,
            end: anchor,
            head: point,
        }
    }

    fn simple_range(range: Range) -> Self {
        let mut selection = Self::new(SelectionType::Simple, range.start(), SelectionSide::Left);
        selection.update(range.end(), SelectionSide::Right);
        selection
    }

    fn update(&mut self, point: Point, side: SelectionSide) {
        self.end = SelectionAnchor { point, side };
        self.head = point;
    }
}

pub fn is_default_background_color(color: Color) -> bool {
    matches!(color, Color::Named(NamedColor::Background))
}

pub fn is_app_chosen_exact_color(color: Color) -> bool {
    matches!(color, Color::Spec(_) | Color::Indexed(16..=255))
}

pub type AnsiSpans = Vec<(StdRange<usize>, Option<Color>)>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParsedAnsiText {
    pub text: String,
    pub foreground_spans: AnsiSpans,
    pub background_spans: AnsiSpans,
}

pub fn parse_ansi_text(input: &[u8]) -> ParsedAnsiText {
    let mut handler = StyledAnsiTextHandler::default();
    let mut processor = Processor::<StdSyncHandler>::default();
    processor.advance(&mut handler, input);
    handler.finish()
}

pub fn strip_ansi_text(input: &[u8]) -> String {
    let mut handler = PlainAnsiTextHandler::default();
    let mut processor = Processor::<StdSyncHandler>::default();
    processor.advance(&mut handler, input);
    handler.text
}

#[derive(Default)]
struct StyledAnsiTextHandler {
    text: String,
    foreground_spans: AnsiSpans,
    background_spans: AnsiSpans,
    current_foreground_range_start: usize,
    current_background_range_start: usize,
    current_foreground_color: Option<Color>,
    current_background_color: Option<Color>,
}

impl StyledAnsiTextHandler {
    fn finish(mut self) -> ParsedAnsiText {
        if self.current_foreground_range_start < self.text.len() {
            self.foreground_spans.push((
                self.current_foreground_range_start..self.text.len(),
                self.current_foreground_color,
            ));
        }

        if self.current_background_range_start < self.text.len() {
            self.background_spans.push((
                self.current_background_range_start..self.text.len(),
                self.current_background_color,
            ));
        }

        ParsedAnsiText {
            text: self.text,
            foreground_spans: self.foreground_spans,
            background_spans: self.background_spans,
        }
    }

    fn break_foreground_span(&mut self, color: Option<Color>) {
        self.foreground_spans.push((
            self.current_foreground_range_start..self.text.len(),
            self.current_foreground_color,
        ));
        self.current_foreground_color = color;
        self.current_foreground_range_start = self.text.len();
    }

    fn break_background_span(&mut self, color: Option<Color>) {
        self.background_spans.push((
            self.current_background_range_start..self.text.len(),
            self.current_background_color,
        ));
        self.current_background_color = color;
        self.current_background_range_start = self.text.len();
    }
}

impl Handler for StyledAnsiTextHandler {
    fn input(&mut self, c: char) {
        self.text.push(c);
    }

    fn linefeed(&mut self) {
        self.text.push('\n');
    }

    fn put_tab(&mut self, count: u16) {
        self.text.extend(std::iter::repeat_n('\t', count as usize));
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        match attr {
            Attr::Foreground(color) => {
                self.break_foreground_span(Some(color));
            }
            Attr::Background(color) => {
                self.break_background_span(Some(color));
            }
            Attr::Reset => {
                self.break_foreground_span(None);
                self.break_background_span(None);
            }
            _ => {}
        }
    }
}

#[derive(Default)]
struct PlainAnsiTextHandler {
    text: String,
    line_start: usize,
}

impl Handler for PlainAnsiTextHandler {
    fn input(&mut self, c: char) {
        self.text.push(c);
    }

    fn linefeed(&mut self) {
        self.text.push('\n');
        self.line_start = self.text.len();
    }

    fn carriage_return(&mut self) {
        self.text.truncate(self.line_start);
    }

    fn put_tab(&mut self, count: u16) {
        self.text.extend(std::iter::repeat_n('\t', count as usize));
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Hyperlink {
    data: HyperlinkData,
}
#[derive(Debug, Clone, Eq, PartialEq)]
enum HyperlinkData {
    Alacritty(AlacrittyHyperlink),
    Owned { id: Option<Arc<str>>, uri: Arc<str> },
}
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Cell {
    pub cell: AlacrittyCell,
}

impl Cell {
    #[inline]
    pub fn character(&self) -> char {
        self.cell.c
    }

    #[cfg(test)]
    pub(crate) fn set_character(&mut self, character: char) {
        self.cell.c = character;
    }

    #[inline]
    pub fn foreground(&self) -> Color {
        self.cell.fg
    }

    #[inline]
    pub fn background(&self) -> Color {
        self.cell.bg
    }

    #[inline]
    pub fn zerowidth(&self) -> Option<&[char]> {
        self.cell.zerowidth()
    }

    #[cfg(test)]
    pub(crate) fn push_zerowidth(&mut self, character: char) {
        self.cell.push_zerowidth(character);
    }

    #[inline]
    pub fn hyperlink(&self) -> Option<Hyperlink> {
        None
        // self.cell.hyperlink().map(terminal_hyperlink_from_alacritty)
    }

    #[inline]
    pub fn is_inverse(&self) -> bool {
        self.cell.flags.contains(Flags::INVERSE)
    }

    #[inline]
    pub fn is_wide_char_spacer(&self) -> bool {
        self.cell.flags.contains(Flags::WIDE_CHAR_SPACER)
    }

    #[inline]
    pub fn is_dim(&self) -> bool {
        self.cell.flags.intersects(Flags::DIM)
    }

    #[inline]
    pub fn has_underline(&self) -> bool {
        self.cell.flags.intersects(Flags::ALL_UNDERLINES)
    }

    #[inline]
    pub fn has_undercurl(&self) -> bool {
        self.cell.flags.contains(Flags::UNDERCURL)
    }

    #[inline]
    pub fn has_strikeout(&self) -> bool {
        self.cell.flags.intersects(Flags::STRIKEOUT)
    }

    #[inline]
    pub fn is_bold(&self) -> bool {
        self.cell.flags.intersects(Flags::BOLD)
    }

    #[inline]
    pub fn is_italic(&self) -> bool {
        self.cell.flags.intersects(Flags::ITALIC)
    }

    #[inline]
    pub fn has_visible_style_modifier(&self) -> bool {
        self.cell
            .flags
            .intersects(Flags::ALL_UNDERLINES | Flags::INVERSE | Flags::STRIKEOUT)
    }
}

pub type AlacrittyCell = alacritty_terminal::term::cell::Cell;
pub type AlacrittyGridIterator<'a> = GridIterator<'a, alacritty_terminal::term::cell::Cell>;
pub type AlacrittyHyperlink = alacritty_terminal::term::cell::Hyperlink;
// fn terminal_hyperlink_from_alacritty(hyperlink: AlacrittyHyperlink) -> Hyperlink {
// Hyperlink::from_alacritty(hyperlink)
// }
pub struct RenderableCells<'a> {
    cells: AlacrittyGridIterator<'a>,
}

#[derive(Debug, Clone)]
pub struct IndexedCell {
    pub point: Point,
    pub cell: Cell,
}

impl Deref for IndexedCell {
    type Target = Cell;

    #[inline]
    fn deref(&self) -> &Cell {
        &self.cell
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modes(u32);

impl Modes {
    pub const NONE: Self = Self(0);
    pub const APP_CURSOR: Self = Self(1 << 0);
    pub const APP_KEYPAD: Self = Self(1 << 1);
    pub const SHOW_CURSOR: Self = Self(1 << 2);
    pub const LINE_WRAP: Self = Self(1 << 3);
    pub const ORIGIN: Self = Self(1 << 4);
    pub const INSERT: Self = Self(1 << 5);
    pub const LINE_FEED_NEW_LINE: Self = Self(1 << 6);
    pub const FOCUS_IN_OUT: Self = Self(1 << 7);
    pub const ALTERNATE_SCROLL: Self = Self(1 << 8);
    pub const BRACKETED_PASTE: Self = Self(1 << 9);
    pub const SGR_MOUSE: Self = Self(1 << 10);
    pub const UTF8_MOUSE: Self = Self(1 << 11);
    pub const ALT_SCREEN: Self = Self(1 << 12);
    pub const MOUSE_REPORT_CLICK: Self = Self(1 << 13);
    pub const MOUSE_DRAG: Self = Self(1 << 14);
    pub const MOUSE_MOTION: Self = Self(1 << 15);
    pub const VI: Self = Self(1 << 16);
    pub const MOUSE_MODE: Self =
        Self(Self::MOUSE_REPORT_CLICK.0 | Self::MOUSE_DRAG.0 | Self::MOUSE_MOTION.0);

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

impl BitOr for Modes {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Modes {
    fn bitor_assign(&mut self, rhs: Self) {
        self.insert(rhs);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cursor {
    pub shape: CursorShape,
    pub point: Point,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CursorShape {
    Block,
    Underline,
    Bar,
    HollowBlock,
    Hidden,
}
pub type SettingsCursorShape = terminal_settings::CursorShape;
impl From<SettingsCursorShape> for CursorShape {
    fn from(shape: SettingsCursorShape) -> Self {
        match shape {
            SettingsCursorShape::Block => Self::Block,
            SettingsCursorShape::Underline => Self::Underline,
            SettingsCursorShape::Bar => Self::Bar,
            SettingsCursorShape::Hollow => Self::HollowBlock,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Point {
    pub line: i32,
    pub column: usize,
}

impl Point {
    pub fn new(line: i32, column: usize) -> Self {
        Self { line, column }
    }
}
#[derive(Debug, PartialEq, Eq)]
enum TerminalModifiers {
    None,
    Alt,
    Ctrl,
    Shift,
    CtrlShift,
    Other,
}

impl TerminalModifiers {
    fn new(ks: &Keystroke) -> Self {
        match (
            ks.modifiers.alt,
            ks.modifiers.control,
            ks.modifiers.shift,
            ks.modifiers.platform,
        ) {
            (false, false, false, false) => TerminalModifiers::None,
            (true, false, false, false) => TerminalModifiers::Alt,
            (false, true, false, false) => TerminalModifiers::Ctrl,
            (false, false, true, false) => TerminalModifiers::Shift,
            (false, true, true, false) => TerminalModifiers::CtrlShift,
            _ => TerminalModifiers::Other,
        }
    }

    fn any(&self) -> bool {
        match &self {
            TerminalModifiers::None => false,
            TerminalModifiers::Alt => true,
            TerminalModifiers::Ctrl => true,
            TerminalModifiers::Shift => true,
            TerminalModifiers::CtrlShift => true,
            TerminalModifiers::Other => true,
        }
    }
}

pub(crate) fn to_esc_str(
    keystroke: &Keystroke,
    mode: Modes,
    option_as_meta: bool,
) -> Option<Cow<'static, str>> {
    let modifiers = TerminalModifiers::new(keystroke);

    // Manual Bindings including modifiers
    let manual_esc_str: Option<&'static str> = match (keystroke.key.as_ref(), &modifiers) {
        //Basic special keys
        ("tab", TerminalModifiers::None) => Some("\x09"),
        ("escape", TerminalModifiers::None) => Some("\x1b"),
        ("enter", TerminalModifiers::None) => Some("\x0d"),
        ("enter", TerminalModifiers::Shift) => Some("\x0a"),
        ("enter", TerminalModifiers::Alt) => Some("\x1b\x0d"),
        ("backspace", TerminalModifiers::None) => Some("\x7f"),
        //Interesting escape codes
        ("tab", TerminalModifiers::Shift) => Some("\x1b[Z"),
        ("backspace", TerminalModifiers::Ctrl) => Some("\x08"),
        ("backspace", TerminalModifiers::Alt) => Some("\x1b\x7f"),
        ("backspace", TerminalModifiers::Shift) => Some("\x7f"),
        ("space", TerminalModifiers::Ctrl) => Some("\x00"),
        ("home", TerminalModifiers::None) if mode.contains(Modes::APP_CURSOR) => Some("\x1bOH"),
        ("home", TerminalModifiers::None) if !mode.contains(Modes::APP_CURSOR) => Some("\x1b[H"),
        ("end", TerminalModifiers::None) if mode.contains(Modes::APP_CURSOR) => Some("\x1bOF"),
        ("end", TerminalModifiers::None) if !mode.contains(Modes::APP_CURSOR) => Some("\x1b[F"),
        ("up", TerminalModifiers::None) if mode.contains(Modes::APP_CURSOR) => Some("\x1bOA"),
        ("up", TerminalModifiers::None) if !mode.contains(Modes::APP_CURSOR) => Some("\x1b[A"),
        ("down", TerminalModifiers::None) if mode.contains(Modes::APP_CURSOR) => Some("\x1bOB"),
        ("down", TerminalModifiers::None) if !mode.contains(Modes::APP_CURSOR) => Some("\x1b[B"),
        ("right", TerminalModifiers::None) if mode.contains(Modes::APP_CURSOR) => Some("\x1bOC"),
        ("right", TerminalModifiers::None) if !mode.contains(Modes::APP_CURSOR) => Some("\x1b[C"),
        ("left", TerminalModifiers::None) if mode.contains(Modes::APP_CURSOR) => Some("\x1bOD"),
        ("left", TerminalModifiers::None) if !mode.contains(Modes::APP_CURSOR) => Some("\x1b[D"),
        ("back", TerminalModifiers::None) => Some("\x7f"),
        ("insert", TerminalModifiers::None) => Some("\x1b[2~"),
        ("delete", TerminalModifiers::None) => Some("\x1b[3~"),
        ("pageup", TerminalModifiers::None) => Some("\x1b[5~"),
        ("pagedown", TerminalModifiers::None) => Some("\x1b[6~"),
        ("f1", TerminalModifiers::None) => Some("\x1bOP"),
        ("f2", TerminalModifiers::None) => Some("\x1bOQ"),
        ("f3", TerminalModifiers::None) => Some("\x1bOR"),
        ("f4", TerminalModifiers::None) => Some("\x1bOS"),
        ("f5", TerminalModifiers::None) => Some("\x1b[15~"),
        ("f6", TerminalModifiers::None) => Some("\x1b[17~"),
        ("f7", TerminalModifiers::None) => Some("\x1b[18~"),
        ("f8", TerminalModifiers::None) => Some("\x1b[19~"),
        ("f9", TerminalModifiers::None) => Some("\x1b[20~"),
        ("f10", TerminalModifiers::None) => Some("\x1b[21~"),
        ("f11", TerminalModifiers::None) => Some("\x1b[23~"),
        ("f12", TerminalModifiers::None) => Some("\x1b[24~"),
        ("f13", TerminalModifiers::None) => Some("\x1b[25~"),
        ("f14", TerminalModifiers::None) => Some("\x1b[26~"),
        ("f15", TerminalModifiers::None) => Some("\x1b[28~"),
        ("f16", TerminalModifiers::None) => Some("\x1b[29~"),
        ("f17", TerminalModifiers::None) => Some("\x1b[31~"),
        ("f18", TerminalModifiers::None) => Some("\x1b[32~"),
        ("f19", TerminalModifiers::None) => Some("\x1b[33~"),
        ("f20", TerminalModifiers::None) => Some("\x1b[34~"),
        // NumpadEnter, Action::Esc("\n".into());
        //Mappings for caret notation keys
        ("a", TerminalModifiers::Ctrl) => Some("\x01"), //1
        ("A", TerminalModifiers::CtrlShift) => Some("\x01"), //1
        ("b", TerminalModifiers::Ctrl) => Some("\x02"), //2
        ("B", TerminalModifiers::CtrlShift) => Some("\x02"), //2
        ("c", TerminalModifiers::Ctrl) => Some("\x03"), //3
        ("C", TerminalModifiers::CtrlShift) => Some("\x03"), //3
        ("d", TerminalModifiers::Ctrl) => Some("\x04"), //4
        ("D", TerminalModifiers::CtrlShift) => Some("\x04"), //4
        ("e", TerminalModifiers::Ctrl) => Some("\x05"), //5
        ("E", TerminalModifiers::CtrlShift) => Some("\x05"), //5
        ("f", TerminalModifiers::Ctrl) => Some("\x06"), //6
        ("F", TerminalModifiers::CtrlShift) => Some("\x06"), //6
        ("g", TerminalModifiers::Ctrl) => Some("\x07"), //7
        ("G", TerminalModifiers::CtrlShift) => Some("\x07"), //7
        ("h", TerminalModifiers::Ctrl) => Some("\x08"), //8
        ("H", TerminalModifiers::CtrlShift) => Some("\x08"), //8
        ("i", TerminalModifiers::Ctrl) => Some("\x09"), //9
        ("I", TerminalModifiers::CtrlShift) => Some("\x09"), //9
        ("j", TerminalModifiers::Ctrl) => Some("\x0a"), //10
        ("J", TerminalModifiers::CtrlShift) => Some("\x0a"), //10
        ("k", TerminalModifiers::Ctrl) => Some("\x0b"), //11
        ("K", TerminalModifiers::CtrlShift) => Some("\x0b"), //11
        ("l", TerminalModifiers::Ctrl) => Some("\x0c"), //12
        ("L", TerminalModifiers::CtrlShift) => Some("\x0c"), //12
        ("m", TerminalModifiers::Ctrl) => Some("\x0d"), //13
        ("M", TerminalModifiers::CtrlShift) => Some("\x0d"), //13
        ("n", TerminalModifiers::Ctrl) => Some("\x0e"), //14
        ("N", TerminalModifiers::CtrlShift) => Some("\x0e"), //14
        ("o", TerminalModifiers::Ctrl) => Some("\x0f"), //15
        ("O", TerminalModifiers::CtrlShift) => Some("\x0f"), //15
        ("p", TerminalModifiers::Ctrl) => Some("\x10"), //16
        ("P", TerminalModifiers::CtrlShift) => Some("\x10"), //16
        ("q", TerminalModifiers::Ctrl) => Some("\x11"), //17
        ("Q", TerminalModifiers::CtrlShift) => Some("\x11"), //17
        ("r", TerminalModifiers::Ctrl) => Some("\x12"), //18
        ("R", TerminalModifiers::CtrlShift) => Some("\x12"), //18
        ("s", TerminalModifiers::Ctrl) => Some("\x13"), //19
        ("S", TerminalModifiers::CtrlShift) => Some("\x13"), //19
        ("t", TerminalModifiers::Ctrl) => Some("\x14"), //20
        ("T", TerminalModifiers::CtrlShift) => Some("\x14"), //20
        ("u", TerminalModifiers::Ctrl) => Some("\x15"), //21
        ("U", TerminalModifiers::CtrlShift) => Some("\x15"), //21
        ("v", TerminalModifiers::Ctrl) => Some("\x16"), //22
        ("V", TerminalModifiers::CtrlShift) => Some("\x16"), //22
        ("w", TerminalModifiers::Ctrl) => Some("\x17"), //23
        ("W", TerminalModifiers::CtrlShift) => Some("\x17"), //23
        ("x", TerminalModifiers::Ctrl) => Some("\x18"), //24
        ("X", TerminalModifiers::CtrlShift) => Some("\x18"), //24
        ("y", TerminalModifiers::Ctrl) => Some("\x19"), //25
        ("Y", TerminalModifiers::CtrlShift) => Some("\x19"), //25
        ("z", TerminalModifiers::Ctrl) => Some("\x1a"), //26
        ("Z", TerminalModifiers::CtrlShift) => Some("\x1a"), //26
        ("@", TerminalModifiers::Ctrl) => Some("\x00"), //0
        ("[", TerminalModifiers::Ctrl) => Some("\x1b"), //27
        ("\\", TerminalModifiers::Ctrl) => Some("\x1c"), //28
        ("]", TerminalModifiers::Ctrl) => Some("\x1d"), //29
        ("^", TerminalModifiers::Ctrl) => Some("\x1e"), //30
        ("_", TerminalModifiers::Ctrl) => Some("\x1f"), //31
        ("?", TerminalModifiers::Ctrl) => Some("\x7f"), //127
        _ => None,
    };
    if let Some(esc_str) = manual_esc_str {
        return Some(Cow::Borrowed(esc_str));
    }

    // Automated bindings applying modifiers
    if modifiers.any() {
        let modifier_code = modifier_code(keystroke);
        let modified_esc_str = match keystroke.key.as_ref() {
            "up" => Some(format!("\x1b[1;{}A", modifier_code)),
            "down" => Some(format!("\x1b[1;{}B", modifier_code)),
            "right" => Some(format!("\x1b[1;{}C", modifier_code)),
            "left" => Some(format!("\x1b[1;{}D", modifier_code)),
            "f1" => Some(format!("\x1b[1;{}P", modifier_code)),
            "f2" => Some(format!("\x1b[1;{}Q", modifier_code)),
            "f3" => Some(format!("\x1b[1;{}R", modifier_code)),
            "f4" => Some(format!("\x1b[1;{}S", modifier_code)),
            "F5" => Some(format!("\x1b[15;{}~", modifier_code)),
            "f6" => Some(format!("\x1b[17;{}~", modifier_code)),
            "f7" => Some(format!("\x1b[18;{}~", modifier_code)),
            "f8" => Some(format!("\x1b[19;{}~", modifier_code)),
            "f9" => Some(format!("\x1b[20;{}~", modifier_code)),
            "f10" => Some(format!("\x1b[21;{}~", modifier_code)),
            "f11" => Some(format!("\x1b[23;{}~", modifier_code)),
            "f12" => Some(format!("\x1b[24;{}~", modifier_code)),
            "f13" => Some(format!("\x1b[25;{}~", modifier_code)),
            "f14" => Some(format!("\x1b[26;{}~", modifier_code)),
            "f15" => Some(format!("\x1b[28;{}~", modifier_code)),
            "f16" => Some(format!("\x1b[29;{}~", modifier_code)),
            "f17" => Some(format!("\x1b[31;{}~", modifier_code)),
            "f18" => Some(format!("\x1b[32;{}~", modifier_code)),
            "f19" => Some(format!("\x1b[33;{}~", modifier_code)),
            "f20" => Some(format!("\x1b[34;{}~", modifier_code)),
            "insert" => Some(format!("\x1b[2;{}~", modifier_code)),
            "pageup" => Some(format!("\x1b[5;{}~", modifier_code)),
            "pagedown" => Some(format!("\x1b[6;{}~", modifier_code)),
            "end" => Some(format!("\x1b[1;{}F", modifier_code)),
            "home" => Some(format!("\x1b[1;{}H", modifier_code)),
            _ => None,
        };
        if let Some(esc_str) = modified_esc_str {
            return Some(Cow::Owned(esc_str));
        }
    }

    if !cfg!(target_os = "macos") || option_as_meta {
        let is_alt_lowercase_ascii =
            modifiers == TerminalModifiers::Alt && keystroke.key.is_ascii();
        let is_alt_uppercase_ascii =
            keystroke.modifiers.alt && keystroke.modifiers.shift && keystroke.key.is_ascii();
        if is_alt_lowercase_ascii || is_alt_uppercase_ascii {
            let key = if is_alt_uppercase_ascii {
                &keystroke.key.to_ascii_uppercase()
            } else {
                &keystroke.key
            };
            return Some(Cow::Owned(format!("\x1b{}", key)));
        }
    }

    None
}
///   Code     Modifiers
/// ---------+---------------------------
///    2     | Shift
///    3     | Alt
///    4     | Shift + Alt
///    5     | Control
///    6     | Shift + Control
///    7     | Alt + Control
///    8     | Shift + Alt + Control
/// ---------+---------------------------
/// from: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-PC-Style-Function-Keys
fn modifier_code(keystroke: &Keystroke) -> u32 {
    let mut modifier_code = 0;
    if keystroke.modifiers.shift {
        modifier_code |= 1;
    }
    if keystroke.modifiers.alt {
        modifier_code |= 1 << 1;
    }
    if keystroke.modifiers.control {
        modifier_code |= 1 << 2;
    }
    modifier_code + 1
}
