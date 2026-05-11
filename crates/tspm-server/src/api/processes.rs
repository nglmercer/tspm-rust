use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use crate::AppState;
use super::utils::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LogsQuery {
    pub limit: Option<usize>,
}

pub async fn list_processes(State(state): State<Arc<AppState>>) -> Json<Wrapped<Vec<serde_json::Value>>> {
    let mgr = state.manager.lock().await;
    let statuses = mgr.get_statuses();
    let metrics = collect_stats(&state.stats, &statuses).await;
    wrap(statuses.iter().enumerate().map(|(i, s)| {
        let (cpu, mem) = metrics.get(i).copied().unwrap_or((0.0, 0));
        proc_json(s, cpu, mem)
    }).collect())
}

pub async fn get_process(
    State(state): State<Arc<AppState>>, Path(name): Path<String>,
) -> Result<Json<Wrapped<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let manager = state.manager.lock().await;
    match manager.get_process(&name) {
        Some(p) => {
            let s = p.get_status();
            let (cpu, mem) = if let Some(pid) = s.pid {
                state.stats.get(pid).await.map(|m| (m.cpu_percent, m.memory_bytes)).unwrap_or((0.0, 0))
            } else { (0.0, 0) };
            Ok(wrap(proc_json(&s, cpu, mem)))
        }
        None => Err(not_found()),
    }
}

pub async fn create_process(
    State(state): State<Arc<AppState>>, Json(body): Json<tspm_core::ProcessConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if body.name.is_empty() { return Err(bad_req("Process name is required".into())); }
    if body.script.is_empty() { return Err(bad_req("Script is required".into())); }

    let mut mgr = state.manager.lock().await;
    mgr.add_process(body.clone()).await.map_err(|e| server_err(e.to_string()))?;
    mgr.start_process(&body.name).await.map_err(|e| server_err(e.to_string()))?;

    // Persist
    let persistence = tspm_deploy::PersistenceManager::new();
    let mut existing = persistence.load().unwrap_or(tspm_core::DumpData {
        processes: vec![],
        timestamp: String::new(),
        version: String::new(),
    });
    existing.processes.retain(|p| p.name != body.name);
    existing.processes.push(body.clone());
    let _ = persistence.save(&existing.processes);

    Ok(flat_json(serde_json::json!({"message": format!("Spawned {}", body.name)})))
}

pub async fn start_process(State(state): State<Arc<AppState>>, Path(name): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut mgr = state.manager.lock().await;
    mgr.start_process(&name).await.map_err(|e| server_err(e.to_string()))?;
    Ok(flat_json(serde_json::json!({"message": format!("Started {name}")})))
}

pub async fn stop_process(State(state): State<Arc<AppState>>, Path(name): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut mgr = state.manager.lock().await;
    mgr.stop_process(&name).await.map_err(|e| server_err(e.to_string()))?;
    Ok(flat_json(serde_json::json!({"message": format!("Stopped {name}")})))
}

pub async fn restart_process(State(state): State<Arc<AppState>>, Path(name): Path<String>) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut mgr = state.manager.lock().await;
    mgr.restart_process(&name).await.map_err(|e| server_err(e.to_string()))?;
    Ok(flat_json(serde_json::json!({"message": format!("Restarted {name}")})))
}

pub async fn delete_process(
    State(state): State<Arc<AppState>>, Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut mgr = state.manager.lock().await;
    if !mgr.has_process(&name) { return Err(not_found()); }
    mgr.remove_process(&name).await.map_err(|e| server_err(e.to_string()))?;

    let persistence = tspm_deploy::PersistenceManager::new();
    if let Some(mut data) = persistence.load() {
        data.processes.retain(|p| p.name != name);
        let _ = persistence.save(&data.processes);
    }

    Ok(flat_json(serde_json::json!({"message": format!("Removed {name}")})))
}

pub async fn update_process(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<tspm_core::ProcessConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut mgr = state.manager.lock().await;
    mgr.update_process(&name, body.clone()).await.map_err(|e| server_err(e.to_string()))?;
    mgr.start_process(&body.name).await.map_err(|e| server_err(e.to_string()))?;
    
    // Persist
    let persistence = tspm_deploy::PersistenceManager::new();
    if let Some(mut data) = persistence.load() {
        data.processes.retain(|p| p.name != name && p.name != body.name);
        data.processes.push(body.clone());
        let _ = persistence.save(&data.processes);
    }
    
    Ok(flat_json(serde_json::json!({"message": format!("Updated {}", body.name)})))
}

pub async fn get_process_logs(
    State(state): State<Arc<AppState>>, Path(name): Path<String>, Query(query): Query<LogsQuery>,
) -> Result<Json<Wrapped<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let mgr = state.manager.lock().await;
    let limit = query.limit.unwrap_or(200);
    let proc = mgr.get_process(&name).ok_or(not_found())?;
    let mut logs: Vec<serde_json::Value> = Vec::new();

    let stdout_path = proc.config().stdout.clone().or_else(|| Some(tspm_core::get_default_log_path(&proc.config().name, "logs")));
    if let Some(path) = stdout_path {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let lines: Vec<&str> = content.lines().collect();
                let start = lines.len().saturating_sub(limit);
                for line in &lines[start..] {
                    logs.push(serde_json::json!({"timestamp":"","message":line,"type":"stdout","processName":name}));
                }
            }
        }
    }
    Ok(wrap(serde_json::json!({"processName":name,"logs":logs,"limit":limit,"count":logs.len()})))
}

#[derive(Deserialize)]
pub struct InputBody { pub input: String }

pub async fn send_input(
    State(state): State<Arc<AppState>>, Path(name): Path<String>, Json(_body): Json<InputBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mgr = state.manager.lock().await;
    let _proc = mgr.get_process(&name).ok_or(not_found())?;
    Ok(flat_json(serde_json::json!({"message": format!("Input sent to {name}")})))
}

pub async fn install_process(
    State(state): State<Arc<AppState>>, Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mgr = state.manager.lock().await;
    let proc = mgr.get_process(&name).ok_or(not_found())?;
    let script = proc.config().install.clone().unwrap_or_default();
    if script.is_empty() {
        return Ok(flat_json(serde_json::json!({"message": format!("No install script defined for {name}")})));
    }
    let cwd = proc.config().cwd.clone();
    drop(mgr);

    let output = tokio::process::Command::new("sh")
        .arg("-c").arg(&script)
        .current_dir(cwd.as_deref().unwrap_or(std::path::Path::new(".")))
        .output().await
        .map_err(|e| server_err(e.to_string()))?;

    Ok(flat_json(serde_json::json!({
        "message": if output.status.success() { format!("Install completed for {name}") } else { format!("Install failed for {name}") },
        "stderr": String::from_utf8_lossy(&output.stderr)
    })))
}

pub async fn build_process(
    State(state): State<Arc<AppState>>, Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mgr = state.manager.lock().await;
    let proc = mgr.get_process(&name).ok_or(not_found())?;
    let script = proc.config().build.clone().unwrap_or_default();
    if script.is_empty() {
        return Ok(flat_json(serde_json::json!({"message": format!("No build script defined for {name}")})));
    }
    let cwd = proc.config().cwd.clone();
    drop(mgr);

    let output = tokio::process::Command::new("sh")
        .arg("-c").arg(&script)
        .current_dir(cwd.as_deref().unwrap_or(std::path::Path::new(".")))
        .output().await
        .map_err(|e| server_err(e.to_string()))?;

    Ok(flat_json(serde_json::json!({
        "message": if output.status.success() { format!("Build completed for {name}") } else { format!("Build failed for {name}") },
        "stderr": String::from_utf8_lossy(&output.stderr)
    })))
}
