use axum::{
    routing::{get, post, delete, patch},
    Router,
};
use std::sync::Arc;
use tspm_engine::ProcessManager;
use tspm_monitor::StatsCollector;
use tokio::sync::Mutex;

pub struct AppState {
    pub manager: Arc<Mutex<ProcessManager>>,
    pub stats: StatsCollector,
}

pub mod processes;
pub mod system;
pub mod ports;
pub mod dump;
pub mod ws;
pub mod utils;

pub fn build_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ws", get(ws::ws_handler))
        // Processes
        .route("/api/v1/processes", get(processes::list_processes).post(processes::create_process))
        .route("/api/v1/processes/:name", get(processes::get_process).delete(processes::delete_process))
        .route("/api/v1/processes/:name/start", post(processes::start_process))
        .route("/api/v1/processes/:name/stop", post(processes::stop_process))
        .route("/api/v1/processes/:name/restart", post(processes::restart_process))
        .route("/api/v1/processes/:name/logs", get(processes::get_process_logs))
        .route("/api/v1/processes/:name/input", post(processes::send_input))
        .route("/api/v1/processes/:name/install", post(processes::install_process))
        .route("/api/v1/processes/:name/build", post(processes::build_process))
        // System
        .route("/api/v1/status", get(system::get_status))
        .route("/api/v1/stats", get(system::get_stats))
        .route("/api/v1/health", get(system::health_check))
        .route("/api/v1/logs", get(system::get_all_logs))
        .route("/api/v1/execute", post(system::execute_command))
        .route("/api/v1/autocomplete", post(system::autocomplete))
        // Dump
        .route("/api/v1/dump", get(dump::get_dump).put(dump::put_dump))
        .route("/api/v1/dump/:name", patch(dump::patch_dump).delete(dump::delete_dump))
        .route("/api/v1/dump/", delete(dump::delete_empty_dump))
        // Ports
        .route("/api/v1/ports", get(ports::get_ports))
        .route("/api/v1/ports/:port", post(kill_port))
}

// Re-export kill_port to avoid moving too much code from ports.rs if needed,
// but actually I should use ports::kill_port
async fn kill_port(
    axum::extract::Path(port): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    ports::kill_port(axum::extract::Path(port)).await
}
