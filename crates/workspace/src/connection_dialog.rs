//! ConnectionDialog
//!
//! 新建 / 编辑会话窗口。
//!
//! 负责：
//! - 编辑会话基本信息
//! - 选择协议
//! - 选择认证方式
//! - 根据认证方式动态显示认证配置
//! - 校验表单
//! - 创建 / 更新 Session
//! - Save / Save & Connect
//!
//! 业务层中的 AuthMethod：
//!
//! AuthMethod::Password {
//!     password: String,
//! }
//!
//! AuthMethod::PublicKey {
//!     private_key: String,
//!     passphrase: Option<String>,
//! }
//!
//! AuthMethod::Agent
//!
//! AuthMethod::KeyboardInteractive

use gpui::{prelude::FluentBuilder, *};

use gpui_component::{
    input::{Input, InputState},
    radio::RadioGroup,
    tab::TabBar,
};

use protocol::{AuthMethod, Protocol, Session, SessionId, SessionStatus};

use strum::IntoEnumIterator;
use theme::{ActiveTheme, Theme};

use crate::{
    session_store::SessionStore,
    title_bar::{self, PlatformTitleBar},
};

// ============================================================================
// AuthenticationType
// ============================================================================

/// ConnectionDialog 中使用的认证类型。
///
/// 注意：
///
/// 这个 enum 只负责 UI 状态。
/// 真正需要保存的认证配置仍然放在 AuthMethod 中。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthenticationType {
    PublicKey,
    Password,
    Agent,
    KeyboardInteractive,
}

impl AuthenticationType {
    pub fn index(self) -> usize {
        match self {
            Self::PublicKey => 0,
            Self::Password => 1,
            Self::Agent => 2,
            Self::KeyboardInteractive => 3,
        }
    }

    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::PublicKey),
            1 => Some(Self::Password),
            2 => Some(Self::Agent),
            3 => Some(Self::KeyboardInteractive),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::PublicKey => "SSH Key",
            Self::Password => "Password",
            Self::Agent => "Agent",
            Self::KeyboardInteractive => "Keyboard Interactive",
        }
    }

    pub fn requires_password(self) -> bool {
        matches!(self, Self::Password)
    }

    pub fn requires_private_key(self) -> bool {
        matches!(self, Self::PublicKey)
    }

    pub fn requires_passphrase(self) -> bool {
        matches!(self, Self::PublicKey)
    }
}

impl Default for AuthenticationType {
    fn default() -> Self {
        Self::Password
    }
}

// ============================================================================
// ConnectionDialog
// ============================================================================

pub struct ConnectionDialog {
    pub title_bar: Entity<PlatformTitleBar>,
    /// None:
    ///     新建会话
    ///
    /// Some(id):
    ///     编辑已有会话
    session_id: Option<SessionId>,

    // ------------------------------------------------------------------------
    // Basic
    // ------------------------------------------------------------------------
    name: Entity<InputState>,
    group: Entity<InputState>,
    hostname: Entity<InputState>,
    port: Entity<InputState>,
    username: Entity<InputState>,

    // ------------------------------------------------------------------------
    // Authentication
    // ------------------------------------------------------------------------
    /// Password authentication.
    password: Entity<InputState>,

    /// SSH private key.
    private_key: Entity<InputState>,

    /// SSH private key passphrase.
    passphrase: Entity<InputState>,

    // ------------------------------------------------------------------------
    // Selection
    // ------------------------------------------------------------------------
    protocol: Protocol,

    authentication: AuthenticationType,

    // ------------------------------------------------------------------------
    // Backend
    // ------------------------------------------------------------------------
    session_manager: Entity<SessionStore>,
    validation_error: Option<String>,
}

// ============================================================================
// Constructor
// ============================================================================

impl ConnectionDialog {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        session_manager: Entity<SessionStore>,
        session_id: Option<SessionId>,
    ) -> Self {
        let session = session_id
            .and_then(|id| session_manager.read(cx).query(id).cloned())
            .unwrap_or_else(Self::default_session);

        let protocol = session.protocol;

        let authentication = Self::authentication_from_auth(&session.auth);

        let password_value = match &session.auth {
            AuthMethod::Password { password } => password.clone(),
            _ => String::new(),
        };

        let private_key_value = match &session.auth {
            AuthMethod::PublicKey { private_key, .. } => private_key.clone(),

            _ => String::new(),
        };

        let passphrase_value = match &session.auth {
            AuthMethod::PublicKey { passphrase, .. } => passphrase.clone().unwrap_or_default(),

            _ => String::new(),
        };

        // --------------------------------------------------------------------
        // Basic fields
        // --------------------------------------------------------------------

        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("会话名称")
                .default_value(session.name)
        });

        let group = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("默认")
                .default_value(session.group)
        });

        let hostname = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("example.com")
                .default_value(session.hostname)
        });

        let port = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("22")
                .default_value(session.port.to_string())
        });

        let username = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("username")
                .default_value(session.username)
        });

        // --------------------------------------------------------------------
        // Authentication fields
        // --------------------------------------------------------------------

        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("密码")
                .default_value(password_value)
        });

        let private_key = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("~/.ssh/id_ed25519")
                .default_value(private_key_value)
        });

        let passphrase = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("私钥密码，可选")
                .default_value(passphrase_value)
        });
        let title_bar = cx.new(|cx| PlatformTitleBar::new("connection_dialog_title_bar", cx));
        // title_bar::set_title(&title_bar, "新建 / 编辑会话", cx);

        Self {
            title_bar,
            session_id,

            name,
            group,
            hostname,
            port,
            username,

            password,
            private_key,
            passphrase,

            protocol,
            authentication,

            session_manager,
            validation_error: None,
        }
    }

    fn default_session() -> Session {
        Session {
            id: SessionId::new(),

            name: String::new(),

            group: String::new(),

            hostname: String::new(),

            port: 22,

            username: String::new(),

            protocol: Protocol::Ssh,

            auth: AuthMethod::Password {
                password: String::new(),
            },

            status: SessionStatus::Disconnected,
            latencies_ms: Vec::new(),
        }
    }
}

// ============================================================================
// Authentication
// ============================================================================

impl ConnectionDialog {
    fn authentication_from_auth(auth: &AuthMethod) -> AuthenticationType {
        match auth {
            AuthMethod::Password { .. } => AuthenticationType::Password,

            AuthMethod::PublicKey { .. } => AuthenticationType::PublicKey,

            AuthMethod::Agent => AuthenticationType::Agent,

            AuthMethod::KeyboardInteractive => AuthenticationType::KeyboardInteractive,
        }
    }

    fn build_auth_method(&self, cx: &Context<Self>) -> Result<AuthMethod, String> {
        match self.authentication {
            // ----------------------------------------------------------------
            // Password
            // ----------------------------------------------------------------
            AuthenticationType::Password => {
                let password = self.password.read(cx).value().to_string();

                Ok(AuthMethod::Password { password })
            }

            // ----------------------------------------------------------------
            // SSH Key
            // ----------------------------------------------------------------
            AuthenticationType::PublicKey => {
                let private_key = self.private_key.read(cx).value().trim().to_string();

                if private_key.is_empty() {
                    return Err("SSH Key 认证需要指定私钥文件".to_string());
                }

                let passphrase = self.passphrase.read(cx).value().trim().to_string();

                Ok(AuthMethod::PublicKey {
                    private_key,

                    passphrase: if passphrase.is_empty() {
                        None
                    } else {
                        Some(passphrase)
                    },
                })
            }

            // ----------------------------------------------------------------
            // Agent
            // ----------------------------------------------------------------
            AuthenticationType::Agent => Ok(AuthMethod::Agent),

            // ----------------------------------------------------------------
            // Keyboard Interactive
            // ----------------------------------------------------------------
            AuthenticationType::KeyboardInteractive => Ok(AuthMethod::KeyboardInteractive),
        }
    }

    fn select_authentication(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(authentication) = AuthenticationType::from_index(index) {
            self.authentication = authentication;

            cx.notify();
        }
    }
}

// ============================================================================
// Validation
// ============================================================================

impl ConnectionDialog {
    fn validate(&self, cx: &Context<Self>) -> Result<(), String> {
        if self.protocol != Protocol::Ssh {
            return Err("当前版本仅支持 SSH 会话".to_string());
        }

        if self.authentication != AuthenticationType::Password {
            return Err("当前版本仅支持密码认证".to_string());
        }

        if self.password.read(cx).value().is_empty() {
            return Err("请输入密码".to_string());
        }

        // --------------------------------------------------------------------
        // Name
        // --------------------------------------------------------------------

        let name = self.name.read(cx).value().trim().to_string();

        if name.is_empty() {
            return Err("请输入会话名称".to_string());
        }

        // --------------------------------------------------------------------
        // Host
        // --------------------------------------------------------------------

        let hostname = self.hostname.read(cx).value().trim().to_string();

        if hostname.is_empty() {
            return Err("请输入主机地址".to_string());
        }

        // --------------------------------------------------------------------
        // Username
        // --------------------------------------------------------------------

        let username = self.username.read(cx).value().trim().to_string();

        if username.is_empty() {
            return Err("请输入用户名".to_string());
        }

        // --------------------------------------------------------------------
        // Port
        // --------------------------------------------------------------------

        let port = self
            .port
            .read(cx)
            .value()
            .trim()
            .parse::<u16>()
            .map_err(|_| "端口必须是 1-65535 的数字".to_string())?;

        if port == 0 {
            return Err("端口不能为 0".to_string());
        }

        // --------------------------------------------------------------------
        // Authentication
        // --------------------------------------------------------------------

        let _auth = self.build_auth_method(cx)?;

        Ok(())
    }
}

// ============================================================================
// Session
// ============================================================================

impl ConnectionDialog {
    fn build_session(&self, cx: &Context<Self>) -> Result<Session, String> {
        self.validate(cx)?;

        let id = self.session_id.clone().unwrap_or_else(|| SessionId::new());

        let name = self.name.read(cx).value().trim().to_string();

        let group = self.group.read(cx).value().trim().to_string();

        let hostname = self.hostname.read(cx).value().trim().to_string();

        let port = self
            .port
            .read(cx)
            .value()
            .trim()
            .parse::<u16>()
            .map_err(|_| "端口格式错误".to_string())?;

        let username = self.username.read(cx).value().trim().to_string();

        let auth = self.build_auth_method(cx)?;

        Ok(Session {
            id,

            name,

            group,

            hostname,

            port,

            username,

            protocol: self.protocol,

            auth,

            status: SessionStatus::Disconnected,
            latencies_ms: Vec::new(),
        })
    }

    fn save_session(&mut self, cx: &mut Context<Self>, is_connect: bool) -> Result<(), String> {
        let session = self.build_session(cx)?;

        self.session_manager.update(cx, |manager, _cx| {
            manager.save_session(session, is_connect, _cx);
        });

        self.validation_error = None;

        Ok(())
    }
}

// ============================================================================
// Rendering helpers
// ============================================================================

impl ConnectionDialog {
    fn render_input(value: Entity<InputState>, t: &Theme) -> AnyElement {
        Input::new(&value)
            .px(px(10.0))
            .py(px(7.0))
            .rounded(px(6.0))
            .bg(t.colors().element_background)
            .border_1()
            .border_color(t.colors().border)
            .text_size(px(12.5))
            .text_color(t.colors().text)
            .into_any_element()
    }

    fn render_password_input(value: Entity<InputState>, t: &Theme) -> AnyElement {
        Input::new(&value)
            .px(px(10.0))
            .py(px(7.0))
            .rounded(px(6.0))
            .bg(t.colors().element_background)
            .border_1()
            .border_color(t.colors().border)
            .text_size(px(12.5))
            .text_color(t.colors().text)
            .into_any_element()
    }

    fn render_field(
        &self,
        label: &'static str,
        input: AnyElement,
        cx: &Context<Self>,
    ) -> AnyElement {
        let t = cx.theme();

        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.colors().text_muted)
                    .child(label),
            )
            .child(div().flex_1().child(input))
            .into_any_element()
    }

    fn render_authentication(&self, cx: &&mut Context<Self>) -> AnyElement {
        RadioGroup::horizontal("authentication")
            .children([
                AuthenticationType::PublicKey.display_name(),
                AuthenticationType::Password.display_name(),
                AuthenticationType::Agent.display_name(),
                AuthenticationType::KeyboardInteractive.display_name(),
            ])
            .selected_index(Some(self.authentication.index()))
            .on_click(cx.listener(|this, selected_index: &usize, _window, cx| {
                this.select_authentication(*selected_index, cx);
            }))
            .into_any_element()
    }
}

// ============================================================================
// Render
// ============================================================================

impl Render for ConnectionDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = if self.session_id.is_some() {
            "编辑会话"
        } else {
            "新建会话"
        };

        let t = cx.theme();
        // --------------------------------------------------------------------
        // Clone GPUI entities
        // --------------------------------------------------------------------

        let name = self.name.clone();

        let group = self.group.clone();

        let hostname = self.hostname.clone();

        let port = self.port.clone();

        let username = self.username.clone();

        let password = self.password.clone();

        let private_key = self.private_key.clone();

        let passphrase = self.passphrase.clone();

        // --------------------------------------------------------------------
        // Authentication state
        // --------------------------------------------------------------------

        let show_password = self.authentication.requires_password();

        let show_private_key = self.authentication.requires_private_key();

        let show_passphrase = self.authentication.requires_passphrase();

        // --------------------------------------------------------------------
        // Current protocol
        // --------------------------------------------------------------------

        let selected_protocol = Protocol::iter()
            .position(|protocol| protocol == self.protocol)
            .unwrap_or(0);

        // --------------------------------------------------------------------
        // Main window
        // --------------------------------------------------------------------

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t.colors().background)
            .child(self.title_bar.clone())
            // ================================================================
            // Content
            // ================================================================
            .child(
                div().flex_1().flex().items_center().justify_center().child(
                    div()
                        .w_full()
                        // .max_w(px(640.0))
                        .flex()
                        .flex_col()
                        .rounded(px(10.0))
                        .border_1()
                        .border_color(t.colors().border)
                        .bg(t.colors().surface_background)
                        .overflow_hidden()
                        // =================================================
                        // Header
                        // =================================================
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .px(px(28.0))
                                .py(px(22.0))
                                .border_b_1()
                                .border_color(t.colors().border)
                                .child(
                                    div()
                                        .text_size(px(18.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(t.colors().text)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(t.colors().text_muted)
                                        .child(if self.session_id.is_some() {
                                            "修改当前会话的连接配置"
                                        } else {
                                            "配置一个新的终端连接"
                                        }),
                                ),
                        )
                        // =================================================
                        // Form
                        // =================================================
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(16.0))
                                .px(px(28.0))
                                .py(px(24.0))
                                // -----------------------------------------
                                // Protocol
                                // -----------------------------------------
                                .child(self.render_protocol_tabs(selected_protocol, &cx))
                                .when_some(self.validation_error.as_ref(), |this, error| {
                                    this.child(
                                        div()
                                            .px(px(10.0))
                                            .py(px(8.0))
                                            .rounded(px(6.0))
                                            .bg(t.colors().element_background)
                                            .text_size(px(12.0))
                                            .text_color(t.status().conflict)
                                            .child(error.clone()),
                                    )
                                })
                                // -----------------------------------------
                                // Separator
                                // -----------------------------------------
                                .child(div().h(px(1.0)).bg(t.colors().border))
                                // -----------------------------------------
                                // Name
                                // -----------------------------------------
                                .child(self.render_field("名称", Self::render_input(name, t), cx))
                                // -----------------------------------------
                                // Group
                                // -----------------------------------------
                                .child(self.render_field("分组", Self::render_input(group, t), cx))
                                // -----------------------------------------
                                // Host
                                // -----------------------------------------
                                .child(self.render_field(
                                    "主机",
                                    Self::render_input(hostname, t),
                                    cx,
                                ))
                                // -----------------------------------------
                                // Port
                                // -----------------------------------------
                                .child(self.render_field("端口", Self::render_input(port, t), cx))
                                // -----------------------------------------
                                // Username
                                // -----------------------------------------
                                .child(self.render_field(
                                    "用户名",
                                    Self::render_input(username, t),
                                    cx,
                                ))
                                // -----------------------------------------
                                // Authentication
                                // -----------------------------------------
                                .child(self.render_field(
                                    "认证方式",
                                    self.render_authentication(&cx),
                                    cx,
                                ))
                                // -----------------------------------------
                                // SSH Key
                                // -----------------------------------------
                                .when(show_private_key, |this| {
                                    this.child(self.render_field(
                                        "私钥文件",
                                        Self::render_input(private_key, t),
                                        cx,
                                    ))
                                    .when(
                                        show_passphrase,
                                        |this| {
                                            this.child(self.render_field(
                                                "私钥密码",
                                                Self::render_password_input(passphrase, t),
                                                cx,
                                            ))
                                        },
                                    )
                                })
                                // -----------------------------------------
                                // Password
                                // -----------------------------------------
                                .when(show_password, |this| {
                                    this.child(self.render_field(
                                        "密码",
                                        Self::render_password_input(password, t),
                                        cx,
                                    ))
                                })
                                // -----------------------------------------
                                // Agent hint
                                // -----------------------------------------
                                .when(self.authentication == AuthenticationType::Agent, |this| {
                                    this.child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(t.colors().text_muted)
                                            .child("将使用系统 SSH Agent 中可用的密钥进行认证"),
                                    )
                                })
                                // -----------------------------------------
                                // Keyboard Interactive hint
                                // -----------------------------------------
                                .when(
                                    self.authentication == AuthenticationType::KeyboardInteractive,
                                    |this| {
                                        this.child(
                                            div()
                                                .text_size(px(12.0))
                                                .text_color(t.colors().text_muted)
                                                .child("连接过程中由服务器动态请求认证信息"),
                                        )
                                    },
                                ),
                        )
                        // =================================================
                        // Footer
                        // =================================================
                        .child(self.render_footer(cx)),
                ),
            )
    }
}

// ============================================================================
// Protocol Tabs
// ============================================================================

impl ConnectionDialog {
    fn render_protocol_tabs(&self, selected_index: usize, cx: &&mut Context<Self>) -> AnyElement {
        TabBar::new("protocol-tabs")
            .segmented()
            .selected_index(selected_index)
            .on_click(cx.listener(|this, selected_index, _window, cx| {
                if let Some(protocol) = Protocol::iter().nth(*selected_index) {
                    this.protocol = protocol;

                    // 协议变化后立即触发重新渲染。
                    cx.notify();
                }
            }))
            .children(
                Protocol::iter()
                    .map(|protocol| protocol.to_string())
                    .collect::<Vec<_>>(),
            )
            .into_any_element()
    }
}

// ============================================================================
// Footer
// ============================================================================

impl ConnectionDialog {
    fn render_footer(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = cx.theme();

        div()
            .flex()
            .justify_end()
            .items_center()
            .gap(px(8.0))
            .px(px(24.0))
            .py(px(16.0))
            .border_t_1()
            .border_color(t.colors().border)
            // ================================================================
            // Cancel
            // ================================================================
            .child(button(t, ButtonKind::Ghost, "取消").on_mouse_down(
                MouseButton::Left,
                cx.listener(|_this, _event, window, _cx| {
                    window.remove_window();
                }),
            ))
            // ================================================================
            // Save
            // ================================================================
            .child(button(t, ButtonKind::Secondary, "仅保存").on_mouse_down(
                MouseButton::Left,
                cx.listener(
                    |this, _event, window, cx| match this.save_session(cx, false) {
                        Ok(()) => {
                            window.remove_window();
                        }

                        Err(error) => {
                            this.validation_error = Some(error);
                            cx.notify();
                        }
                    },
                ),
            ))
            // ================================================================
            // Save & Connect
            // ================================================================
            .child(button(t, ButtonKind::Primary, "保存并连接").on_mouse_down(
                MouseButton::Left,
                cx.listener(
                    |this, _event, window, cx| match this.save_session(cx, true) {
                        Ok(()) => {
                            window.remove_window();
                        }

                        Err(error) => {
                            this.validation_error = Some(error);
                            cx.notify();
                        }
                    },
                ),
            ))
            .into_any_element()
    }
}

// ============================================================================
// Button
// ============================================================================

#[derive(Clone, Copy)]
enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
}

fn button(t: &Theme, kind: ButtonKind, label: &str) -> Div {
    let (background, foreground, border) = match kind {
        ButtonKind::Primary => (
            t.colors().icon_accent,
            hsla(0.0, 0.0, 0.97, 1.0),
            transparent_hsla(),
        ),

        ButtonKind::Secondary => (t.colors().background, t.colors().text, t.colors().border),

        ButtonKind::Ghost => (
            transparent_hsla(),
            t.colors().text_muted,
            transparent_hsla(),
        ),
    };

    div()
        .px(px(12.0))
        .py(px(7.0))
        .rounded(px(6.0))
        .bg(background)
        .text_color(foreground)
        .border_1()
        .border_color(border)
        .text_size(px(12.5))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .child(text!(label))
}

// ============================================================================
// Color Helpers
// ============================================================================

fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla { h, s, l, a }
}

fn transparent_hsla() -> Hsla {
    Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.0,
    }
}
