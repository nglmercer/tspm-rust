use axum::{Router, routing::get_service};
use std::path::PathBuf;
use tower_http::services::ServeDir;

/// Start a static file server (like PM2's `serve` command)
pub async fn start_static_server(
    path: PathBuf,
    port: u16,
    host: &str,
) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Directory not found: {}", path.display()));
    }

    let serve_dir = ServeDir::new(&path)
        .append_index_html_on_directories(true);

    let app = Router::new()
        .fallback_service(get_service(serve_dir));

    let addr = format!("{host}:{port}");
    tracing::info!("[TSPM] Static server: http://{} serving {}", addr, path.display());

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {addr}: {e}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {e}"))?;

    Ok(())
}
