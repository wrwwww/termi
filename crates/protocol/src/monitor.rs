use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct MetricSnapshot {
    pub timestamp: std::time::Instant,

    pub cpu_usage: f32,

    pub memory_used: u64,

    pub memory_total: u64,

    pub load_1m: f32,

    pub disk_used: u64,

    pub disk_total: u64,

    pub network_rx: u64,

    pub network_tx: u64,
}
pub struct MonitorStore {
    current: Option<MetricSnapshot>,

    history: VecDeque<MetricSnapshot>,
}
