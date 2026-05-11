use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
use rust_embed::RustEmbed;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use crate::api::{self, AppState};
use tspm_monitor::StatsCollector;

#[derive(RustEmbed)]
#[folder = "../../dist/web/"]
struct Assets;

pub async fn start_dashboard(
    manager: Arc<tokio::sync::Mutex<tspm_engine::ProcessManager>>,
    port: u16,
    host: &str,
) -> Result<(), String> {
    let state = Arc::new(AppState { manager, stats: StatsCollector::new() });
    let api_router = api::build_router();

    let app = Router::new()
        .merge(api_router)
        .layer(CorsLayer::permissive())
        .fallback(static_handler)
        .with_state(state);

    let addr = format!("{host}:{port}");
    tracing::info!("[TSPM] Dashboard (Embedded): http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind {addr}: {e}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {e}"))?;

    Ok(())
}

async fn static_handler(
    uri: axum::http::Uri,
) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == "index.html" {
        return index_html().await;
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data))
                .unwrap()
        }
        None => {
            // Fallback to index.html for SPA routing
            if !path.contains('.') {
                return index_html().await;
            }
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

async fn index_html() -> Response {
    match Assets::get("index.html") {
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(content.data))
            .unwrap(),
        None => {
            tracing::error!("index.html not found in embedded assets");
            (StatusCode::NOT_FOUND, "Dashboard not built").into_response()
        }
    }
}
