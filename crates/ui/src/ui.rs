use gpui::{
    App, Div, IntoElement, ParentElement, Rems, RenderOnce, SharedString, Styled, Window, div, rems,
};
use theme::ActiveTheme;

use crate::color::Color;
pub mod button;
pub mod color;
pub mod divider;
pub mod label;
pub mod scroll_bar;
pub mod typography;
/// The base size of a rem, in pixels.
pub const BASE_REM_SIZE_IN_PX: f32 = 16.;
#[inline(always)]
pub fn rems_from_px(px: impl Into<f32>) -> Rems {
    rems(px.into() / BASE_REM_SIZE_IN_PX)
}

/// Horizontally stacks elements. Sets `flex()`, `flex_row()`, `items_center()`
#[track_caller]
pub fn h_flex() -> Div {
    div().flex().flex_row().items_center()
}

/// Vertically stacks elements. Sets `flex()`, `flex_col()`
#[track_caller]
pub fn v_flex() -> Div {
    div().flex().flex_col()
}
