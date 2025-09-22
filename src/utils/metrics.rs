use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

/// Metrics registry (simple, Prometheus-style)
#[derive(Clone)]
pub struct MetricsRegistry {
    counters: Arc<Mutex<HashMap<String, u64>>>,
    gauges: Arc<Mutex<HashMap<String, f64>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            gauges: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn inc_counter(&self, name: &str) {
        let mut counters = self.counters.lock().unwrap();
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn set_gauge(&self, name: &str, val: f64) {
        let mut gauges = self.gauges.lock().unwrap();
        gauges.insert(name.to_string(), val);
    }

    pub fn snapshot(&self) -> (HashMap<String, u64>, HashMap<String, f64>) {
        (
            self.counters.lock().unwrap().clone(),
            self.gauges.lock().unwrap().clone(),
        )
    }
}

lazy_static! {
    pub static ref METRICS: MetricsRegistry = MetricsRegistry::new();
}
