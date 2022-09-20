use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

pub fn init_telemetry() {
    LogTracer::init().expect("Failed to set logger");
    let fmt_layer = fmt::layer();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let subscriber = Registry::default().with(filter_layer).with(fmt_layer);

    set_global_default(subscriber).expect("Failed to set subscriber");
}
