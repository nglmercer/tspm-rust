use axum::Router;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::ServeDir};
use crate::api::{self, AppState};
use tspm_monitor::StatsCollector;

pub async fn start_dashboard(
    manager: Arc<tokio::sync::Mutex<tspm_engine::ProcessManager>>,
    port: u16,
    host: &str,
) -> Result<(), String> {
    let state = Arc::new(AppState { manager, stats: StatsCollector::new() });
    let api_router = api::build_router();

    let web_dir = find_web_dir();

    if !web_dir.join("index.html").exists() {
        tracing::warn!(
            "[TSPM] Web dashboard not built. Run: bun build src/web/public/index.html --outdir dist/public"
        );
    }

    let app = Router::new()
        .merge(api_router)
        .layer(CorsLayer::permissive())
        .fallback_service(ServeDir::new(&web_dir).append_index_html_on_directories(true))
        .with_state(state);

    let addr = format!("{host}:{port}");
    tracing::info!("[TSPM] Dashboard: http://{addr}");
    tracing::info!("[TSPM] Serving web assets from: {}", web_dir.display());

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind {addr}: {e}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {e}"))?;

    Ok(())
}

fn find_web_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("dist/web"),           // Preact build output
        PathBuf::from("dist/public"),        // Legacy Lit build
        PathBuf::from("../dist/web"),
        PathBuf::from("../dist/public"),
    ];

    for path in &candidates {
        if path.join("index.html").exists() {
            return path.clone();
        }
    }

    PathBuf::from("../dist/web")
}
