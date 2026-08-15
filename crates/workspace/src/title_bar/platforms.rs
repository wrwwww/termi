use gpui::{
    App, Hsla, MAX_BUTTONS_PER_SIDE, Pixels, Rgba, Window, WindowButton, WindowButtonLayout,
    WindowControlArea, div, prelude::*, px,
};
use theme::ActiveTheme;
use ui::h_flex;

#[derive(IntoElement)]
pub struct WindowsWindowControls {
    button_height: Pixels,
    layout: Option<WindowButtonLayout>, // 新增布局字段
}
impl WindowsWindowControls {
    // 只显示关闭按钮（Windows 风格）
    pub fn close_only(button_height: Pixels) -> Self {
        Self::new(
            button_height,
            Some(WindowButtonLayout {
                left: [None; MAX_BUTTONS_PER_SIDE],
                right: [Some(WindowButton::Close), None, None],
            }),
        )
    }

    // 标准 Windows 布局（最小化、最大化、关闭）
    pub fn windows_standard(button_height: Pixels) -> Self {
        Self::new(
            button_height,
            Some(WindowButtonLayout {
                left: [None; MAX_BUTTONS_PER_SIDE],
                right: [
                    Some(WindowButton::Minimize),
                    Some(WindowButton::Maximize),
                    Some(WindowButton::Close),
                ],
            }),
        )
    }
}
impl WindowsWindowControls {
    pub fn new(button_height: Pixels, layout: Option<WindowButtonLayout>) -> Self {
        Self {
            button_height,
            layout,
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_font() -> &'static str {
        "Segoe Fluent Icons"
    }

    #[cfg(target_os = "windows")]
    fn get_font() -> &'static str {
        use windows::Wdk::System::SystemServices::RtlGetVersion;

        let mut version = unsafe { std::mem::zeroed() };
        let status = unsafe { RtlGetVersion(&mut version) };

        if status.is_ok() && version.dwBuildNumber >= 22000 {
            "Segoe Fluent Icons"
        } else {
            "Segoe MDL2 Assets"
        }
    }
    // 辅助方法：将 WindowButton 转换为对应的按钮组件
    fn render_button(
        button: WindowButton,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<WindowsCaptionButton> {
        match button {
            WindowButton::Close => Some(WindowsCaptionButton::Close),
            WindowButton::Minimize => Some(WindowsCaptionButton::Minimize),
            WindowButton::Maximize => {
                if window.is_maximized() {
                    Some(WindowsCaptionButton::Restore)
                } else {
                    Some(WindowsCaptionButton::Maximize)
                }
            }
            // 如果有其他变体，处理它们
            _ => None,
        }
    }
}

impl RenderOnce for WindowsWindowControls {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut builder = div()
            .id("windows-window-controls")
            .font_family(Self::get_font())
            .flex()
            .flex_row()
            .justify_center()
            .content_stretch()
            .max_h(self.button_height)
            .min_h(self.button_height);
        // 渲染左侧按钮
        if let None = self.layout {
            self.layout = Some(WindowButtonLayout {
                left: [None; MAX_BUTTONS_PER_SIDE],
                right: [
                    Some(WindowButton::Minimize),
                    Some(WindowButton::Maximize),
                    Some(WindowButton::Close),
                ],
            });
        }
        if let Some(layout) = self.layout {
            for button_opt in layout.left.iter() {
                if let Some(button) = button_opt {
                    if let Some(caption_button) = Self::render_button(*button, window, cx) {
                        builder = builder.child(caption_button);
                    }
                }
            }

            // // 添加一个弹性间隔，让左右两侧按钮分开（可选）
            // builder = builder.flex_grow(1.0);

            // 渲染右侧按钮
            for button_opt in layout.right.iter() {
                if let Some(button) = button_opt {
                    if let Some(caption_button) = Self::render_button(*button, window, cx) {
                        builder = builder.child(caption_button);
                    }
                }
            }
        }

        // .child(WindowsCaptionButton::Minimize)
        // .map(|this| {
        //     this.child(if window.is_maximized() {
        //         WindowsCaptionButton::Restore
        //     } else {
        //         WindowsCaptionButton::Maximize
        //     })
        // })
        // .child(WindowsCaptionButton::Close)
        builder
    }
}

#[derive(IntoElement)]
enum WindowsCaptionButton {
    Minimize,
    Restore,
    Maximize,
    Close,
}

impl WindowsCaptionButton {
    #[inline]
    fn id(&self) -> &'static str {
        match self {
            Self::Minimize => "minimize",
            Self::Restore => "restore",
            Self::Maximize => "maximize",
            Self::Close => "close",
        }
    }

    #[inline]
    fn icon(&self) -> &'static str {
        match self {
            Self::Minimize => "\u{e921}",
            Self::Restore => "\u{e923}",
            Self::Maximize => "\u{e922}",
            Self::Close => "\u{e8bb}",
        }
    }

    #[inline]
    fn control_area(&self) -> WindowControlArea {
        match self {
            Self::Close => WindowControlArea::Close,
            Self::Maximize | Self::Restore => WindowControlArea::Max,
            Self::Minimize => WindowControlArea::Min,
        }
    }
}

impl RenderOnce for WindowsCaptionButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let enabled = match &self {
            Self::Minimize => window.is_minimizable(),
            Self::Restore | Self::Maximize => window.is_resizable(),
            Self::Close => true,
        };
        let (hover_bg, hover_fg, active_bg, active_fg) = match self {
            Self::Close => {
                let color: Hsla = Rgba {
                    r: 232.0 / 255.0,
                    g: 17.0 / 255.0,
                    b: 32.0 / 255.0,
                    a: 1.0,
                }
                .into();

                (
                    color,
                    gpui::white(),
                    color.opacity(0.8),
                    gpui::white().opacity(0.8),
                )
            }
            _ => (
                cx.theme().colors().ghost_element_hover,
                cx.theme().colors().text,
                cx.theme().colors().ghost_element_active,
                cx.theme().colors().text,
            ),
        };

        h_flex()
            .id(self.id())
            .justify_center()
            .content_center()
            .occlude()
            .w(px(36.))
            .h_full()
            .text_size(px(10.0))
            .when(!enabled, |style| {
                style.text_color(cx.theme().colors().text_disabled)
            })
            .when(enabled, |style| {
                style
                    .hover(|style| style.bg(hover_bg).text_color(hover_fg))
                    .active(|style| style.bg(active_bg).text_color(active_fg))
            })
            .window_control_area(self.control_area())
            .child(self.icon())
    }
}
