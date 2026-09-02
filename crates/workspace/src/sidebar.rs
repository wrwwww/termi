//! Sidebar
//!
//! 会话管理侧边栏：
//!
//! - 新建会话
//! - 搜索会话
//! - 分组展示
//! - 分组折叠
//! - 单击选中会话
//! - 双击打开终端
//! - 编辑会话
//! - 复制会话
//! - 删除会话
//! - 右键菜单
//!
//! 注意：
//! `SessionManager` 负责数据管理，Sidebar 只负责 UI 和用户交互。
//!
//! 当前代码假设 SessionManager 至少提供：
//!
//! - grouped_sessions()
//! - query()
//! - list()
//! - del_session()
//! - copy_session()
//! - open_session()
//! - save_session()
//!
//! 编辑窗口通过：
//!
//! ConnectionDialog::new(..., Some(session_id))
//!
//! 打开。

use std::collections::HashSet;

use crate::{
    EditAction,
    connection_dialog::ConnectionDialog,
    session_manager::SessionManager,
    state::AppState,
    terminal::{CloseTerminalAction, OpenTerminalAction},
};

use gpui::*;
use gpui_component::{
    IconName, Root,
    button::Button,
    input::{Input, InputState},
    menu::ContextMenuExt,
};

use log::{error, info};

use protocol::{Session, SessionId};
use schemars::JsonSchema;
use serde::Deserialize;

use theme::ActiveTheme;

// ============================================================================
// Actions
// ============================================================================

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema, Action)]
#[action(namespace = session_manager)]
pub struct CopyAction {
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema, Action)]
#[action(namespace = session_manager)]
pub struct DeleteAction {
    pub session_id: SessionId,
}

// ============================================================================
// Sidebar
// ============================================================================

pub struct Sidebar {
    /// 应用全局状态。
    state: Entity<AppState>,

    /// 哪些 group 当前处于折叠状态。
    collapsed: HashSet<String>,

    /// 搜索输入框。
    search_input: Entity<InputState>,

    /// SessionManager。
    session_manager: Entity<SessionManager>,

    /// Sidebar 自己的 focus。
    ///
    /// 这个东西非常重要：
    /// 右键打开 ContextMenu 时，焦点可能暂时离开 Sidebar。
    /// 所以我们在右键事件发生时主动重新 focus。
    focus: FocusHandle,

    /// 观察搜索输入框变化。
    ///
    /// InputState 改变后通知 Sidebar 重新 render。
    _search_subscription: Subscription,
}

impl Sidebar {
    pub fn new(
        state: Entity<AppState>,
        session_manager: Entity<SessionManager>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();

        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search sessions..."));

        let _search_subscription = cx.observe(&search_input, |_this, _input, cx| {
            cx.notify();
        });

        let mut collapsed = HashSet::new();

        // 你原来的逻辑默认折叠 Personal。
        collapsed.insert("Personal".to_string());

        Self {
            state,
            collapsed,
            search_input,
            session_manager,
            focus,
            _search_subscription,
        }
    }

    // ========================================================================
    // Search
    // ========================================================================

    fn search_query(&self, cx: &Context<Self>) -> String {
        self.search_input.read(cx).value().trim().to_lowercase()
    }

    // ========================================================================
    // Copy
    // ========================================================================

    pub fn copy_session(
        &mut self,
        action: &CopyAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session_id = action.session_id.clone();
        info!("Sidebar: EditAction {:?}", action.session_id);
        let exists = self.session_manager.read(cx).query(session_id).is_some();

        if !exists {
            error!(
                "cannot copy session `{}`: session not found",
                session_id.to_string()
            );
            return;
        }

        self.session_manager.update(cx, |manager, _cx| {
            manager.copy_session(session_id, _cx);
        });

        cx.notify();
    }

    // ========================================================================
    // Delete
    // ========================================================================

    fn delete_session(
        &mut self,
        action: &DeleteAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session_id = action.session_id.clone();
        info!("Sidebar: EditAction {}", action.session_id.to_string());
        let exists = self.session_manager.read(cx).query(session_id).is_some();

        if !exists {
            error!(
                "cannot delete session `{}`: session not found",
                session_id.to_string()
            );
            return;
        }

        self.session_manager.update(cx, |manager, _cx| {
            manager.del_session(session_id, _cx);
        });

        // window.dispatch_action(Box::new(CloseTerminalAction { tab_id: session_id }), cx);

        // 如果删除的是当前 active session，
        // 则取消当前选中状态。
        //
        // 如果你的 AppState 已经实现了：
        //
        //     clear_active_session()
        //
        // 可以在这里直接调用。
        //
        // 当前不直接调用，以避免假设你 AppState 中不存在的方法。
        cx.notify();
    }

    // ========================================================================
    // Open
    // ========================================================================

    fn open_session(&mut self, session_id: SessionId, window: &mut Window, cx: &mut Context<Self>) {
        // 更新 SessionManager 中的状态。
        self.session_manager.update(cx, |manager, _cx| {
            manager.open_session(session_id);
        });

        // 更新 AppState。
        //
        // 你的原始代码这里已经有：
        //
        //     state.set_active_session(...)
        //
        // 所以这里恢复使用这个设计。
        self.state.update(cx, |state, cx| {
            // state.set_active_tab(&session_id);

            cx.notify();
        });

        // 打开 Terminal。
        window.dispatch_action(Box::new(OpenTerminalAction { session_id }), cx);
    }

    // ========================================================================
    // Connection Dialog
    // ========================================================================

    // ========================================================================
    // Group Toggle
    // ========================================================================

    fn toggle_group(&mut self, group_name: String, cx: &mut Context<Self>) {
        if self.collapsed.contains(&group_name) {
            self.collapsed.remove(&group_name);
        } else {
            self.collapsed.insert(group_name);
        }

        cx.notify();
    }

    // ========================================================================
    // Focus
    // ========================================================================

    fn focus_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        info!("Sidebar: focus_sidebar");
        self.focus.focus(window, cx);
    }

    // ========================================================================
    // Refresh
    // ========================================================================

    fn refresh(&mut self, cx: &mut Context<Self>) {
        // SessionManager 如果以后增加 refresh/reload
        // 可以在这里触发。
        cx.notify();
    }
}

// ============================================================================
// Render
// ============================================================================

impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme();

        let query = self.search_query(cx);

        let active_id = self.state.read(cx).active_tab_id.clone();

        let grouped = self.session_manager.read(cx).grouped_sessions();

        let groups = grouped
            .into_iter()
            .filter_map(|(group_name, sessions)| {
                let sessions = if query.is_empty() {
                    sessions
                } else {
                    sessions
                        .into_iter()
                        .filter(|session| Self::matches_search(session, &query))
                        .collect()
                };

                if query.is_empty() || !sessions.is_empty() {
                    Some((group_name, sessions))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        div()
            // ---------------------------------------------------------------
            // Focus
            // ---------------------------------------------------------------
            .track_focus(&self.focus)
            // ---------------------------------------------------------------
            // Actions
            // ---------------------------------------------------------------
            // .on_action(cx.listener(Self::edit_session))
            .on_action(cx.listener(Self::copy_session))
            .on_action(cx.listener(Self::delete_session))
            // ---------------------------------------------------------------
            // Root
            // ---------------------------------------------------------------
            .size_full()
            .flex()
            .flex_col()
            .bg(t.colors().background)
            .border_r_1()
            .border_color(t.colors().border)
            // ================================================================
            // Header
            // ================================================================
            .child(self.render_header(window, cx))
            // ================================================================
            // Search
            // ================================================================
            .child(self.render_search(cx))
            // ================================================================
            // Session List
            // ================================================================
            .child(self.render_groups(groups, cx))
    }
}

// ============================================================================
// Search Match
// ============================================================================

impl Sidebar {
    fn matches_search(session: &Session, query: &str) -> bool {
        session.name.to_lowercase().contains(query)
            || session.hostname.to_lowercase().contains(query)
            || session.username.to_lowercase().contains(query)
            || session.group.to_lowercase().contains(query)
    }
}

// ============================================================================
// Header
// ============================================================================

impl Sidebar {
    fn render_header(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.theme();

        div()
            .flex()
            .items_center()
            .h(px(44.0))
            .px(px(10.0))
            .gap(px(4.0))
            .border_b_1()
            .border_color(t.colors().border)
            // ---------------------------------------------------------------
            // Title
            // ---------------------------------------------------------------
            .child(
                div()
                    .flex_1()
                    .px(px(4.0))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.colors().text_muted)
                    .child("SESSIONS"),
            )
            // ---------------------------------------------------------------
            // Refresh
            // ---------------------------------------------------------------
            .child(
                Button::new("refresh-sessions")
                    .icon(IconName::Redo2)
                    .on_click(cx.listener(|_this, _event, _window, cx| {
                        cx.notify();
                    })),
            )
            // ---------------------------------------------------------------
            // Add
            // ---------------------------------------------------------------
            .child(
                Button::new("new-session")
                    .icon(IconName::Plus)
                    .on_click(cx.listener(|this, _event, window, cx| {
                        // this.new_session(window, cx);
                        window.dispatch_action(Box::new(EditAction { session_id: None }), cx);
                    })),
            )
            // ---------------------------------------------------------------
            // More
            // ---------------------------------------------------------------
            .child(
                Button::new("session-menu")
                    .icon(IconName::Ellipsis)
                    .on_click(|_event, _window, _cx| {
                        // 后续可以放：
                        //
                        // - 新建分组
                        // - 导入
                        // - 导出
                        // - 全部展开
                        // - 全部折叠
                    }),
            )
            .into_any_element()
    }
}

// ============================================================================
// Search
// ============================================================================

impl Sidebar {
    fn render_search(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.theme();

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .py(px(8.0))
            .border_b_1()
            .border_color(t.colors().border)
            .child(
                div()
                    .flex_none()
                    .text_size(px(13.0))
                    .text_color(t.colors().text_muted)
                    .child(IconName::Search),
            )
            .child(
                div().flex_1().child(
                    Input::new(&self.search_input)
                        .px(px(8.0))
                        .py(px(5.0))
                        .rounded(px(5.0))
                        .bg(t.colors().surface_background)
                        .border_1()
                        .border_color(t.colors().border)
                        .text_size(px(12.0))
                        .text_color(t.colors().text),
                ),
            )
            .into_any_element()
    }
}

// ============================================================================
// Group List
// ============================================================================

impl Sidebar {
    fn render_groups(
        &self,
        groups: Vec<(String, Vec<Session>)>,
        // active_id: Option<SessionId>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if groups.is_empty() {
            let t = cx.theme();
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .px(px(24.0))
                .text_center()
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.colors().text)
                        .child("还没有会话"),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(t.colors().text_muted)
                        .child("点击右上角 + 创建第一个连接"),
                )
                .into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .py(px(6.0))
            .children(
                groups
                    .into_iter()
                    .map(|(group_name, sessions)| self.render_group(group_name, sessions, cx)),
            )
            .into_any_element()
    }

    fn render_group(
        &self,
        group_name: String,
        sessions: Vec<Session>,
        // active_id: Option<SessionId>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = cx.theme();

        let collapsed = self.collapsed.contains(&group_name);

        let session_count = sessions.len();

        let chevron = if collapsed {
            IconName::ChevronRight
        } else {
            IconName::ChevronDown
        };

        let toggle_name = group_name.clone();

        let mut element = div().flex().flex_col().mb(px(8.0));

        // ---------------------------------------------------------------
        // Group header
        // ---------------------------------------------------------------

        element = element.child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .px(px(10.0))
                .py(px(5.0))
                .cursor_pointer()
                .hover(|style| style.bg(t.colors().surface_background))
                .child(
                    div()
                        .flex_none()
                        .text_size(px(12.0))
                        .text_color(t.colors().text_muted)
                        .child(chevron),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.colors().text_muted)
                        .child(group_name.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.0))
                        // .text_color(t.colors().text_subtle)
                        .child(session_count.to_string()),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, _window, cx| {
                        this.toggle_group(toggle_name.clone(), cx);
                    }),
                ),
        );

        // ---------------------------------------------------------------
        // Sessions
        // ---------------------------------------------------------------

        if !collapsed {
            for session in sessions {
                // let is_active = active_id.as_deref() == Some(session.id);
                let is_active = false;

                element = element.child(self.render_session_item(session, is_active, cx));
            }
        }

        element.into_any_element()
    }
}

// ============================================================================
// Session Item
// ============================================================================

impl Sidebar {
    fn render_session_item(
        &self,
        session: Session,
        is_active: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = cx.theme();

        let session_id = session.id;

        let context_menu_id = session.id.clone();

        // ---------------------------------------------------------------
        // Active background
        // ---------------------------------------------------------------

        let background = if is_active {
            t.colors().element_background
        } else {
            Hsla::transparent_black()
        };

        let left_border = if is_active {
            t.colors().icon_accent
        } else {
            Hsla::transparent_black()
        };

        let hostname = session.hostname.clone();

        let port = session.port;

        let username = session.username.clone();

        let name = session.name.clone();

        // ---------------------------------------------------------------
        // Session item
        // ---------------------------------------------------------------

        div()
            .id(format!("session-item-{}", session_id.to_string()))
            .flex()
            .items_center()
            .gap(px(8.0))
            .w_full()
            .px(px(10.0))
            .py(px(7.0))
            .bg(background)
            .border_l_2()
            .border_color(left_border)
            .cursor_pointer()
            .hover(|style| style.bg(t.colors().elevated_surface_background))
            // -----------------------------------------------------------
            // Left mouse button
            // -----------------------------------------------------------
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    // 这里先立即把 Sidebar 重新拿回焦点。
                    //
                    // 防止后面的 ContextMenu /
                    // terminal action 导致焦点状态异常。
                    this.focus_sidebar(window, cx);

                    // 双击 -> 打开终端。
                    if event.click_count == 2 {
                        this.open_session(session_id, window, cx);
                    } else {
                        // 单击 -> 只选中。
                        this.state.update(cx, |state, cx| {
                            // state.set_active_tab(&session_id);

                            cx.notify();
                        });

                        cx.notify();
                    }
                }),
            )
            // -----------------------------------------------------------
            // Right click / context menu
            // -----------------------------------------------------------
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _event, window, _cx| {
                    // 非常关键：
                    //
                    // 右键菜单打开前，把焦点重新放回 Sidebar。
                    //
                    // 这能避免：
                    //
                    // RightClick
                    //     ↓
                    // ContextMenu
                    //     ↓
                    // Focus 被菜单拿走
                    //     ↓
                    // Action 找不到 Sidebar
                    //
                    this.focus_sidebar(window, _cx);
                }),
            )
            .context_menu({
                let id = context_menu_id.clone();

                move |menu, _window, _cx| {
                    menu.menu(
                        "编辑",
                        Box::new(EditAction {
                            session_id: Some(id),
                        }),
                    )
                    .separator()
                    .menu(
                        "复制",
                        Box::new(CopyAction {
                            session_id: id.clone(),
                        }),
                    )
                    .separator()
                    .menu(
                        "删除",
                        Box::new(DeleteAction {
                            session_id: id.clone(),
                        }),
                    )
                }
            })
            // -----------------------------------------------------------
            // Session content
            // -----------------------------------------------------------
            .child(
                div()
                    .flex_none()
                    .size(px(18.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.0))
                    .text_color(t.colors().icon_accent), // .child(
                                                         //     // IconName::Terminal
                                                         // ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .overflow_hidden()
                    .child(
                        div()
                            .w_full()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.colors().text)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(name),
                    )
                    .child(
                        div()
                            .w_full()
                            .text_size(px(10.5))
                            .text_color(t.colors().text_muted)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(format!("{}:{} · {}", hostname, port, username)),
                    ),
            )
            .into_any_element()
    }
}
