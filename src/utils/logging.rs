use tracing::{info, warn, error};
use tracing_subscriber;

pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();
}

pub fn log_info(msg: &str) {
    info!("{}", msg);
}

pub fn log_warn(msg: &str) {
    warn!("{}", msg);
}

pub fn log_error(msg: &str) {
    error!("{}", msg);
}
