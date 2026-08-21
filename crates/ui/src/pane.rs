use std::os::windows;

use gpui::{prelude::FluentBuilder, *};
use gpui_component::StyledExt;
use log::info;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneDirection {
    Horizontal,
    Vertical,
}

impl PaneDirection {
    fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }
}

pub struct Pane {
    style: StyleRefinement,

    content: Option<AnyElement>,
}

impl Pane {
    pub fn new() -> Self {
        let style = StyleRefinement::default().flex_1();
        Self {
            content: None,
            style,
        }
    }

    pub fn child<E>(mut self, child: E) -> Self
    where
        E: IntoElement,
    {
        self.content = Some(child.into_any_element());
        self
    }
}
impl Styled for Pane {
    #[doc = " Returns a reference to the style memory of this element."]
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
struct PaneState {
    /// 当前实际尺寸。
    size: Pixels,

    min_size: Option<Pixels>,

    max_size: Option<Pixels>,
}

impl PaneState {
    fn clamp_size(&self, size: Pixels) -> Pixels {
        let mut size = size;

        if let Some(min) = self.min_size {
            size = size.max(min);
        }

        if let Some(max) = self.max_size {
            size = size.min(max);
        }

        size
    }
}

#[derive(IntoElement)]
pub struct PaneLayout {
    direction: PaneDirection,

    panes: Vec<Pane>,

    states: Vec<PaneState>,

    bounds: Bounds<Pixels>,

    drag: Option<DragState>,
}
struct DragState {
    splitter_index: usize,

    start_position: Point<Pixels>,

    first_start_size: Pixels,

    second_start_size: Pixels,
}
impl PaneLayout {
    pub fn horizontal() -> Self {
        Self {
            direction: PaneDirection::Horizontal,
            panes: Vec::new(),
            states: Vec::new(),
            bounds: Bounds::default(),
            drag: None,
        }
    }

    pub fn vertical() -> Self {
        Self {
            direction: PaneDirection::Vertical,
            panes: Vec::new(),
            states: Vec::new(),
            bounds: Bounds::default(),
            drag: None,
        }
    }

    pub fn child(mut self, pane: Pane) -> Self {
        self.panes.push(pane);

        self.states.push(PaneState {
            size: px(0.0),
            min_size: None,
            max_size: None,
        });

        self
    }
    // fn initialize_sizes(&mut self, total_size: Pixels) {
    //     let splitter_count = self.panes.len().saturating_sub(1);

    //     let splitter_size = px(5.0);

    //     let available = total_size - splitter_size * splitter_count as f32;

    //     let mut fixed = px(0.0);
    //     let mut flex_total = 0.0;

    //     for pane in &self.panes {
    //         match pane.size {
    //             PaneSize::Fixed(size) => {
    //                 fixed += size;
    //             }

    //             PaneSize::Flex(flex) => {
    //                 flex_total += flex;
    //             }
    //         }
    //     }

    //     let remaining = (available - fixed).max(px(0.0));

    //     for (index, pane) in self.panes.iter().enumerate() {
    //         let size = match pane.size {
    //             PaneSize::Fixed(size) => size,

    //             PaneSize::Flex(flex) => {
    //                 if flex_total > 0.0 {
    //                     remaining * (flex / flex_total)
    //                 } else {
    //                     px(0.0)
    //                 }
    //             }
    //         };

    //         self.states[index].size = self.states[index].clamp_size(size);
    //     }
    // }
    fn start_drag(&mut self, index: usize, position: Point<Pixels>, cx: &mut Context<Self>) {
        if index + 1 >= self.states.len() {
            return;
        }

        self.drag = Some(DragState {
            splitter_index: index,

            start_position: position,

            first_start_size: self.states[index].size,

            second_start_size: self.states[index + 1].size,
        });

        cx.notify();
    }
    fn update_drag(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = &self.drag else {
            return;
        };

        let delta = if self.direction.is_horizontal() {
            position.x - drag.start_position.x
        } else {
            position.y - drag.start_position.y
        };

        let first_index = drag.splitter_index;
        let second_index = first_index + 1;

        let first_state = &self.states[first_index];
        let second_state = &self.states[second_index];

        let first_min = first_state.min_size.unwrap_or(px(0.0));

        let second_min = second_state.min_size.unwrap_or(px(0.0));

        let first_max = first_state.max_size.unwrap_or(Pixels::MAX);

        let second_max = second_state.max_size.unwrap_or(Pixels::MAX);

        let total = drag.first_start_size + drag.second_start_size;

        /*
         * first + second 必须保持不变。
         *
         * 例如：
         *
         * A = 400
         * B = 600
         *
         * 向右拖 100：
         *
         * A = 500
         * B = 500
         */
        let mut first = drag.first_start_size + delta;

        let mut second = drag.second_start_size - delta;

        /*
         * 先处理最小值。
         */
        if first < first_min {
            first = first_min;
            second = total - first;
        }

        if second < second_min {
            second = second_min;
            first = total - second;
        }

        /*
         * 再处理最大值。
         */
        if first > first_max {
            first = first_max;
            second = total - first;
        }

        if second > second_max {
            second = second_max;
            first = total - second;
        }

        /*
         * 最终再次 clamp。
         */
        first = first.max(first_min).min(first_max);
        second = second.max(second_min).min(second_max);

        self.states[first_index].size = first;
        self.states[second_index].size = second;

        cx.notify();
    }
    fn end_drag(&mut self, cx: &mut Context<Self>) {
        self.drag = None;
        cx.notify();
    }
}

struct PaneSplitter {
    index: usize,
    direction: PaneDirection,
}
struct SplitterState {
    should_move: bool,
}
impl PaneSplitter {
    fn new(index: usize, direction: PaneDirection) -> Self {
        Self { index, direction }
    }
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_state(cx, |_, _| SplitterState { should_move: false });
        let horizontal = self.direction.is_horizontal();

        // let layout = self.layout.clone();
        let index = self.index;

        div()
            .id(("pane-splitter", index))
            .flex_none()
            .when(horizontal, |this| {
                this.w(px(5.0)).h_full().cursor(CursorStyle::ResizeColumn)
            })
            .when(!horizontal, |this| {
                this.h(px(5.0)).w_full().cursor(CursorStyle::ResizeRow)
            })
            .bg(rgb(0x303030))
            .hover(|this| this.bg(rgb(0x505050)))
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&state, |state, _, _, _| {
                    state.should_move = true;
                }),
            )
            .on_mouse_down_out(window.listener_for(&state, |state, _, _, _| {
                state.should_move = false;
            }))
            .on_mouse_move(window.listener_for(&state, |state, e, _, _| {
                if state.should_move {
                    info!("move");
                }
            }))
            .on_drag_move(
                window.listener_for(&state, |state, e: &DragMoveEvent<()>, _, _| {
                    info!("drag move")
                }),
            )
    }
}

pub struct PaneLayoutState {}

impl RenderOnce for PaneLayout {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_state(cx, |_, _| PaneLayoutState {});
        let mut root = div().id("pane-layout").size_full().flex();

        root = match self.direction {
            PaneDirection::Horizontal => root.flex_row(),

            PaneDirection::Vertical => root.flex_col(),
        };

        // let layout_entity = cx.entity().clone();

        for index in 0..self.panes.len() {
            let size = self.states[index].size;

            let pane = self.panes[index].content.take();
            let style = self.panes[index].style();
            let pane_element = div()
                .flex_none()
                .when(self.direction.is_horizontal(), |this| this.w(size))
                .when(!self.direction.is_horizontal(), |this| this.h(size))
                .size_full()
                .refine_style(style)
                .children(pane);

            root = root.child(pane_element);

            /*
             * 自动插入 Splitter
             */
            if index + 1 < self.panes.len() {
                root = root.child(PaneSplitter::new(index, self.direction).render(window, cx));
            }
        }

        root
    }
}
