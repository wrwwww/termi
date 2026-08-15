//! `ConnectionDialog` — view shown when `ActiveView::NewConnection` is active.
//!
//! Layout: 2-column with a marketing/feature list on the left and a card with
//! the connection form on the right. The card model is purely visual;
//! the actual field state lives in `AppState` once the user saves.

use gpui::*;
use gpui_component::{
    input::{Input, InputState},
    radio::RadioGroup,
};
use gpui_rsx::rsx;
use log::info;
use strum::IntoEnumIterator;
use theme::{ActiveTheme, Theme};

use crate::{
    session_manager::{self, Protocol, Session, SessionManager, SessionStatus},
    title_bar::PlatformTitleBar,
};
// struct SessionForm {
//     name: Entity<InputState>,
//     group: Entity<InputState>,
//     host: Entity<InputState>,
//     port: Entity<InputState>,
//     username: Entity<InputState>,
//     protocol: Option<usize>,
//     authentication: Entity<InputState>,
//     identity: Entity<InputState>,
// }
// impl SessionForm {
//     pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
//         let name = cx.new(|cx| InputState::new(window, cx));
//         let group = cx.new(|cx| InputState::new(window, cx));
//         let host = cx.new(|cx| InputState::new(window, cx));
//         let port = cx.new(|cx| InputState::new(window, cx));
//         let username = cx.new(|cx| InputState::new(window, cx));
//         let protocol = Some(0);
//         let authentication = cx.new(|cx| InputState::new(window, cx));
//         let identity = cx.new(|cx| InputState::new(window, cx));
//         Self {
//             name,
//             group,
//             host,
//             port,
//             username,
//             protocol,
//             authentication,
//             identity,
//         }
//     }
//     pub fn to_session(self, cx: &mut Context<Self>) -> Session {
//         let idx = self.protocol.unwrap_or_else(|| 0);
//         let (_, protocol) = Protocol::iter()
//             .enumerate()
//             .find(|(i, _)| *i == idx)
//             .unwrap();
//         Session {
//             id: String::new(),
//             name: self.name.read(cx).value().to_string(),
//             group: self.group.read(cx).value().to_string(),
//             host: self.host.read(cx).value().to_string(),
//             port: self
//                 .port
//                 .read(cx)
//                 .value()
//                 .to_string()
//                 .parse::<u16>()
//                 .unwrap(),
//             username: self.port.read(cx).value().to_string(),
//             protocol: protocol,
//             auth: session_manager::AuthMethod::Password,
//             identity_file: Some(self.identity.read(cx).value().to_string()),
//             status: SessionStatus::Disconnected,
//             latencies_ms: vec![],
//         }
//     }
// }

pub struct ConnectionDialog {
    pub title_bar: Entity<PlatformTitleBar>,
    // state: Entity<AppState>,
    name: Entity<InputState>,
    group: Entity<InputState>,
    host: Entity<InputState>,
    port: Entity<InputState>,
    username: Entity<InputState>,
    protocol: Option<usize>,
    authentication: Entity<InputState>,
    identity: Entity<InputState>,
    // form: Entity<SessionForm>,
    session_manager: Entity<SessionManager>,
}

impl ConnectionDialog {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        session_manager: Entity<SessionManager>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx));
        let group = cx.new(|cx| InputState::new(window, cx));
        let host = cx.new(|cx| InputState::new(window, cx));
        let port = cx.new(|cx| InputState::new(window, cx));
        let username = cx.new(|cx| InputState::new(window, cx));
        let protocol = Some(0);
        let authentication = cx.new(|cx| InputState::new(window, cx));
        let identity = cx.new(|cx| InputState::new(window, cx));
        // let form = cx.new(|cx| SessionForm::new(window, cx));
        // let protocol = cx.new(|cx| {
        //     ButtonGroup::new(Some(Protocol::Ssh))
        //         .display_fn(|_, e| e.to_string())
        //         .options(Protocol::iter())
        // });

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
            protocol,
            authentication,
            identity,
            // form,
            session_manager,
        }
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
        let protocol = self.protocol.clone();
        let authentication = self.authentication.clone();
        let identity = self.identity.clone();
        let list = Protocol::iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>();

        div().size_full().child(self.title_bar.clone()).child(
        rsx! {
            <div id="newconn" flex flex_1 items_center justify_center p={px(32.)}>

                <div flex flex_row w_full max_w={px(1080.)} gap={px(40.)} items_start>
                    <div w_full flex flex_col overflow_hidden>
                        <div>
                            <div flex flex_col px={px(24.)} py={px(20.)} gap={px(12.)}>
                                <div flex flex_row items_center gap={px(12.)}>
                                    <div><div w={px(130.0)}  text_right  text_size={px(12.5)} text_color={t.colors().text_muted}>"name"</div></div>
                                    <div flex_1>{input(t, name, false)}</div>
                                </div>
                                     <div flex flex_row items_center gap={px(12.)}>
                                    <div><div w={px(130.0)}  text_right  text_size={px(12.5)} text_color={t.colors().text_muted}>"group"</div></div>
                                    <div flex_1>{input(t, group, false)}</div>
                                </div>
                                <div flex flex_row items_center gap={px(12.)}>
                                    <div><div w={px(130.0)}  text_right  text_size={px(12.5)} text_color={t.colors().text_muted}>"host"</div></div>
                                    <div flex_1>{input(t, host, false)}</div>
                                </div>
                                <div flex flex_row items_center gap={px(12.)}>
                                    <div><div w={px(130.0)}  text_right  text_size={px(12.5)} text_color={t.colors().text_muted}>"port"</div></div>
                                    <div flex_1>{input(t, port, false)}</div>
                                </div>
                                <div flex flex_row items_center gap={px(12.)}>
                                    <div><div w={px(130.0)}  text_right  text_size={px(12.5)} text_color={t.colors().text_muted}>"username"</div></div>
                                    <div flex_1>{input(t, username, false)}</div>
                                </div>
                                <div flex flex_row items_center gap={px(12.)}>
                                    <div><div w={px(130.0)}  text_right  text_size={px(12.5)} text_color={t.colors().text_muted}>"protocol"</div></div>
                                    <div flex_1>
                                    {
                                        // self.protocol.clone()
                                        RadioGroup::horizontal("options")
                                        .children(list)
                                        .selected_index(self.protocol)
                                        .on_click(cx.listener(|view, selected_index: &usize, _, cx| {
                                           info!("点击{}",*selected_index);
                                           view.protocol = Some(*selected_index);

                                            cx.notify();
                                        }))



                                    }
                                    </div>
                                </div>
                                <div flex flex_row items_center gap={px(12.)}>
                                    <div><div w={px(130.0)}  text_right  text_size={px(12.5)} text_color={t.colors().text_muted}>"authentication"</div></div>
                                    <div flex_1>{input(t, authentication, false)}</div>
                                </div>
                                <div flex flex_row items_center gap={px(12.)}>
                                    <div><div w={px(130.0)}  text_right  text_size={px(12.5)} text_color={t.colors().text_muted}>"identity"</div></div>
                                     <div flex_1>{input(t, identity, false)}</div>
                                </div>

                            </div>
                        </div>
                        <div>{card_footer(t,&cx)}</div>
                    </div>
                </div>
            </div>
        })
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

fn card_footer(t: &Theme, cx: &&mut Context<ConnectionDialog>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px(px(24.0))
        .py(px(16.0))
        .border_t_1()
        .border_color(t.colors().border)
        .bg(t.colors().background)
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(button(t, ButtonKind::Ghost, "Cancel"))
                .child(button(t, ButtonKind::Secondary, "Save only"))
                .child(
                    button(t, ButtonKind::Primary, "Save & connect").on_mouse_down_out(
                        cx.listener(move |this, _, _, cx| {
                            // this.state.update(cx, |state, cx| {
                            //     state.set_active_view(ActiveView::Workspace)
                            // });

                            let idx = this.protocol.unwrap_or_else(|| 0);
                            let (_, protocol) = Protocol::iter()
                                .enumerate()
                                .find(|(i, _)| *i == idx)
                                .unwrap();
                            let session = Session {
                                id: String::new(),
                                name: this.name.read(cx).value().to_string(),
                                group: this.group.read(cx).value().to_string(),
                                host: this.host.read(cx).value().to_string(),
                                port: this
                                    .port
                                    .read(cx)
                                    .value()
                                    .to_string()
                                    .parse::<u16>()
                                    .unwrap_or(22),
                                username: this.port.read(cx).value().to_string(),
                                protocol: protocol,
                                auth: session_manager::AuthMethod::Password,
                                identity_file: Some(this.identity.read(cx).value().to_string()),
                                status: SessionStatus::Disconnected,
                                latencies_ms: vec![],
                            };
                            this.session_manager.update(cx, |this, cx| {
                                this.add(session);
                            })
                        }),
                    ),
                )
                .child(div().id("__state_ref").child(state_to_hidden_marker())),
        )
}

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
        .bg(t.colors().background)
        .border_1()
        .border_color(t.colors().border)
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
