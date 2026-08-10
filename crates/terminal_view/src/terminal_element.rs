use std::time::{SystemTime, UNIX_EPOCH};
use std::{mem, rc::Rc, time::Instant};

use crate::TerminalView;
use gpui::{
    AbsoluteLength, AnyElement, App, AvailableSpace, Bounds, ContentMask, DefiniteLength,
    DispatchPhase, Element, Entity, FocusHandle, Font, FontFeatures, FontStyle, FontWeight,
    HighlightStyle, Hitbox, HitboxBehavior, Hsla, InputHandler, InteractiveElement, Interactivity,
    IntoElement, KeyDownEvent, Keystroke, ModifiersChangedEvent, MouseButton, MouseMoveEvent,
    ParentElement, Pixels, Point as GpuiPoint, ShapedLine, StrikethroughStyle, TextAlign, TextRun,
    TextStyle, UTF16Selection, UnderlineStyle, WhiteSpace, Window, div, fill, font, hsla, point,
    px, relative, size,
};
use itertools::Itertools;
use settings::Settings;
use terminal::{
    BASE_REM_SIZE_IN_PX, BlockContext, BlockProperties, Cell, Content, CursorShape, DisplayCursor,
    IndexedCell, Modes, Point, Range, is_blank,
};
use terminal::{Terminal, TerminalBounds};
use theme_settings::ThemeSettings;

pub struct TerminalElement {
    terminal: Entity<Terminal>,
    pub terminal_view: Entity<TerminalView>,
    focus: FocusHandle,
    focused: bool,
    cursor_visible: bool,
    interactivity: Interactivity,
    block_below_cursor: Option<Rc<BlockProperties>>,
}
impl TerminalElement {
    pub fn new(
        terminal: Entity<Terminal>,
        terminal_view: Entity<TerminalView>,
        focus: FocusHandle,
        focused: bool,
        cursor_visible: bool,
        block_below_cursor: Option<Rc<BlockProperties>>,
    ) -> Self {
        Self {
            terminal,
            terminal_view,
            focus: focus.clone(),
            focused,
            cursor_visible,
            interactivity: Default::default(),
            block_below_cursor,
        }
    }

    fn block_element_regions_to_rects(
        regions: Vec<BackgroundRegion>,
    ) -> Vec<BlockElementLayoutRect> {
        merge_background_regions(regions)
            .into_iter()
            .map(|region| {
                BlockElementLayoutRect::new(
                    LayoutPoint::new(region.start_line, region.start_col),
                    (region.end_col - region.start_col + 1) as usize,
                    (region.end_line - region.start_line + 1) as usize,
                    region.color,
                )
            })
            .collect()
    }
    pub fn layout_grid<T: TerminalLayoutCell>(
        grid: impl Iterator<Item = T>,
        start_line_offset: i32,
        text_style: &TextStyle,
        hyperlink: Option<(HighlightStyle, &Range)>,
        minimum_contrast: f32,
        cx: &App,
    ) -> (
        Vec<LayoutRect>,
        Vec<BatchedTextRun>,
        Vec<BlockElementLayoutRect>,
    ) {
        let start_time = Instant::now();

        // 内存预分配优化（pre-allocation）
        // 减少 Vec 在不断 push() 时触发扩容和内存搬迁。
        let estimated_cells = grid.size_hint().0;
        let estimated_runs = estimated_cells / 10; // Estimate ~10 cells per run
        let estimated_regions = estimated_cells / 20; // Estimate ~20 cells per background region

        // 表示一段连续文本
        let mut batched_runs = Vec::with_capacity(estimated_runs);
        // 背景区域
        let mut block_element_regions = Vec::new();
        let mut cell_count = 0;

        // Collect background regions for efficient merging
        let mut background_regions: Vec<BackgroundRegion> = Vec::with_capacity(estimated_regions);
        let mut current_batch: Option<BatchedTextRun> = None;

        // First pass: collect all cells and their backgrounds
        let linegroups = grid.into_iter().chunk_by(|cell| cell.point().line);
        for (line_index, (_, line)) in linegroups.into_iter().enumerate() {
            let display_line = start_line_offset + line_index as i32;

            // Flush any existing batch at line boundaries
            if let Some(batch) = current_batch.take() {
                batched_runs.push(batch);
            }

            let mut previous_cell_had_extras = false;

            for cell in line {
                let point = cell.point();
                let cell = cell.cell();
                let mut fg = cell.cell.fg;
                let mut bg = cell.cell.bg;
                // if cell.cell.flags.contains(Flags::INVERSE) {
                //     mem::swap(&mut fg, &mut bg);
                // }

                // Collect background regions (skip default background)
                // if !is_default_background_color(bg) {
                //     let color = convert_color(&bg, theme);
                //     let col = point.column as i32;

                //     // Try to extend the last region if it's on the same line with the same color
                //     if let Some(last_region) = background_regions.last_mut()
                //         && last_region.color == color
                //         && last_region.start_line == display_line
                //         && last_region.end_line == display_line
                //         && last_region.end_col + 1 == col
                //     {
                //         last_region.end_col = col;
                //     } else {
                //         background_regions.push(BackgroundRegion::new(display_line, col, color));
                //     }
                // }
                // Skip wide character spacers - they're just placeholders for the second cell of wide characters
                // if cell.cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                //     continue;
                // }

                // Skip spaces that follow cells with extras (emoji variation sequences)
                if cell.character() == ' ' && previous_cell_had_extras {
                    previous_cell_had_extras = false;
                    continue;
                }
                // Update tracking for next iteration
                previous_cell_had_extras =
                    matches!(cell.zerowidth(), Some(chars) if !chars.is_empty());

                //Layout current cell text
                {
                    if !is_blank(cell) {
                        cell_count += 1;
                        let cell_style = TerminalElement::cell_style(
                            point,
                            cell,
                            fg,
                            bg,
                            text_style,
                            hyperlink,
                            minimum_contrast,
                        );

                        let cell_point = LayoutPoint::new(display_line, point.column as i32);
                        // if Self::collect_block_element_regions(
                        //     cell_point,
                        //     cell.character(),
                        //     cell_style.color,
                        //     &mut block_element_regions,
                        // ) {
                        //     if let Some(batch) = current_batch.take() {
                        //         batched_runs.push(batch);
                        //     }
                        //     continue;
                        // }

                        let zero_width_chars = cell.zerowidth();

                        // Try to batch with existing run
                        if let Some(ref mut batch) = current_batch {
                            if batch.can_append(&cell_style)
                                && batch.start_point.line == cell_point.line
                                && batch.start_point.column + batch.cell_count as i32
                                    == cell_point.column
                            {
                                batch.append_char(cell.character());
                                if let Some(chars) = zero_width_chars {
                                    batch.append_zero_width_chars(chars);
                                }
                            } else {
                                // Flush current batch and start new one
                                let old_batch = current_batch.take().unwrap();
                                batched_runs.push(old_batch);
                                let mut new_batch = BatchedTextRun::new_from_char(
                                    cell_point,
                                    cell.character(),
                                    TextRun::default(),
                                    text_style.font_size,
                                );
                                if let Some(chars) = zero_width_chars {
                                    new_batch.append_zero_width_chars(chars);
                                }
                                current_batch = Some(new_batch);
                            }
                        } else {
                            // Start new batch
                            let mut new_batch = BatchedTextRun::new_from_char(
                                cell_point,
                                cell.character(),
                                TextRun::default(),
                                text_style.font_size,
                            );
                            if let Some(chars) = zero_width_chars {
                                new_batch.append_zero_width_chars(chars);
                            }
                            current_batch = Some(new_batch);
                        }
                    };
                }
            }
        }

        // Flush any remaining batch
        if let Some(batch) = current_batch {
            batched_runs.push(batch);
        }

        // Second pass: merge background regions and convert to layout rects
        let region_count = background_regions.len();
        let merged_regions = merge_background_regions(background_regions);
        let mut rects = Vec::with_capacity(merged_regions.len() * 2); // Estimate 2 rects per merged region

        // Convert merged regions to layout rects
        // Since LayoutRect only supports single-line rectangles, we need to split multi-line regions
        for region in merged_regions {
            for line in region.start_line..=region.end_line {
                rects.push(LayoutRect::new(
                    LayoutPoint::new(line, region.start_col),
                    (region.end_col - region.start_col + 1) as usize,
                    region.color,
                ));
            }
        }

        let block_element_region_count = block_element_regions.len();
        let block_element_rects = Self::block_element_regions_to_rects(block_element_regions);
        let layout_time = start_time.elapsed();

        log::debug!(
            "Terminal layout_grid: {} cells processed, \
            {} batched runs created, {} block element rects (from {} regions), {} rects (from {} merged regions), \
            layout took {:?}",
            cell_count,
            batched_runs.len(),
            block_element_rects.len(),
            block_element_region_count,
            rects.len(),
            region_count,
            layout_time
        );

        (rects, batched_runs, block_element_rects)
    }

    fn cell_style(
        point: Point,
        cell: &Cell,
        fg: vte::ansi::Color,
        bg: vte::ansi::Color,
        // colors: &Theme,
        text_style: &TextStyle,
        hyperlink: Option<(HighlightStyle, &Range)>,
        minimum_contrast: f32,
    ) -> TextRun {
        // let skip_contrast = Self::is_app_chosen_exact_color(&fg);

        let skip_contrast = true;
        let mut fg = Hsla::black();
        let bg = Hsla::black();
        // let bg = convert_color(&bg, colors);

        // if !skip_contrast && !Self::is_decorative_character(cell.character()) {
        // fg = ensure_minimum_contrast(fg, bg, minimum_contrast);
        //        }

        // Use a dim multiplier that stays close to the existing Alacritty look.
        if cell.is_dim() {
            fg.a *= 0.7;
        }

        let underline =
            (cell.has_underline() || cell.hyperlink().is_some()).then(|| UnderlineStyle {
                color: Some(fg),
                thickness: Pixels::from(1.0),
                wavy: cell.has_undercurl(),
            });

        let strikethrough = cell.has_strikeout().then(|| StrikethroughStyle {
            color: Some(fg),
            thickness: Pixels::from(1.0),
        });

        let weight = if cell.is_bold() {
            FontWeight::BOLD
        } else {
            text_style.font_weight
        };

        let style = if cell.is_italic() {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };

        let mut result = TextRun {
            len: cell.character().len_utf8(),
            color: fg,
            background_color: None,
            font: Font {
                weight,
                style,
                ..text_style.font()
            },
            underline,
            strikethrough,
        };

        if let Some((style, range)) = hyperlink
            && range.contains(point)
        {
            if let Some(underline) = style.underline {
                result.underline = Some(underline);
            }

            if let Some(color) = style.color {
                result.color = color;
            }
        }

        result
    }
    // 用于在终端模拟器中将逻辑坐标（内部网格位置）转换为显示坐标（屏幕上的像素位置）
    fn cursor_position(
        cursor_point: DisplayCursor,
        size: TerminalBounds,
    ) -> Option<GpuiPoint<Pixels>> {
        if cursor_point.line() < size.num_lines() as i32 {
            // When on pixel boundaries round the origin down
            Some(point(
                (cursor_point.col() as f32 * size.cell_width()).floor(),
                (cursor_point.line() as f32 * size.line_height()).floor(),
            ))
        } else {
            None
        }
    }

    fn register_mouse_listeners(&mut self, mode: Modes, hitbox: &Hitbox, window: &mut Window) {
        let focus = self.focus.clone();
        let terminal = self.terminal.clone();
        let terminal_view = self.terminal_view.clone();

        self.interactivity.on_mouse_down(MouseButton::Left, {
            let terminal = terminal.clone();
            let focus = focus.clone();
            let terminal_view = terminal_view.clone();

            move |e, window, cx| {
                window.focus(&focus, cx);

                let scroll_top = terminal_view.read(cx).scroll_top;
                terminal.update(cx, |terminal, cx| {
                    let mut adjusted_event = e.clone();
                    if scroll_top > Pixels::ZERO {
                        adjusted_event.position.y += scroll_top;
                    }
                    // terminal.mouse_down(&adjusted_event, cx);
                    cx.notify();
                })
            }
        });

        window.on_mouse_event({
            let terminal = self.terminal.clone();
            let hitbox = hitbox.clone();
            let focus = focus.clone();
            let terminal_view = terminal_view;
            move |e: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }

                if e.pressed_button.is_some() && !cx.has_active_drag() && focus.is_focused(window) {
                    let hovered = hitbox.is_hovered(window);

                    let scroll_top = terminal_view.read(cx).scroll_top;
                    // terminal.update(cx, |terminal, cx| {
                    //     // if terminal.selection_started() || hovered {
                    //     //     let mut adjusted_event = e.clone();
                    //     //     if scroll_top > Pixels::ZERO {
                    //     //         adjusted_event.position.y += scroll_top;
                    //     //     }
                    //     //     terminal.mouse_drag(&adjusted_event, hitbox.bounds, cx);
                    //     //     cx.notify();
                    //     // }
                    // })
                }

                if hitbox.is_hovered(window) {
                    // terminal.update(cx, |terminal, cx| {
                    //     terminal.mouse_move(e, cx);
                    // })
                }
            }
        });

        // self.interactivity.on_mouse_up(
        //     MouseButton::Left,
        //     TerminalElement::generic_button_handler(
        //         terminal.clone(),
        //         focus.clone(),
        //         false,
        //         move |terminal, e, cx| {
        //             terminal.mouse_up(e, cx);
        //         },
        //     ),
        // );
        // self.interactivity.on_mouse_down(
        //     MouseButton::Middle,
        //     TerminalElement::generic_button_handler(
        //         terminal.clone(),
        //         focus.clone(),
        //         true,
        //         move |terminal, e, cx| {
        //             terminal.mouse_down(e, cx);
        //         },
        //     ),
        // );

        // if content_mode.is_scrollable() {
        //     self.interactivity.on_scroll_wheel({
        //         let terminal_view = self.terminal_view.downgrade();
        //         move |e, window, cx| {
        //             terminal_view
        //                 .update(cx, |terminal_view, cx| {
        //                     if matches!(terminal_view.mode, TerminalMode::Standalone)
        //                         || terminal_view.focus_handle.is_focused(window)
        //                     {
        //                         terminal_view.scroll_wheel(e, cx);
        //                         cx.notify();
        //                     }
        //                 })
        //                 .ok();
        //         }
        //     });
        // }

        // Mouse mode handlers:
        // All mouse modes need the extra click handlers
        // if mode.intersects(Modes::MOUSE_MODE) {
        //     self.interactivity.on_mouse_down(
        //         MouseButton::Right,
        //         TerminalElement::generic_button_handler(
        //             terminal.clone(),
        //             focus.clone(),
        //             true,
        //             move |terminal, e, cx| {
        //                 terminal.mouse_down(e, cx);
        //             },
        //         ),
        //     );
        //     self.interactivity.on_mouse_up(
        //         MouseButton::Right,
        //         TerminalElement::generic_button_handler(
        //             terminal.clone(),
        //             focus.clone(),
        //             false,
        //             move |terminal, e, cx| {
        //                 terminal.mouse_up(e, cx);
        //             },
        //         ),
        //     );
        //     self.interactivity.on_mouse_up(
        //         MouseButton::Middle,
        //         TerminalElement::generic_button_handler(
        //             terminal,
        //             focus,
        //             false,
        //             move |terminal, e, cx| {
        //                 terminal.mouse_up(e, cx);
        //             },
        //         ),
        //     );
        // }
    }

    fn rem_size(&self, cx: &mut App) -> Option<Pixels> {
        let settings = ThemeSettings::get_global(cx).clone();
        let buffer_font_size = settings.buffer_font_size(cx);
        let rem_size_scale = {
            // Our default UI font size is 14px on a 16px base scale.
            // This means the default UI font size is 0.875rems.
            let default_font_size_scale = 14. / BASE_REM_SIZE_IN_PX;

            // We then determine the delta between a single rem and the default font
            // size scale.
            let default_font_size_delta = 1. - default_font_size_scale;

            // Finally, we add this delta to 1rem to get the scale factor that
            // should be used to scale up the UI.
            1. + default_font_size_delta
        };

        Some(buffer_font_size * rem_size_scale)
    }

    // fn input_for_keystroke(keystroke: &Keystroke) -> Option<Vec<u8>> {
    //     let modifiers = &keystroke.modifiers;
    //     let key = keystroke.key.as_str();

    //     let mut input = if modifiers.control {
    //         // control_input_for_key(key)
    //     } else {
    //         // special_input_for_key(key)
    //         //     .map(str::as_bytes)
    //         //     .map(ToOwned::to_owned)
    //     };

    //     if input.is_none() && (modifiers.alt || modifiers.function) {
    //         input = keystroke
    //             .key_char
    //             .as_ref()
    //             .filter(|text| !text.is_empty())
    //             .map(|text| {
    //                 let mut bytes = Vec::with_capacity(text.len() + 1);
    //                 bytes.push(0x1b);
    //                 bytes.extend_from_slice(text.as_bytes());
    //                 bytes
    //             });
    //     }

    //     input
    // }
}

impl IntoElement for TerminalElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}
pub fn terminal_text_style() -> TextStyle {
    TextStyle {
        // 颜色：常用终端的经典绿/白/灰，这里用浅灰色作为示例
        color: hsla(0.0, 0.0, 0.9, 1.0), // 90% 亮度的白色

        // 字体系列：优先使用 Maple Mono NF CN，并设置合适的回退
        font_family: "Maple Mono NF CN".into(),
        font_features: FontFeatures::default(),

        // 回退字体：保证中文和特殊字符正常显示
        font_fallbacks: None,

        // 字体大小：终端常用 14px
        font_size: px(16.0).into(),

        // 行高：终端通常用 1.2 ~ 1.4 倍字号，这里用 1.3
        line_height: DefiniteLength::Fraction(1.3),

        // 字重：终端一般用 Regular (400)
        font_weight: FontWeight::NORMAL,

        // 字体样式：标准，无斜体
        font_style: FontStyle::Normal,

        // 背景色：终端一般由终端背景色单独控制，这里留空
        background_color: None,

        // 无下划线（除非特定场景，如 git 状态）
        underline: None,

        // 无删除线
        strikethrough: None,

        // 终端通常保留空白符，用于对齐
        white_space: WhiteSpace::Normal,

        // 终端通常不截断文本，依赖滚动
        text_overflow: None,

        // 终端文本一般左对齐
        text_align: TextAlign::Left,

        // 不限制行数，显示所有内容
        line_clamp: None,
    }
}
impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = MyPaintState;

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    // 这是第一阶段，负责计算你的元素及其子元素的大小和位置。
    // 通过 window.request_layout() 向 GPUI 的布局引擎（Taffy）注册布局信息
    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut style = gpui::Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();

        (window.request_layout(style, None, cx), ())
    }

    //   这是第二阶段，负责创建交互区域（Hitbox）并准备绘制所需的数据。
    //   此时你的元素已经获得了由父元素分配的最终位置和大小（bounds 参数）
    fn prepaint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>, // 这就是你的元素最终在屏幕上的位置和大小
        request_layout: &mut Self::RequestLayoutState, // 来自 request_layout 的数据,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> Self::PrepaintState {
        self.interactivity.prepaint(
            id,
            inspector_id,
            bounds,
            bounds.size,
            window,
            cx,
            |_, _, hitbox, window, cx| {
                // let hitbox = Hitbox {
                //     id: (),
                //     bounds,
                //     content_mask: (),
                //     behavior: (),
                // };
                let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
                // let settings = ThemeSettings::get_global(cx).clone();

                // let buffer_font_size = settings.buffer_font_size(cx);

                // let terminal_settings = TerminalSettings::get_global(cx);
                // let minimum_contrast = terminal_settings.minimum_contrast;

                // let font_family = terminal_settings.font_family.as_ref().map_or_else(
                //     || settings.buffer_font.family.clone(),
                //     |font_family| font_family.0.clone().into(),
                // );
                // let font_family=

                // let font_fallbacks = terminal_settings
                //     .font_fallbacks
                //     .as_ref()
                //     .or(settings.buffer_font.fallbacks.as_ref())
                //     .cloned();

                // let font_features = terminal_settings
                //     .font_features
                //     .as_ref()
                //     .unwrap_or(&FontFeatures::disable_ligatures())
                //     .clone();

                // let font_weight = terminal_settings.font_weight.unwrap_or_default();

                // let line_height = terminal_settings.line_height.value();

                // let font_size = match &self.mode {
                //     TerminalMode::Embedded { .. } => {
                //         window.text_style().font_size.to_pixels(window.rem_size())
                //     }
                //     TerminalMode::Standalone => terminal_settings
                //         .font_size
                //         .map_or(buffer_font_size, |size| {
                //             theme_settings::adjusted_font_size(size, cx)
                //         }),
                // };

                // let theme = cx.theme().clone();

                let link_style = HighlightStyle {
                    color: Some(Hsla {
                        h: 0.,
                        s: 0.,
                        l: 0.,
                        a: 0.,
                    }),
                    font_weight: Some(FontWeight::NORMAL),
                    font_style: None,
                    background_color: None,
                    underline: Some(UnderlineStyle {
                        thickness: px(1.0),
                        color: Some(Hsla {
                            h: 0.,
                            s: 0.,
                            l: 0.,
                            a: 0.,
                        }),
                        wavy: false,
                    }),
                    strikethrough: None,
                    fade_out: None,
                };

                let text_style = terminal_text_style();

                let line_height = 1.5;
                let text_system = cx.text_system();
                // let player_color = theme.players().local();
                // let match_color = theme.colors().search_match_background;
                let gutter;
                let (dimensions, line_height_px) = {
                    let rem_size = window.rem_size();
                    let font_pixels = text_style.font_size.to_pixels(rem_size);
                    let line_height = f32::from(font_pixels) * line_height;
                    let font_id = cx.text_system().resolve_font(&text_style.font());

                    let cell_width = text_system
                        .advance(font_id, font_pixels, 'm')
                        .unwrap()
                        .width;
                    gutter = cell_width;

                    let mut size = bounds.size;
                    size.width -= gutter;
                    let available_height = size.height;

                    // https://github.com/zed-industries/zed/issues/2750
                    // if the terminal is one column wide, rendering 🦀
                    // causes alacritty to misbehave.
                    if size.width < cell_width * 2.0 {
                        size.width = cell_width * 2.0;
                    }

                    let mut origin = bounds.origin;
                    origin.x += gutter;

                    // if matches!(self.terminal_view.read(cx).mode, TerminalMode::Standalone) {
                    //     let should_anchor_to_bottom = {
                    //         let content = self.terminal.read(cx).last_content();
                    //         content.mode.contains(Modes::ALT_SCREEN)
                    //             || (content.scrolled_to_bottom && content.bottom_row_occupied)
                    //     };
                    //     let scale_factor = window.scale_factor();
                    //     let line_height_pixels = px(line_height);
                    //     let line_height_device_px = (f32::from(line_height_pixels) * scale_factor)
                    //         .round()
                    //         .max(1.0) as i32;
                    //     let available_height_device_px =
                    //         (f32::from(available_height) * scale_factor)
                    //             .floor()
                    //             .max(0.0) as i32;

                    //     let rows =
                    //         ((available_height_device_px / line_height_device_px) as usize).max(1);
                    //     let snapped_height_device_px = (rows as i32) * line_height_device_px;
                    //     let padding_device_px =
                    //         (available_height_device_px - snapped_height_device_px).max(0);

                    //     let snapped_height =
                    //         px(snapped_height_device_px as f32 / scale_factor.max(1.0));
                    //     let padding = px(padding_device_px as f32 / scale_factor.max(1.0));

                    //     size.height = snapped_height;
                    //     if should_anchor_to_bottom {
                    //         origin.y += padding;
                    //     }
                    // }

                    // Snap to device pixels to avoid subpixel jitter while resizing.
                    // Terminal rendering is grid-based; allowing fractional origins can cause the
                    // glyph rasterization to shift between frames, which looks like flicker.
                    let scale_factor = window.scale_factor();
                    let snap_px = |value: Pixels| {
                        Pixels::from((f32::from(value) * scale_factor).floor() / scale_factor)
                    };
                    origin.x = snap_px(origin.x);
                    origin.y = snap_px(origin.y);

                    (
                        TerminalBounds::new(px(line_height), cell_width, Bounds { origin, size }),
                        line_height,
                    )
                };

                // let search_matches = self.terminal.read(cx).matches.clone();

                let background_color = Hsla::black();

                // let (last_hovered_word, hover_tooltip) =
                self.terminal.update(cx, |terminal, cx| {
                    // terminal.set_size(dimensions);
                    terminal.sync(window, cx);

                    // (None, None)
                    // if window.modifiers().secondary()
                    //     && bounds.contains(&window.mouse_position())
                    //     && self.terminal_view.read(cx).hover.is_some()
                    // {
                    //     let registered_hover = self.terminal_view.read(cx).hover.as_ref();
                    //     if terminal.last_content.last_hovered_word.as_ref()
                    //         == registered_hover.map(|hover| &hover.hovered_word)
                    //     {
                    //         (
                    //             terminal.last_content.last_hovered_word.clone(),
                    //             registered_hover.map(|hover| hover.tooltip.clone()),
                    //         )
                    //     } else {
                    //         (None, None)
                    //     }
                    // } else {
                    //     (None, None)
                    // }
                });

                // let scroll_top = self.terminal_view.read(cx).scroll_top;
                // let hyperlink_tooltip = hover_tooltip.map(|hover_tooltip| {
                //     let offset = dimensions.bounds.origin - point(px(0.), scroll_top);
                //     let mut element = div()
                //         .size_full()
                //         .id("terminal-element")
                //         .tooltip(Tooltip::text(hover_tooltip))
                //         .into_any_element();
                //     element.prepaint_as_root(offset, bounds.size.into(), window, cx);
                //     element
                // });
                let Content {
                    cells,
                    // mode,
                    display_offset,
                    cursor_char,
                    // selection,
                    cursor,
                    ..
                } = &self.terminal.read(cx).last_content;

                // let mode = *mode;
                // let display_offset = *display_offset;

                // // searches, highlights to a single range representations
                // let mut relative_highlighted_ranges = Vec::new();
                // for search_match in search_matches {
                //     relative_highlighted_ranges.push((search_match, match_color))
                // }
                // if let Some(selection) = selection {
                //     relative_highlighted_ranges
                //         .push((selection.point_range(), player_color.selection));
                // }

                // // then have that representation be converted to the appropriate highlight data structure

                // let content_mode = self.terminal_view.read(cx).content_mode(window, cx);

                // Calculate the intersection of the terminal's bounds with the current
                // content mask (the visible viewport after all parent clipping).
                // This allows us to only render cells that are actually visible, which is
                // critical for performance when terminals are inside scrollable containers
                // like the Agent Panel thread view.
                //
                // This optimization is analogous to the editor optimization in PR #45077
                // which fixed performance issues with large AutoHeight editors inside Lists.
                let content_bounds = dimensions.bounds;
                let visible_bounds = window.content_mask().bounds;
                let intersection = visible_bounds.intersect(&content_bounds);

                // If the terminal is entirely outside the viewport, skip all cell processing.
                // This handles the case where the terminal has been scrolled past (above or
                // below the viewport), similar to the editor fix in PR #45077 where start_row
                // could exceed max_row when the editor was positioned above the viewport.
                let (rects, batched_text_runs, block_element_rects) = if intersection.size.height
                    <= px(0.)
                    || intersection.size.width <= px(0.)
                {
                    (Vec::new(), Vec::new(), Vec::new())
                } else if intersection == content_bounds {
                    // 用于处理终端内容完全可见、无需裁剪
                    // Fast path: terminal fully visible, no clipping needed.
                    // Avoid grouping/allocation overhead by streaming cells directly.

                    TerminalElement::layout_grid(cells.iter(), 0, &text_style, None, 0., cx)
                } else {
                    // Calculate which screen rows are visible based on pixel positions.
                    // This works for both Scrollable and Inline modes because we filter
                    // by screen position (enumerated line group index), not by the cell's
                    // internal line number (which can be negative in Scrollable mode for
                    // scrollback history).
                    let rows_above_viewport = f32::from(
                        (intersection.top() - content_bounds.top()).max(px(0.)) / line_height_px,
                    ) as usize;
                    let visible_row_count =
                        f32::from((intersection.size.height / line_height_px).ceil()) as usize + 1;

                    TerminalElement::layout_grid(
                        // Group cells by line and filter to only the visible screen rows.
                        // skip() and take() work on enumerated line groups (screen position),
                        // making this work regardless of the actual cell.point.line values.
                        cells
                            .iter()
                            .chunk_by(|c| c.point.line)
                            .into_iter()
                            .skip(rows_above_viewport)
                            .take(visible_row_count)
                            .flat_map(|(_, line_cells)| line_cells),
                        rows_above_viewport as i32,
                        &text_style,
                        None,
                        0.,
                        cx,
                    )
                };

                // Layout cursor. Rectangle is used for IME, so we should lay it out even
                // if we don't end up showing it.
                let cursor_point = DisplayCursor::from(cursor.point, *display_offset);
                let cursor_text = {
                    let cursor_text = cursor_char.to_string();
                    let len = cursor_text.len();
                    window.text_system().shape_line(
                        cursor_text.into(),
                        text_style.font_size.to_pixels(window.rem_size()),
                        &[TextRun {
                            len,
                            font: text_style.font(),
                            color: hsla(220., 15., 10., 1.),
                            ..Default::default()
                        }],
                        None,
                    )
                };

                // For whitespace, use cell width to avoid cursor stretching.
                // For other characters, use the larger of shaped width and cell width
                // to properly cover wide characters like emojis.
                let cursor_width = if cursor_char.is_whitespace() {
                    dimensions.cell_width()
                } else {
                    cursor_text.width.max(dimensions.cell_width())
                };

                let ime_cursor_bounds = TerminalElement::cursor_position(cursor_point, dimensions)
                    .map(|cursor_position| Bounds {
                        origin: cursor_position,
                        size: size(cursor_width.ceil(), dimensions.line_height),
                    });

                let cursor = if matches!(cursor.shape, CursorShape::Hidden) {
                    None
                } else {
                    let cursor_shape = if self.focused {
                        cursor.shape
                    } else {
                        CursorShape::HollowBlock
                    };
                    let cursor_text =
                        matches!(cursor_shape, CursorShape::Block).then_some(cursor_text);

                    ime_cursor_bounds.map(move |bounds| {
                        CursorLayout::new(
                            bounds.origin,
                            bounds.size.width,
                            bounds.size.height,
                            hsla(0., 0., 0.85, 1.),
                            cursor_shape,
                            cursor_text,
                        )
                    })
                };

                let block_below_cursor_element = if let Some(block) = &self.block_below_cursor {
                    let terminal = self.terminal.read(cx);
                    if terminal.last_content.display_offset == 0 {
                        // let target_line = terminal.last_content.cursor.point.line + 1;
                        let target_line = 1;
                        let render = &block.render;
                        let mut block_cx = BlockContext {
                            window,
                            context: cx,
                            dimensions,
                        };
                        let element = render(&mut block_cx);
                        let mut element = div().occlude().child(element).into_any_element();
                        let available_space = size(
                            AvailableSpace::Definite(dimensions.width() + gutter),
                            AvailableSpace::Definite(
                                block.height as f32 * dimensions.line_height(),
                            ),
                        );
                        let origin = GpuiPoint::new(bounds.origin.x, dimensions.bounds.origin.y)
                            + point(px(0.), target_line as f32 * dimensions.line_height())
                            - point(px(0.), px(0.));
                        // window.with_rem_size(rem_size, |window| {
                        //     element.prepaint_as_root(origin, available_space, window, cx);
                        // });
                        Some(element)
                    } else {
                        None
                    }
                } else {
                    None
                };
                MyPaintState {
                    hitbox,
                    batched_text_runs: batched_text_runs,
                    block_element_rects,
                    rects,
                    relative_highlighted_ranges: vec![],
                    cursor,
                    ime_cursor_bounds,
                    background_color,
                    dimensions,
                    mode: Modes::empty(),
                    display_offset: 0,
                    hyperlink_tooltip: None,
                    block_below_cursor_element: block_below_cursor_element,
                    base_text_style: text_style,
                }
            },
        )
    }

    // 这是第三阶段，也是最终绘制像素和绑定交互逻辑的地方。
    // 在这里，你可以调用 window.paint_quad 等方法绘制图形，并通过 window.on_mouse_event 等监听事件。
    fn paint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        layout: &mut Self::PrepaintState,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) {
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            // let scroll_top = self.terminal_view.read(cx).scroll_top;
            // 画背景
            window.paint_quad(fill(bounds, layout.background_color));

            let origin = layout.dimensions.bounds.origin - GpuiPoint::new(px(0.), px(0.));
            let scale_factor = window.scale_factor();
            // 像素对齐函数  避免 10.3px 这种非整数物理像素的坐标，导致文字或图形在屏幕上显示模糊（亚像素渲染）
            let snap_px = |value: Pixels| {
                Pixels::from((f32::from(value) * scale_factor).floor() / scale_factor)
            };
            let origin = point(snap_px(origin.x), snap_px(origin.y));

            // 获取当前输入法正在输入的、但还未确认的文本，
            let marked_text_cloned: Option<String> = {
                let ime_state = &self.terminal_view.read(cx).ime_state;
                ime_state.as_ref().map(|state| state.marked_text.clone())
            };

            let terminal_input_handler = TerminalInputHandler {
                terminal_view: self.terminal_view.clone(),
                cursor_bounds: layout.ime_cursor_bounds.map(|bounds| bounds + origin),
                // workspace: self.workspace.clone(),
            };

            // self.register_mouse_listeners(layout.mode, &layout.hitbox, window);
            // if window.modifiers().secondary()
            //     && bounds.contains(&window.mouse_position())
            //     && self.terminal_view.read(cx).hover.is_some()
            // {
            //     window.set_cursor_style(gpui::CursorStyle::GpuiPointingHand, &layout.hitbox);
            // } else {
            //     window.set_cursor_style(gpui::CursorStyle::IBeam, &layout.hitbox);
            // }

            let original_cursor = layout.cursor.take();
            let hyperlink_tooltip = layout.hyperlink_tooltip.take();
            let block_below_cursor_element = layout.block_below_cursor_element.take();
            let focused = self.focus.is_focused(window);
            let cursor_blink_visible = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| (duration.as_millis() / 500) % 2 == 0)
                .unwrap_or(true);
            if focused && marked_text_cloned.is_none() {
                window.request_animation_frame();
            }

            self.register_mouse_listeners(layout.mode, &layout.hitbox, window);
            self.interactivity.paint(
                id,
                inspector_id,
                bounds,
                Some(&layout.hitbox),
                window,
                cx,
                |_, window, cx| {
                    window.handle_input(&self.focus, terminal_input_handler, cx);
                    window.on_key_event({
                        let terminal = self.terminal.clone();
                        let focus = self.focus.clone();
                        move |event: &KeyDownEvent, phase, window, cx| {
                            if phase != DispatchPhase::Bubble || !focus.is_focused(window) {
                                return;
                            }

                            log::debug!(
                                "Key event received: key={:?}, modifiers={:?}",
                                event.keystroke.key,
                                event.keystroke.modifiers
                            );
                            // if let Some(input) =
                            // TerminalElement::input_for_keystroke(&event.keystroke)
                            // {
                            //     log::debug!("Converted key to input: {:?}", input);
                            //     window.prevent_default();
                            //     cx.stop_propagation();
                            //     terminal.update(cx, |terminal, cx| {
                            //         // terminal.write_input(input);
                            //         cx.notify();
                            //     });
                            // }
                        }
                    });

                    window.on_key_event({
                        let this = self.terminal.clone();
                        let focus = self.focus.clone();
                        move |event: &ModifiersChangedEvent, phase, window, cx| {
                            if phase != DispatchPhase::Bubble || !focus.is_focused(window) {
                                return;
                            }

                            this.update(cx, |term, cx| {
                                term.try_modifiers_change(&event.modifiers, window, cx)
                            });
                        }
                    });

                    for rect in &layout.rects {
                        rect.paint(origin, &layout.dimensions, window);
                    }

                    log::debug!("1dsf23123123iasasfsdf");
                    // for (relative_highlighted_range, color) in &layout.relative_highlighted_ranges {
                    //     if let Some((start_y, highlighted_range_lines)) =
                    //         to_highlighted_range_lines(relative_highlighted_range, layout, origin)
                    //     {
                    //         let corner_radius = if EditorSettings::get_global(cx).rounded_selection
                    //         {
                    //             0.15 * layout.dimensions.line_height
                    //         } else {
                    //             Pixels::ZERO
                    //         };
                    //         let hr = HighlightedRange {
                    //             start_y,
                    //             line_height: layout.dimensions.line_height,
                    //             lines: highlighted_range_lines,
                    //             color: *color,
                    //             corner_radius: corner_radius,
                    //         };
                    //         hr.paint(true, bounds, window);
                    //     }
                    // }

                    // Paint batched text runs instead of individual cells
                    let text_paint_start = Instant::now();

                    for batch in &layout.batched_text_runs {
                        batch.paint(origin, &layout.dimensions, window, cx);
                    }

                    for batch in &layout.batched_text_runs {
                        for block_element_rect in &layout.block_element_rects {
                            block_element_rect.paint(origin, &layout.dimensions, window);
                        }

                        let text_paint_time = text_paint_start.elapsed();

                        if let Some(text_to_mark) = &marked_text_cloned
                            && !text_to_mark.is_empty()
                            && let Some(ime_bounds) = layout.ime_cursor_bounds
                        {
                            let ime_position = (ime_bounds + origin).origin;
                            let mut ime_style = layout.base_text_style.clone();
                            ime_style.underline = Some(UnderlineStyle {
                                color: Some(ime_style.color),
                                thickness: px(1.0),
                                wavy: false,
                            });

                            let shaped_line = window.text_system().shape_line(
                                text_to_mark.clone().into(),
                                ime_style.font_size.to_pixels(window.rem_size()),
                                &[TextRun {
                                    len: text_to_mark.len(),
                                    font: ime_style.font(),
                                    color: ime_style.color,
                                    underline: ime_style.underline,
                                    ..Default::default()
                                }],
                                None,
                            );

                            // Paint background to cover terminal text behind marked text
                            let ime_background_bounds = Bounds::new(
                                ime_position,
                                size(shaped_line.width, layout.dimensions.line_height),
                            );
                            window.paint_quad(fill(ime_background_bounds, layout.background_color));

                            shaped_line
                                .paint(
                                    ime_position,
                                    layout.dimensions.line_height,
                                    gpui::TextAlign::Left,
                                    None,
                                    window,
                                    cx,
                                )
                                .unwrap();
                        }

                        if self.cursor_visible
                            && marked_text_cloned.is_none()
                            && (!focused || cursor_blink_visible)
                            && let Some(cursor) = &original_cursor
                        {
                            cursor.paint(origin, window, cx);
                        }

                        // if let Some(mut element) = block_below_cursor_element {
                        //     element.paint(window, cx);
                        // }

                        // if let Some(mut element) = hyperlink_tooltip {
                        //     element.paint(window, cx);
                        // }
                    }
                },
            );
        });
    }
}

struct TerminalInputHandler {
    terminal_view: Entity<TerminalView>,
    // workspace: WeakEntity<Workspace>,
    cursor_bounds: Option<Bounds<Pixels>>,
}

impl InputHandler for TerminalInputHandler {
    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _: &mut Window,
        _cx: &mut App,
    ) -> Option<UTF16Selection> {
        // Always return a valid selection for IME positioning,
        // even in ALT_SCREEN mode (fullscreen TUI apps like opencode, vim, etc.)
        // The terminal still has a cursor position that should be used for IME candidate window placement.
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(
        &mut self,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<std::ops::Range<usize>> {
        self.terminal_view.read(cx).marked_text_range()
    }

    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        adjusted_range: &mut Option<std::ops::Range<usize>>,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        let marked_text = self
            .terminal_view
            .read(cx)
            .ime_state
            .as_ref()
            .map(|state| state.marked_text.clone())?;

        let utf16_len = marked_text.encode_utf16().count();
        let start = range_utf16.start.min(utf16_len);
        let end = range_utf16.end.min(utf16_len);
        *adjusted_range = Some(start..end);

        Some(slice_utf16(&marked_text, start..end))
    }

    fn replace_text_in_range(
        &mut self,
        _replacement_range: Option<std::ops::Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.terminal_view.update(cx, |view, view_cx| {
            view.commit_text(text, view_cx);
        });
        window.invalidate_character_coordinates();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<std::ops::Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.terminal_view.update(cx, |view, view_cx| {
            view.set_marked_text(new_text.to_string(), view_cx);
        });
        window.invalidate_character_coordinates();
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut App) {
        self.terminal_view.update(cx, |view, view_cx| {
            view.clear_marked_text(view_cx);
        });
        window.invalidate_character_coordinates();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        let term_bounds = self.terminal_view.read(cx).terminal_bounds(cx);

        let mut bounds = self.cursor_bounds?;
        let offset_x = term_bounds.cell_width * range_utf16.start as f32;
        bounds.origin.x += offset_x;

        Some(bounds)
    }

    fn apple_press_and_hold_enabled(&mut self) -> bool {
        false
    }

    fn character_index_for_point(
        &mut self,
        point: GpuiPoint<Pixels>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        let marked_len = self
            .terminal_view
            .read(cx)
            .ime_state
            .as_ref()
            .map(|state| state.marked_text.encode_utf16().count())
            .unwrap_or_default();

        let cursor_bounds = self.cursor_bounds?;
        if point.x <= cursor_bounds.origin.x {
            Some(0)
        } else {
            Some(marked_len)
        }
    }
}

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
pub struct MyPaintState {
    // ... 你的数据，比如字符网格、颜色等
    hitbox: Hitbox,
    batched_text_runs: Vec<BatchedTextRun>,
    block_element_rects: Vec<BlockElementLayoutRect>,
    rects: Vec<LayoutRect>,
    relative_highlighted_ranges: Vec<(Range, Hsla)>,
    cursor: Option<CursorLayout>,
    ime_cursor_bounds: Option<Bounds<Pixels>>,
    background_color: Hsla,
    dimensions: TerminalBounds,
    mode: Modes,
    display_offset: usize,
    hyperlink_tooltip: Option<AnyElement>,
    block_below_cursor_element: Option<AnyElement>,
    base_text_style: TextStyle,
}

#[derive(Clone, Debug)]
pub struct BlockElementLayoutRect {
    point: LayoutPoint,
    num_of_columns: usize,
    num_of_lines: usize,
    color: Hsla,
}

impl BlockElementLayoutRect {
    fn new(point: LayoutPoint, num_of_columns: usize, num_of_lines: usize, color: Hsla) -> Self {
        Self {
            point,
            num_of_columns,
            num_of_lines,
            color,
        }
    }

    pub fn paint(
        &self,
        origin: GpuiPoint<Pixels>,
        dimensions: &TerminalBounds,
        window: &mut Window,
    ) {
        let subcell_width = dimensions.cell_width / BLOCK_SUBCELL_COLUMNS as f32;
        let subcell_height = dimensions.line_height / BLOCK_SUBCELL_LINES as f32;
        let position = point(
            origin.x + self.point.column as f32 * subcell_width,
            origin.y + self.point.line as f32 * subcell_height,
        );
        let size = size(
            subcell_width * self.num_of_columns as f32,
            subcell_height * self.num_of_lines as f32,
        );

        window.paint_quad(fill(Bounds::new(position, size), self.color));
    }

    pub fn line(&self) -> i32 {
        (self.point.line + self.num_of_lines as i32 - 1) / BLOCK_SUBCELL_LINES
    }
}
const BLOCK_SUBCELL_COLUMNS: i32 = 8;
const BLOCK_SUBCELL_LINES: i32 = 24;

#[derive(Copy, Clone, Debug, Default)]
pub struct LayoutPoint {
    pub line: i32,
    pub column: i32,
}

impl LayoutPoint {
    pub fn new(line: i32, column: i32) -> Self {
        Self { line, column }
    }

    pub fn line(&self) -> i32 {
        self.line
    }

    pub fn column(&self) -> i32 {
        self.column
    }
}

#[derive(Clone, Debug, Default)]
pub struct LayoutRect {
    point: LayoutPoint,
    num_of_cells: usize,
    color: Hsla,
}

impl LayoutRect {
    pub fn new(point: LayoutPoint, num_of_cells: usize, color: Hsla) -> LayoutRect {
        LayoutRect {
            point,
            num_of_cells,
            color,
        }
    }

    pub fn paint(
        &self,
        origin: GpuiPoint<Pixels>,
        dimensions: &TerminalBounds,
        window: &mut Window,
    ) {
        let position = {
            let layout_point = self.point;
            point(
                (origin.x + layout_point.column as f32 * dimensions.cell_width).floor(),
                origin.y + layout_point.line as f32 * dimensions.line_height,
            )
        };
        let size = point(
            (dimensions.cell_width * self.num_of_cells as f32).ceil(),
            dimensions.line_height,
        )
        .into();

        window.paint_quad(fill(Bounds::new(position, size), self.color));
    }
}

/// Represents a rectangular region with a specific color on a logical grid.
#[derive(Debug, Clone)]
struct BackgroundRegion {
    start_line: i32,
    start_col: i32,
    end_line: i32,
    end_col: i32,
    color: Hsla,
}

impl BackgroundRegion {
    fn new(line: i32, col: i32, color: Hsla) -> Self {
        BackgroundRegion {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
            color,
        }
    }

    fn with_extents(
        start_line: i32,
        start_col: i32,
        end_line: i32,
        end_col: i32,
        color: Hsla,
    ) -> Self {
        BackgroundRegion {
            start_line,
            start_col,
            end_line,
            end_col,
            color,
        }
    }

    /// Check if this region can be merged with another region
    fn can_merge_with(&self, other: &BackgroundRegion) -> bool {
        if self.color != other.color {
            return false;
        }

        // Check if regions are adjacent horizontally
        if self.start_line == other.start_line && self.end_line == other.end_line {
            return self.end_col + 1 == other.start_col || other.end_col + 1 == self.start_col;
        }

        // Check if regions are adjacent vertically with same column span
        if self.start_col == other.start_col && self.end_col == other.end_col {
            return self.end_line + 1 == other.start_line || other.end_line + 1 == self.start_line;
        }

        false
    }

    /// Merge this region with another region
    fn merge_with(&mut self, other: &BackgroundRegion) {
        self.start_line = self.start_line.min(other.start_line);
        self.start_col = self.start_col.min(other.start_col);
        self.end_line = self.end_line.max(other.end_line);
        self.end_col = self.end_col.max(other.end_col);
    }
}

pub trait TerminalLayoutCell {
    fn point(&self) -> Point;
    fn cell(&self) -> &Cell;
}

impl TerminalLayoutCell for IndexedCell {
    fn point(&self) -> Point {
        self.point
    }

    fn cell(&self) -> &Cell {
        &self.cell
    }
}

impl TerminalLayoutCell for &IndexedCell {
    fn point(&self) -> Point {
        self.point
    }

    fn cell(&self) -> &Cell {
        &self.cell
    }
}

/// Merge grid regions to minimize the number of rectangles.
fn merge_background_regions(regions: Vec<BackgroundRegion>) -> Vec<BackgroundRegion> {
    if regions.is_empty() {
        return regions;
    }

    let mut merged = regions;
    let mut changed = true;

    // Keep merging until no more merges are possible
    while changed {
        changed = false;
        let mut i = 0;

        while i < merged.len() {
            let mut j = i + 1;
            while j < merged.len() {
                if merged[i].can_merge_with(&merged[j]) {
                    let other = merged.remove(j);
                    merged[i].merge_with(&other);
                    changed = true;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    merged
}

/// A batched text run that combines multiple adjacent cells with the same style
#[derive(Debug)]
pub struct BatchedTextRun {
    pub start_point: LayoutPoint,
    pub text: String,
    pub cell_count: usize,
    pub style: TextRun,
    pub font_size: AbsoluteLength,
}

impl BatchedTextRun {
    fn new_from_char(
        start_point: LayoutPoint,
        c: char,
        style: TextRun,
        font_size: AbsoluteLength,
    ) -> Self {
        let mut text = String::with_capacity(100); // Pre-allocate for typical line length
        text.push(c);
        BatchedTextRun {
            start_point,
            text,
            cell_count: 1,
            style,
            font_size,
        }
    }

    fn can_append(&self, other_style: &TextRun) -> bool {
        self.style.font == other_style.font
            && self.style.color == other_style.color
            && self.style.background_color == other_style.background_color
            && self.style.underline == other_style.underline
            && self.style.strikethrough == other_style.strikethrough
    }

    fn append_char(&mut self, c: char) {
        self.append_char_internal(c, true);
    }

    fn append_zero_width_chars(&mut self, chars: &[char]) {
        for &c in chars {
            self.append_char_internal(c, false);
        }
    }

    fn append_char_internal(&mut self, c: char, counts_cell: bool) {
        self.text.push(c);
        if counts_cell {
            self.cell_count += 1;
        }
        self.style.len += c.len_utf8();
    }

    pub fn paint(
        &self,
        origin: GpuiPoint<Pixels>,
        dimensions: &TerminalBounds,
        window: &mut Window,
        cx: &mut App,
    ) {
        let pos = GpuiPoint::new(
            origin.x + self.start_point.column as f32 * dimensions.cell_width,
            origin.y + self.start_point.line as f32 * dimensions.line_height,
        );
        let a = TextRun {
            len: self.text.len(),
            font: font("Maple Mono NF CN"),
            color: Hsla::white(),
            ..Default::default()
        };

        if let Err(e) = window
            .text_system()
            .shape_line(
                self.text.clone().into(),
                self.font_size.to_pixels(window.rem_size()),
                std::slice::from_ref(&a),
                Some(dimensions.cell_width),
            )
            .paint(
                pos,
                dimensions.line_height,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            )
        {
            log::debug!("打印错误日志");
        }
        log::debug!("打印错误日志");
    }
}
pub struct CursorLayout {
    origin: GpuiPoint<Pixels>,
    block_width: Pixels,
    line_height: Pixels,
    color: Hsla,
    shape: CursorShape,
    block_text: Option<ShapedLine>,
    cursor_name: Option<AnyElement>,
}

impl CursorLayout {
    fn new(
        origin: GpuiPoint<Pixels>,
        block_width: Pixels,
        line_height: Pixels,
        color: Hsla,
        shape: CursorShape,
        block_text: Option<ShapedLine>,
    ) -> Self {
        Self {
            origin,
            block_width,
            line_height,
            color,
            shape,
            block_text,
            cursor_name: None,
        }
    }

    fn paint(&self, origin: GpuiPoint<Pixels>, window: &mut Window, cx: &mut App) {
        let cursor_origin = origin + self.origin;
        let thickness = px(2.);
        let bounds = match self.shape {
            CursorShape::Block => Bounds::new(
                cursor_origin,
                size(self.block_width.max(px(1.)), self.line_height),
            ),
            CursorShape::Underline => Bounds::new(
                point(
                    cursor_origin.x,
                    cursor_origin.y + self.line_height - thickness,
                ),
                size(self.block_width.max(px(1.)), thickness),
            ),
            CursorShape::Bar => Bounds::new(cursor_origin, size(thickness, self.line_height)),
            CursorShape::HollowBlock => {
                let full_bounds = Bounds::new(
                    cursor_origin,
                    size(self.block_width.max(px(1.)), self.line_height),
                );
                window.paint_quad(fill(
                    Bounds::new(full_bounds.origin, size(full_bounds.size.width, thickness)),
                    self.color,
                ));
                window.paint_quad(fill(
                    Bounds::new(
                        point(full_bounds.origin.x, full_bounds.bottom() - thickness),
                        size(full_bounds.size.width, thickness),
                    ),
                    self.color,
                ));
                window.paint_quad(fill(
                    Bounds::new(full_bounds.origin, size(thickness, full_bounds.size.height)),
                    self.color,
                ));
                window.paint_quad(fill(
                    Bounds::new(
                        point(full_bounds.right() - thickness, full_bounds.origin.y),
                        size(thickness, full_bounds.size.height),
                    ),
                    self.color,
                ));
                return;
            }
            CursorShape::Hidden => return,
        };

        window.paint_quad(fill(bounds, self.color));

        if let Some(block_text) = &self.block_text {
            let _ = block_text.paint(
                cursor_origin,
                self.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            );
        }
    }
}
