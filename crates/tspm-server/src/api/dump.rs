use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use crate::AppState;
use super::utils::*;

pub async fn get_dump() -> Json<Wrapped<serde_json::Value>> {
    let p = tspm_deploy::PersistenceManager::new();
    let procs = p.load().map(|d| d.processes).unwrap_or_default();
    wrap(serde_json::json!({"processes": procs}))
}

pub async fn put_dump(
    State(state): State<Arc<AppState>>, Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let procs: Vec<tspm_core::ProcessConfig> = serde_json::from_value(body["processes"].clone())
        .map_err(|e| bad_req(e.to_string()))?;
    tspm_deploy::PersistenceManager::new().save(&procs).map_err(|e| server_err(e.to_string()))?;
    let mut mgr = state.manager.lock().await;
    for p in &procs { let _ = mgr.add_process(p.clone()).await; }
    Ok(flat_json(serde_json::json!({"message": format!("Dump updated with {} process(es)", procs.len()), "data": {"processes": procs}})))
}

pub async fn patch_dump(Path(name): Path<String>, Json(_body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    flat_json(serde_json::json!({"message": format!("Process {name} updated in dump")}))
}

pub async fn delete_dump(Path(name): Path<String>) -> Json<serde_json::Value> {
    if name.is_empty() {
        return flat_json(serde_json::json!({"message": "Process name is required"}));
    }
    flat_json(serde_json::json!({"message": format!("Process {name} removed from dump")}))
}

pub async fn delete_empty_dump() -> Json<serde_json::Value> {
    flat_json(serde_json::json!({"message": "Process name is required"}))
}
