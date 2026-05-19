use axum::{Router, response::IntoResponse, routing::get};
use lazy_static::lazy_static;
use prometheus::{
    Encoder, HistogramOpts, HistogramTimer, HistogramVec, IntCounterVec, TextEncoder,
};
use std::net::SocketAddr;

lazy_static! {
    pub static ref REQUEST_COUNTER: IntCounterVec = prometheus::register_int_counter_vec!(
        "bookcloud_requests_total",
        "Total number of requests handled by BookCloud services",
        &["service", "operation", "status"]
    )
    .expect("failed to register request counter");
    pub static ref REQUEST_DURATION: HistogramVec = prometheus::register_histogram_vec!(
        HistogramOpts::new(
            "bookcloud_request_duration_seconds",
            "Request duration in seconds for BookCloud services"
        ),
        &["service", "operation"]
    )
    .expect("failed to register request duration histogram");
}

pub fn record_success(service: &str, operation: &str) {
    REQUEST_COUNTER
        .with_label_values(&[service, operation, "success"])
        .inc();
}

pub fn record_error(service: &str, operation: &str) {
    REQUEST_COUNTER
        .with_label_values(&[service, operation, "error"])
        .inc();
}

pub fn start_timer(service: &str, operation: &str) -> HistogramTimer {
    REQUEST_DURATION
        .with_label_values(&[service, operation])
        .start_timer()
}

async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();

    if let Err(error) = encoder.encode(&metric_families, &mut buffer) {
        return format!("failed to encode metrics: {error}");
    }

    match String::from_utf8(buffer) {
        Ok(metrics) => metrics,
        Err(error) => format!("failed to convert metrics to utf8: {error}"),
    }
}

pub async fn start_metrics_server(service_name: &'static str) {
    let app = Router::new().route("/metrics", get(metrics_handler));
    let addr = SocketAddr::from(([0, 0, 0, 0], 9100));

    println!("{service_name} metrics server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind metrics server");

    axum::serve(listener, app)
        .await
        .expect("metrics server failed");
}
