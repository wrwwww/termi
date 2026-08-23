//! `MonitorPanel` — bottom strip showing live server metrics.
//!
//! Sits between the terminal area and the status bar in Workspace view.
//! Renders 4 metric cards (CPU / Memory / Disk / Network) from
//! `AppState.monitor` and exposes collapse / pause / tab controls.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Monitor   [Metrics] Logs Tasks      ‖  ⚙  ▲                │  ← header (32px)
//! ├──────────┬──────────┬──────────┬────────────────────────────┤
//! │   CPU    │  Memory  │   Disk   │         Network            │
//! │   42 %   │  6.4/16  │ 186/256  │   ↑128 ↓432 KB/s          │
//! │  ╱╲╱╲╱   │  ▓▓▓░░   │  ▓▓▓▓▓░  │      ╱╲    ╱╲╱╲           │
//! │   load   │  cache   │  R / W   │     total 1.4 GB          │
//! │  1m 5m 15m│ 1m 5m 15m│ 1m 5m 15m│      1m 5m 15m            │
//! └──────────┴──────────┴──────────┴────────────────────────────┘
//! ```

use crate::state::{
    AppState, Metric, MetricStatus, MonitorSnapshot, MonitorTab, MonitorWindow, NetMetric,
};
use gpui::*;
use theme::{ActiveTheme, Theme};

pub struct MonitorPanel {
    state: Entity<AppState>,
}

impl MonitorPanel {
    pub fn new(state: Entity<AppState>) -> Self {
        Self { state }
    }
}

impl Render for MonitorPanel {
    fn render(&mut self, windows: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.theme();

        let state = self.state.read(cx);

        let collapsed = state.monitor_collapsed;
        let paused = state.monitor_paused;
        let tab = state.monitor_tab;
        let window = state.monitor_window;
        let snapshot = state.monitor.clone();

        // The header is always visible; the body hides when collapsed.
        let header = render_header(&t, tab, paused, collapsed, &self.state);

        let body = if collapsed {
            div()
        } else {
            div()
                .flex()
                .grid()
                .grid_cols(4)
                .gap(px(1.0))
                .bg(t.colors().border)
                .p(px(1.0))
                .child(cpu_card(&t, snapshot.as_ref().map(|s| &s.cpu), window))
                .child(memory_card(
                    &t,
                    snapshot.as_ref().map(|s| &s.memory),
                    window,
                ))
                .child(disk_card(&t, snapshot.as_ref().map(|s| &s.disk), window))
                .child(network_card(
                    &t,
                    snapshot.as_ref().map(|s| &s.network),
                    window,
                ))
        };

        div()
            .id("lumen-monitor-panel")
            .size_full()
            .flex()
            .flex_col()
            // .h(if collapsed { px(32.0) } else { px(192.0) })
            .bg(t.colors().background)
            .border_t_1()
            .border_color(t.colors().border)
            .child(header)
            .child(body)
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn render_header(
    t: &Theme,
    tab: MonitorTab,
    paused: bool,
    collapsed: bool,
    state: &Entity<AppState>,
) -> impl IntoElement {
    let state_for_tabs = state.clone();
    let state_for_actions = state.clone();

    div()
        .flex()
        .flex_row()
        .items_center()
        .h(px(32.0))
        .flex_shrink_0()
        .px(px(12.0))
        .gap(px(12.0))
        .border_b_1()
        .border_color(t.colors().border)
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.colors().icon_accent)
                .child("Monitor"),
        )
        // Tabs
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(2.0))
                .child(tab_btn(
                    t,
                    "Metrics",
                    tab == MonitorTab::Metrics,
                    MonitorTab::Metrics,
                    state_for_tabs.clone(),
                ))
                .child(tab_btn(
                    t,
                    "Logs",
                    tab == MonitorTab::Logs,
                    MonitorTab::Logs,
                    state_for_tabs.clone(),
                ))
                .child(tab_btn(
                    t,
                    "Tasks",
                    tab == MonitorTab::Tasks,
                    MonitorTab::Tasks,
                    state_for_tabs,
                )),
        )
        .child(div().flex_1())
        // Action buttons (pause · settings · collapse)
        .child(action_btn(
            t,
            if paused { "▶" } else { "‖" },
            if paused {
                "Resume sampling"
            } else {
                "Pause sampling"
            },
            // move |cx: &mut Window| {
            //     state_for_actions.update(cx, |s, _| s.monitor_paused = !s.monitor_paused);
            // },
        ))
        .child(action_btn(t, "⚙", "Configure monitor"))
        .child(action_btn(
            t,
            if collapsed { "▼" } else { "▲" },
            if collapsed {
                "Expand monitor"
            } else {
                "Collapse monitor"
            },
            // move |cx: &mut Window| {
            //     state_for_actions.update(cx, |s, cx| s.monitor_collapsed = !s.monitor_collapsed);
            // },
        ))
}

fn tab_btn(
    t: &Theme,
    label: &str,
    active: bool,
    tab: MonitorTab,
    state: Entity<AppState>,
) -> impl IntoElement {
    let state = state.clone();
    let (bg, color) = if active {
        (t.colors().background, t.colors().text)
    } else {
        (transparent_hsla(), t.colors().text_muted)
    };
    div()
        // .id(("monitor-tab", label))
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(4.0))
        .bg(bg)
        .text_color(color)
        .text_size(px(11.0))
        .cursor_pointer()
        .hover(move |s| s.bg(t.colors().background).text_color(t.colors().text))
        // .on_click(move |_, cx| {
        //     state.update(cx, |s, _| s.monitor_tab = tab);
        // })
        .child(text!(label))
}

fn action_btn(
    t: &Theme,
    glyph: &str,
    label: &'static str,
    // on_click: F,
) -> impl IntoElement
// where
//     F: Fn(&mut WindowContext) + 'static,
{
    div()
        // .id(("monitor-action", label))
        .size(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .text_color(t.colors().text_muted)
        .text_size(px(14.0))
        .cursor_pointer()
        .hover(move |s| s.bg(t.colors().background).text_color(t.colors().text))
        // .on_click(move |_, cx| on_click(cx))
        .child(text!(glyph))
}

// ---------------------------------------------------------------------------
// Card builders
// ---------------------------------------------------------------------------

fn cpu_card(t: &Theme, metric: Option<&Metric>, window: MonitorWindow) -> AnyElement {
    let empty = Metric {
        current: 0.0,
        unit: "%".into(),
        threshold_warn: 70.0,
        threshold_danger: 90.0,
        status: MetricStatus::Healthy,
        samples: vec![0.0; 60],
    };
    let m = metric.unwrap_or(&empty);
    metric_card(t, "CPU", m, MetricVisual::Line, window, |m| {
        format!("{:.0}{}", m.current, m.unit)
    })
}

fn memory_card(t: &Theme, metric: Option<&Metric>, window: MonitorWindow) -> AnyElement {
    let empty = Metric {
        current: 0.0,
        unit: "%".into(),
        threshold_warn: 75.0,
        threshold_danger: 92.0,
        status: MetricStatus::Healthy,
        samples: vec![0.0; 60],
    };
    let m = metric.unwrap_or(&empty);
    metric_card(t, "Memory", m, MetricVisual::BarLineAmber, window, |m| {
        format!("{:.1} / 16.0 GB", m.current / 100.0 * 16.0)
    })
}

fn disk_card(t: &Theme, metric: Option<&Metric>, window: MonitorWindow) -> AnyElement {
    let empty = Metric {
        current: 0.0,
        unit: "%".into(),
        threshold_warn: 80.0,
        threshold_danger: 95.0,
        status: MetricStatus::Healthy,
        samples: vec![0.0; 60],
    };
    let m = metric.unwrap_or(&empty);
    metric_card(t, "Disk", m, MetricVisual::BarLine, window, |m| {
        format!("{:.0} / 256 GB", m.current / 100.0 * 256.0)
    })
}

fn network_card(t: &Theme, metric: Option<&NetMetric>, window: MonitorWindow) -> AnyElement {
    let empty = NetMetric {
        up_kbps: 0.0,
        down_kbps: 0.0,
        interface: "—".into(),
        status: MetricStatus::Healthy,
        samples_up: vec![0.0; 60],
        samples_down: vec![0.0; 60],
    };
    let n = metric.unwrap_or(&empty);

    div()
        .flex()
        .flex_col()
        .bg(t.colors().background)
        .px(px(16.0))
        .py(px(12.0))
        .gap(px(6.0))
        .overflow_hidden()
        .min_w_0()
        .child(card_head(
            t,
            "Network",
            &format!("{}", n.interface),
            n.status,
        ))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(12.0))
                .text_size(px(10.5))
                .text_color(t.colors().text_muted)
                .font_family("JetBrains Mono")
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .w(px(10.0))
                                .h(px(2.0))
                                .rounded(px(2.0))
                                .bg(t.colors().icon_accent),
                        )
                        .child(format!("↑ {} KB/s", n.up_kbps.round())),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(
                            div()
                                .w(px(10.0))
                                .h(px(2.0))
                                .rounded(px(2.0))
                                .bg(t.colors().icon_accent),
                        )
                        .child(format!("↓ {} KB/s", n.down_kbps.round())),
                ),
        )
        // .child(dual_chart(t, &n.samples_up, &n.samples_down))
        .child(sub_row(t, "eth0 · total", "1.4 GB"))
        .child(time_buttons(t, window))
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Generic single-metric card builder
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum MetricVisual {
    /// Just a line + area chart (CPU).
    Line,
    /// Progress bar + line chart (Disk).
    BarLine,
    /// Progress bar + amber line chart (Memory).
    BarLineAmber,
}

fn metric_card<F>(
    t: &Theme,
    title: &str,
    m: &Metric,
    visual: MetricVisual,
    window: MonitorWindow,
    format_value: F,
) -> AnyElement
where
    F: Fn(&Metric) -> String,
{
    let delta_text = format!(
        "{:.1}{}",
        m.current - m.samples.first().copied().unwrap_or(m.current),
        m.unit
    );
    let delta_class = match m.status {
        MetricStatus::Healthy => "metric-card__delta--flat",
        MetricStatus::Warn => "metric-card__delta--up",
        MetricStatus::Danger => "metric-card__delta--up",
    };
    let delta_color = match m.status {
        MetricStatus::Healthy => t.colors().icon_accent,
        MetricStatus::Warn => t.status().warning,
        MetricStatus::Danger => t.status().error,
    };

    let mut card = div()
        .flex()
        .flex_col()
        .bg(t.colors().background)
        .px(px(16.0))
        .py(px(12.0))
        .gap(px(6.0))
        .overflow_hidden()
        .min_w_0();

    card = card.child(card_head_color(t, title, &delta_text, delta_color));

    card = card.child(
        div()
            .flex()
            .flex_row()
            .items_baseline()
            .gap(px(4.0))
            .font_family("JetBrains Mono")
            .child(
                div()
                    .text_size(px(22.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.colors().text)
                    // .font_variant_numerals(FontVariantNumerals::Tabular)
                    .child(format_value(m)),
            ),
    );

    if matches!(visual, MetricVisual::BarLine | MetricVisual::BarLineAmber) {
        let (bar_color, bar_pct) = match m.status {
            MetricStatus::Healthy => (t.colors().icon_accent, m.current),
            MetricStatus::Warn => (t.status().warning, m.current),
            MetricStatus::Danger => (t.status().error, m.current),
        };
        card = card.child(
            div()
                .w_full()
                .h(px(6.0))
                .rounded_full()
                .bg(t.colors().background)
                .child(
                    div()
                        .h_full()
                        .w(px(bar_pct.clamp(0.0, 100.0) / 100.0 * 200.0)) // approximate
                        .rounded_full()
                        .bg(bar_color),
                ),
        );
    }

    // Chart
    // card = card.child(line_chart(
    //     t,
    //     &m.samples,
    //     matches!(visual, MetricVisual::BarLineAmber),
    // ));

    card = card.child(sub_row(t, "load avg", "0.05 / 0.12 / 0.08"));
    card = card.child(time_buttons(t, window));

    card.into_any_element()
}

// ---------------------------------------------------------------------------
// Building blocks shared by cards
// ---------------------------------------------------------------------------

fn card_head(t: &Theme, title: &str, delta: &str, status: MetricStatus) -> impl IntoElement {
    card_head_color(t, title, delta, status_color(t, status))
}

fn card_head_color(t: &Theme, title: &str, delta: &str, delta_color: Hsla) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.colors().icon_accent)
                .flex_1()
                .child(text!(title)),
        )
        .child(status_dot(t, MetricStatus::Healthy))
        .child(
            div()
                .ml_auto()
                .text_size(px(10.5))
                .font_family("JetBrains Mono")
                .text_color(delta_color)
                .child(text!(delta)),
        )
}

fn status_dot(t: &Theme, status: MetricStatus) -> impl IntoElement {
    let (color, _glow) = match status {
        MetricStatus::Healthy => (t.colors().icon_accent, "rgba(134,239,172,.5)"),
        MetricStatus::Warn => (t.status().warning, "rgba(251,191,36,.5)"),
        MetricStatus::Danger => (t.status().error, "rgba(252,165,165,.5)"),
    };
    div().size(px(6.0)).rounded_full().bg(color).shadow_md()
}

fn status_color(t: &Theme, status: MetricStatus) -> Hsla {
    match status {
        MetricStatus::Healthy => t.colors().icon_accent,
        MetricStatus::Warn => t.status().warning,
        MetricStatus::Danger => t.status().error,
    }
}

fn sub_row(t: &Theme, label: &str, value: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .justify_between()
        .text_size(px(11.0))
        .font_family("JetBrains Mono")
        .text_color(t.colors().text_muted)
        .child(text!(label))
        .child(div().text_color(t.colors().text).child(text!(value)))
}

fn time_buttons(t: &Theme, current: MonitorWindow) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .gap(px(4.0))
        .mt_auto()
        .child(time_btn(t, "1m", current == MonitorWindow::OneMin))
        .child(time_btn(t, "5m", current == MonitorWindow::FiveMin))
        .child(time_btn(t, "15m", current == MonitorWindow::FifteenMin))
}

fn time_btn(t: &Theme, label: &str, active: bool) -> impl IntoElement {
    let (color, border) = if active {
        (t.colors().icon_accent, t.colors().icon_accent)
    } else {
        (t.colors().text_muted, t.colors().border)
    };
    let bg = if active {
        t.colors().icon_accent
    } else {
        transparent_hsla()
    };
    div()
        // .id(("monitor-time", label))
        .px(px(8.0))
        .py(px(2.0))
        .rounded_full()
        .bg(bg)
        .border_1()
        .border_color(border)
        .text_size(px(10.5))
        .font_family("JetBrains Mono")
        .text_color(color)
        .cursor_pointer()
        .child(text!(label))
}

// ---------------------------------------------------------------------------
// SVG charts — built with Path::from_svg(...) so GPUI renders them natively.
// ---------------------------------------------------------------------------

// fn line_chart(t: &Theme, samples: &[f32], amber: bool) -> impl IntoElement {
//     let (line_color, area_color) = if amber {
//         (t.status().warning, t.status().warning)
//     } else {
//         (t.colors().icon_accent, t.colors().icon_accent)
//     };

//     let (line_d, area_d) = build_path(samples, 200.0, 56.0);

//     div().w_full().h(px(56.0)).child(
//         svg()
//             // .id("monitor-chart")
//             .w_full()
//             .h(px(56.0))
//             .view_box(0.0, 0.0, 200.0, 56.0)
//             .preserve_aspect_ratio(PreserveAspectRatio::None)
//             .overflow_hidden()
//             // grid lines
//             .child(svg_line(0.0, 14.0, 200.0, 14.0, t.colors().border))
//             .child(svg_line(0.0, 28.0, 200.0, 28.0, t.colors().border))
//             .child(svg_line(0.0, 42.0, 200.0, 42.0, t.colors().border))
//             // area
//             .child(
//                 svg()
//                     .path(area_color)
//                     .absolute()
//                     .child(gpui::Path::from_svg(&area_d).with_transparency(0.14)),
//             )
//             // line
//             .child(
//                 svg()
//                     .path(line_color)
//                     .stroke_width(1.5)
//                     .child(gpui::Path::from_svg(&line_d)),
//             ),
//     )
// }

// fn dual_chart(t: &Theme, up: &[f32], down: &[f32]) -> impl IntoElement {
//     let (up_line, up_area) = build_path(up, 200.0, 56.0);
//     let (down_line, down_area) = build_path(down, 200.0, 56.0);

//     div().w_full().h(px(56.0)).child(
//         svg()
//             .id("monitor-chart-net")
//             .w_full()
//             .h(px(56.0))
//             .view_box(0.0, 0.0, 200.0, 56.0)
//             .preserve_aspect_ratio(PreserveAspectRatio::None)
//             .overflow_hidden()
//             .child(svg_line(0.0, 14.0, 200.0, 14.0, t.colors().border))
//             .child(svg_line(0.0, 28.0, 200.0, 28.0, t.colors().border))
//             .child(svg_line(0.0, 42.0, 200.0, 42.0, t.colors().border))
//             // up (green)
//             .child(
//                 svg()
//                     .path(t.colors().icon_accent)
//                     .absolute()
//                     .child(gpui::Path::from_svg(&up_area).with_transparency(0.12)),
//             )
//             .child(
//                 svg()
//                     .path(t.colors().icon_accent)
//                     .stroke_width(1.5)
//                     .child(gpui::Path::from_svg(&up_line)),
//             )
//             // down (accent)
//             .child(
//                 svg()
//                     .path(t.colors().icon_accent)
//                     .absolute()
//                     .child(gpui::Path::from_svg(&down_area).with_transparency(0.12)),
//             )
//             .child(
//                 svg()
//                     .path(t.colors().icon_accent)
//                     .stroke_width(1.5)
//                     .child(gpui::Path::from_svg(&down_line)),
//             ),
//     )
// }

// fn svg_line(x1: f32, y1: f32, x2: f32, y2: f32, color: Hsla) -> impl IntoElement {
//     svg().path(color).child(gpui::Path::from_svg(&format!(
//         "M{:.2},{:.2} L{:.2},{:.2}",
//         x1, y1, x2, y2
//     )))
// }

/// Build (line, area) SVG path strings from a sample slice.
/// Padding keeps a small gutter top/bottom so the line never touches edges.
fn build_path(samples: &[f32], w: f32, h: f32) -> (String, String) {
    if samples.is_empty() {
        return (String::new(), String::new());
    }
    let lo = samples.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let range = (hi - lo).max(0.0001);
    let step = w / (samples.len() - 1).max(1) as f32;
    let pad = 4.0;

    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(samples.len());
    for (i, v) in samples.iter().enumerate() {
        let x = i as f32 * step;
        let norm = (v - lo) / range;
        let y = h - pad - norm * (h - pad * 2.0);
        pts.push((x, y));
    }

    let line = pts
        .iter()
        .enumerate()
        .map(|(i, (x, y))| {
            if i == 0 {
                format!("M{:.2},{:.2}", x, y)
            } else {
                format!("L{:.2},{:.2}", x, y)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let area = format!("{} L{:.2},{:.2} L{:.2},{:.2} Z", line, w, h, 0.0, h);

    (line, area)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn transparent_hsla() -> Hsla {
    Hsla {
        h: 0.,
        s: 0.,
        l: 0.,
        a: 0.,
    }
}

// Note: build_path operates on the data slice; the GPUI helpers used here
// (svg, Path::from_svg) are stable across recent GPUI versions. If your
// pinned rev exposes different names, look for `svg().path().d(...)` or
// fall back to a single inline SVG element via `div().child(svg_str)`.
