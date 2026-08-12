use gpui::*;
use gpui_component::button::Button;
use std::sync::Arc;

/// ButtonGroup 组件 - 单选按钮组

pub struct ButtonGroup<T: Clone + PartialEq> {
    /// 当前选中的值
    selected: Option<T>,
    /// 可选项列表
    options: Vec<T>,
    /// 获取显示文本的函数
    display_fn: Arc<dyn Fn(usize, &T) -> String>,
    /// 选中回调
    on_select: Option<Arc<dyn Fn(&T, &mut Window, &mut Context<ButtonGroup<T>>)>>,
}

impl<T: Clone + PartialEq + 'static> ButtonGroup<T> {
    /// 创建新的 ButtonGroup
    pub fn new(selected: Option<T>) -> Self {
        Self {
            selected,
            options: Vec::new(),
            display_fn: Arc::new(|idx, item| format!("{:?}", idx)),
            on_select: None,
        }
    }

    /// 方式1: 传入可迭代的选项数组
    pub fn options(mut self, items: impl IntoIterator<Item = T>) -> Self {
        self.options = items.into_iter().collect();
        self
    }

    /// 方式2: 使用自定义函数提取显示文本
    pub fn display_fn(mut self, f: impl Fn(usize, &T) -> String + 'static) -> Self {
        self.display_fn = Arc::new(f);
        self
    }

    /// 设置选中回调
    pub fn on_select(
        mut self,
        callback: impl Fn(&T, &mut Window, &mut Context<Self>) + 'static,
    ) -> Self {
        self.on_select = Some(Arc::new(callback));
        self
    }

    /// 获取当前选中的值
    pub fn selected(&self) -> Option<&T> {
        self.selected.as_ref()
    }

    /// 通过值来选中
    pub fn select(&mut self, value: Option<T>, cx: &mut Context<Self>) {
        if self.selected != value {
            self.selected = value;
            cx.notify();
        }
    }
}

impl<T: Clone + PartialEq + 'static> Render for ButtonGroup<T> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected.clone();
        let options = self.options.clone();
        let display_fn = self.display_fn.clone();
        let on_select = self.on_select.clone();

        div()
            .flex()
            .gap_1()
            .children(options.iter().enumerate().map(move |(index, item)| {
                let is_selected = selected.as_ref() == Some(item);
                let item = item.clone();
                let display_fn = display_fn.clone();
                let on_select = on_select.clone();

                let text = display_fn(index, &item);

                // 构建按钮样式
                let mut button = Button::new(format!("btn-group-{}", index)).label(text);

                if is_selected {
                    button = button
                        .bg(rgb(0x3b82f6)) // 蓝色选中背景
                        .text_color(rgb(0xffffff)) // 白色文字
                        .hover(|style| style.bg(rgb(0x2563eb)));
                } else {
                    button = button
                        .bg(rgb(0xe5e7eb)) // 灰色默认背景
                        .text_color(rgb(0x374151)) // 深色文字
                        .hover(|style| style.bg(rgb(0xd1d5db)));
                }

                // 圆角处理：第一个和最后一个按钮
                // if index == 0 && options.len() > 1 {
                //     button = button
                //         .rounded_l_2xl() // 左侧圆角
                //         .rounded_r_none();
                // } else if index == options.len() - 1 && options.len() > 1 {
                //     button = button
                //         .rounded_r_2xl() // 右侧圆角
                //         .rounded_l_none();
                // } else if options.len() == 1 {
                //     // 只有一个按钮，保持全圆角
                // } else {
                //     button = button.rounded_none();
                // }

                button.on_click(cx.listener({
                    let item = item.clone();
                    move |this,
                          _event: &ClickEvent,
                          window: &mut Window,
                          cx: &mut Context<ButtonGroup<T>>| {
                        this.selected = Some(item.clone());

                        // 调用回调
                        if let Some(ref callback) = on_select {
                            callback(&item, window, cx);
                        }

                        cx.notify();
                    }
                }))
            }))
    }
}
