//! `ConnectionDialog` — view shown when `ActiveView::NewConnection` is active.
//!
//! Layout: 2-column with a marketing/feature list on the left and a card with
//! the connection form on the right. The card model is purely visual;
//! the actual field state lives in `AppState` once the user saves.

use gpui::{accesskit::Uuid, prelude::FluentBuilder, *};
use gpui_component::{
    IconName,
    checkbox::Checkbox,
    input::{Input, InputState},
    radio::RadioGroup,
    tab::TabBar,
};
use gpui_rsx::rsx;

use strum::IntoEnumIterator;
use theme::{ActiveTheme, Theme};

use crate::{
    session_manager::{self, Protocol, Session, SessionManager, SessionStatus},
    title_bar::PlatformTitleBar,
};

pub struct ConnectionDialog {
    pub title_bar: Entity<PlatformTitleBar>,
    // state: Entity<AppState>,
    name: Entity<InputState>,
    group: Entity<InputState>,
    host: Entity<InputState>,
    port: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    protocol: usize,
    authentication: Option<usize>,
    identity: Entity<InputState>,

    session_manager: Entity<SessionManager>,
}

impl ConnectionDialog {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        session_manager: Entity<SessionManager>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder(" "));
        let group = cx.new(|cx| InputState::new(window, cx));
        let host = cx.new(|cx| InputState::new(window, cx).placeholder("[用户@]主机地址"));
        let port = cx.new(|cx| InputState::new(window, cx).placeholder("22"));
        let username = cx.new(|cx| InputState::new(window, cx));
        let password = cx.new(|cx| InputState::new(window, cx));
        let protocol = 0;
        let authentication = Some(0);
        let identity = cx.new(|cx| InputState::new(window, cx));

        let title_bar = cx.new(|cx| {
            let mut bar = PlatformTitleBar::new("settings-title-bar", cx);
            bar.set_button_layout(Some(WindowButtonLayout {
                left: [None, None, None],
                right: [None, None, Some(WindowButton::Close)],
            }));

            bar
        });

        Self {
            title_bar,
            name,
            group,
            host,
            port,
            username,
            password,
            protocol,
            authentication,
            identity,
            // form,
            session_manager,
        }
    }

    fn save_session(&self, cx: &mut Context<ConnectionDialog>, is_connect: bool) {
        let idx = self.protocol;
        let (_, protocol) = Protocol::iter()
            .enumerate()
            .find(|(i, _)| *i == idx)
            .unwrap();
        let session = Session {
            id: Uuid::new_v4().to_string(),
            name: self.name.read(cx).value().to_string(),
            group: self.group.read(cx).value().to_string(),
            host: self.host.read(cx).value().to_string(),
            port: self
                .port
                .read(cx)
                .value()
                .to_string()
                .parse::<u16>()
                .unwrap_or(22),
            username: self.port.read(cx).value().to_string(),
            protocol: protocol,
            auth: session_manager::AuthMethod::Password,
            identity_file: Some(self.identity.read(cx).value().to_string()),
            status: SessionStatus::Disconnected,
            latencies_ms: vec![],
        };
        self.session_manager.update(cx, |this, cx| {
            this.save_session(session,is_connect);
        })
    }
}

impl Render for ConnectionDialog {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.title_bar.update(cx, |this, _cx| {
            this.set_children(["新建会话".into_any_element()]);
        });
        let t = cx.theme();

        let name = self.name.clone();
        let group = self.group.clone();
        let host = self.host.clone();
        let port = self.port.clone();
        let username = self.username.clone();
        let password= self.password.clone();
        let protocol = self.protocol;
        let authentication = self.authentication.clone();
        let identity = self.identity.clone();

        div().size_full().child(self.title_bar.clone()).child(
            div()
                .id("newconn")
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .p(px(32.0))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .w_full()
                        .max_w(px(1080.0))
                        .gap(px(40.0))
                        .items_start()
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .flex_col()
                                .overflow_hidden()
                                .child(
                                    div().child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .px(px(24.0))
                                            .py(px(20.0))
                                            .gap(px(12.0))
                                            .child(
                                                // TabBar 组件
                                                div().child(
                                                    TabBar::new("segmented-tabs")
                                                        .segmented()
                                                        .selected_index(protocol)
                                                        .on_click(cx.listener(
                                                            |this, e, _window, _cx| {
                                                                this.protocol = *e;
                                                            },
                                                        ))
                                                        .children(
                                                            Protocol::iter()
                                                                .map(|e| e.to_string())
                                                                .collect::<Vec<String>>(),
                                                        ),
                                                ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(12.0))
                                                    .child(
                                                        div().child(
                                                            div()
                                                                .w(px(130.0))
                                                                .text_right()
                                                                .text_size(px(12.5))
                                                                .text_color(t.colors().text_muted)
                                                                .child("name"),
                                                        ),
                                                    )
                                                    .child(
                                                        div().flex_1().child(input(t, name, false)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(12.0))
                                                    .child(
                                                        div().child(
                                                            div()
                                                                .w(px(130.0))
                                                                .text_right()
                                                                .text_size(px(12.5))
                                                                .text_color(t.colors().text_muted)
                                                                .child("group"),
                                                        ),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .child(input(t, group, false)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(12.0))
                                                    .child(
                                                        div().child(
                                                            div()
                                                                .w(px(130.0))
                                                                .text_right()
                                                                .text_size(px(12.5))
                                                                .text_color(t.colors().text_muted)
                                                                .child("host"),
                                                        ),
                                                    )
                                                    .child(
                                                        div().flex_1().child(input(t, host, false)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(12.0))
                                                    .child(
                                                        div().child(
                                                            div()
                                                                .w(px(130.0))
                                                                .text_right()
                                                                .text_size(px(12.5))
                                                                .text_color(t.colors().text_muted)
                                                                .child("port"),
                                                        ),
                                                    )
                                                    .child(
                                                        div().flex_1().child(input(t, port, false)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(12.0))
                                                    .child(
                                                        div().child(
                                                            div()
                                                                .w(px(130.0))
                                                                .text_right()
                                                                .text_size(px(12.5))
                                                                .text_color(t.colors().text_muted)
                                                                .child("username"),
                                                        ),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .child(input(t, username, false)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(12.0))
                                                    .child(
                                                        div().child(
                                                            div()
                                                                .w(px(130.0))
                                                                .text_right()
                                                                .text_size(px(12.5))
                                                                .text_color(t.colors().text_muted)
                                                                .child("authentication"),
                                                        ),
                                                    )
                                                    .child(div().flex_1().child( RadioGroup::horizontal("options")
                                                        .children(["SSH key","Password", "Agent"])
                                                        .selected_index(self.authentication)
                                                        .on_click(cx.listener(|view, selected_index: &usize, _, cx| {
                                                            view.authentication = Some(*selected_index);
                                                            cx.notify();
                                                        })))),
                                            )
                                            .when_some(self.authentication, |this,v|{
                                                match v{
                                                    0=>{
                                                        this.child("child")
                                                    },
                                                    1=>{
                                                        
                                                        this               .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(12.0))
                                                    .child(
                                                        div().child(
                                                            div()
                                                                .w(px(130.0))
                                                                .text_right()
                                                                .text_size(px(12.5))
                                                                .text_color(t.colors().text_muted)
                                                                .child("password"),
                                                        ),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .child(input(t, password, false)),
                                                    ),
                                            )
                                                    }
                                                    _=>{

                                                        this.child("child")
                                                    }
                                                }
                                            })
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(12.0))
                                                    .child(
                                                        div().child(
                                                            div()
                                                                .w(px(130.0))
                                                                .text_right()
                                                                .text_size(px(12.5))
                                                                .text_color(t.colors().text_muted)
                                                                .child("identity"),
                                                        ),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .child(input(t, identity, false)),
                                                    ),
                                            ),
                                    ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_row()
                                                .justify_end()
                                                .gap(px(8.0))
                                                .child(
                                                    button(t, ButtonKind::Ghost, "Cancel")
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(|t, e, w, cx| {
                                                                w.remove_window();
                                                            }),
                                                        ),
                                                )
                                                .child(
                                                    button(t, ButtonKind::Secondary, "Save only")
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(move |this, _, w, cx| {
                                                                this.save_session (cx, false);
                                                                w.remove_window();
                                                            }),
                                                        ),
                                                )
                                                .child(
                                                    button(
                                                        t,
                                                        ButtonKind::Primary,
                                                        "Save & connect",
                                                    )
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |this, _, window, cx| {
                                                            this.save_session(cx, true);
                                                            window.remove_window();
                                                        }),
                                                    ),
                                                ),
                                        ),
                                ),
                        ),
                ),
        )
    }
}

// fn card_body(t: &Theme) -> impl IntoElement {
//     let row_label = |label: &str| -> AnyElement {
//         div()
//             .w(px(130.0))
//             .text_size(px(12.5))
//             .text_color(t.colors().text_muted)
//             .text_align(TextAlign::Right)
//             .child(text!(label))
//             .into_any_element()
//     };

//     let mut body = div()
//         .flex()
//         .flex_col()
//         .px(px(24.0))
//         .py(px(20.0))
//         .gap(px(12.0));

//     body = body
//         .child(field_row(
//             row_label("Name"),
//             input(t, , false),
//         ))
//         .child(field_row(
//             row_label("Group"),
//             select_dd(t, &["Production", "Staging", "Personal", "+ New group"]),
//         ))
//         .child(field_row(row_label("Host"), input(t, "10.0.1.21", true)))
//         .child(field_row(row_label("Port"), input(t, "22", true)))
//         .child(field_row(row_label("Username"), input(t, "deploy", false)))
//         .child(field_row(
//             row_label("Protocol"),
//             radio_row(t, &["SSH", "Mosh", "Telnet", "Local"], 0),
//         ))
//         .child(field_row(
//             row_label("Authentication"),
//             radio_row(t, &["SSH key", "Password", "Agent"], 0),
//         ))
//         .child(field_row(row_label("Identity file"), file_input(t)));

//     body
// }

// Tiny invisible marker that retains the Model handle so the lifetime is obvious
// in this reference code. Remove once you wire the model into a form provider.
fn state_to_hidden_marker() -> AnyElement {
    div().id("state-ref").into_any_element()
}

// ---------------------------------------------------------------------------
// Building blocks
// ---------------------------------------------------------------------------

fn field_row(label: AnyElement, control: AnyElement) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(label)
        .child(div().flex_1().child(control))
}

fn input(t: &Theme, value: Entity<InputState>, mono: bool) -> AnyElement {
    let input = Input::new(&value);

    let mut d = input
        .px(px(10.0))
        .py(px(7.0))
        .rounded(px(6.0))
        .bg(rgb(0x16181d))
        .border_1()
        .border_color(rgb(0x2a2e36))
        .text_size(px(12.5))
        .text_color(t.colors().text);
    if mono {
        d = d.font_family("JetBrains Mono");
    }
    // d.child(text!(value)).into_any_element()
    d.into_any_element()
}

fn select_dd(t: &Theme, options: &[&str]) -> AnyElement {
    div()
        .flex_1()
        .px(px(10.0))
        .py(px(7.0))
        .rounded(px(6.0))
        .bg(t.colors().background)
        .border_1()
        .border_color(t.colors().border)
        .text_size(px(12.5))
        .text_color(t.colors().text)
        .child(text!(options.first().copied().unwrap_or("")))
        .into_any_element()
}

fn radio_row(t: &Theme, options: &[&str], active_idx: usize) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .gap(px(16.0))
        .children(options.iter().enumerate().map(|(i, opt)| {
            let active = i == active_idx;
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .text_size(px(12.5))
                .text_color(t.colors().text)
                .child(
                    div()
                        .size(px(14.0))
                        .rounded_full()
                        .border_1()
                        .border_color(if active {
                            t.colors().icon_accent
                        } else {
                            t.colors().border
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if active {
                            div()
                                .size(px(7.0))
                                .rounded_full()
                                .bg(t.colors().icon_accent)
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }),
                )
                .child(text!(*opt))
        }))
        .into_any_element()
}

fn file_input(t: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .flex_1()
        .gap(px(8.0))
        .items_center()
        .child(
            div()
                .flex_1()
                .px(px(10.0))
                .py(px(7.0))
                .rounded(px(6.0))
                .bg(t.colors().background)
                .border_1()
                .border_color(t.colors().border)
                // .font_family( "JetBrains Mono")
                .text_size(px(12.5))
                .text_color(t.colors().text)
                .child("~/.ssh/id_ed25519"),
        )
        .child(
            div()
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .bg(t.colors().background)
                .border_1()
                .border_color(t.colors().border)
                .text_color(t.colors().text)
                .text_size(px(12.0))
                .child("Browse…"),
        )
        .into_any_element()
}

#[derive(Clone, Copy)]
enum ButtonKind {
    Primary,
    Secondary,
    Ghost,
}

fn button(t: &Theme, kind: ButtonKind, label: &str) -> Div {
    let (bg, fg, border) = match kind {
        ButtonKind::Primary => (
            t.colors().icon_accent,
            hsla(0., 0., 0.97, 1.),
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
        // .id(("btn", label))
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .bg(bg)
        .text_color(fg)
        .border_1()
        .border_color(border)
        .text_size(px(12.5))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .child(text!(label))
}

// ---------- tiny colour helpers ----------

fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla { h, s, l, a }
}

fn transparent_hsla() -> Hsla {
    Hsla {
        h: 0.,
        s: 0.,
        l: 0.,
        a: 0.,
    }
}
