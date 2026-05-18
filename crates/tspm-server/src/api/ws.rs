use axum::extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}};
use axum::response::IntoResponse;
use std::sync::Arc;
use crate::AppState;
use super::utils::*;

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("[TSPM] WebSocket client connected");

    let mut tick = 0u64;
    let mut file_positions: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    loop {
        let manager = state.manager.lock().await;
        let statuses = manager.get_statuses();

        let metrics = collect_stats(&state.stats, &statuses).await;

        let processes: Vec<serde_json::Value> = statuses.iter().enumerate().map(|(i, s)| {
            let (cpu, mem) = metrics.get(i).copied().unwrap_or((0.0, 0));
            proc_json(s, cpu, mem)
        }).collect();

        let update = serde_json::json!({"type":"process:update","payload":{"processes":processes}});
        if socket.send(Message::Text(update.to_string().into())).await.is_err() { break; }

        // Tail logs...
        for s in &statuses {
            if let Some(proc) = manager.get_process(&s.name) {
                let paths = vec![
                    (resolve_stdout(proc), "stdout"),
                    (resolve_stderr(proc), "stderr"),
                ];

                for (path_opt, log_type) in paths {
                    if let Some(path) = path_opt {
                        let path_str = path.display().to_string();
                        if let Ok(meta) = std::fs::metadata(&path) {
                            let current_size = meta.len();
                            let last_pos = file_positions.get(&path_str).copied().unwrap_or(0);

                            if current_size > last_pos {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let new_content = if last_pos == 0 {
                                        let lines: Vec<&str> = content.lines().collect();
                                        let start = lines.len().saturating_sub(20);
                                        lines[start..].join("\n")
                                    } else {
                                        content[last_pos as usize..].to_string()
                                    };

                                    for line in new_content.lines() {
                                        if line.is_empty() { continue; }
                                        let timestamp = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .ok()
                                            .map(|d| {
                                                let total_secs = d.as_secs();
                                                let h = (total_secs % 86400) / 3600;
                                                let m = (total_secs % 3600) / 60;
                                                let s = total_secs % 60;
                                                let ms = (d.subsec_millis() as u64) / 100;
                                                format!("{:02}:{:02}:{:02}.{}", h, m, s, ms)
                                            })
                                            .unwrap_or_default();
                                        let log_msg = serde_json::json!({
                                            "type": "process:log",
                                            "payload": {
                                                "timestamp": timestamp,
                                                "message": line,
                                                "type": log_type,
                                                "processName": s.name,
                                            }
                                        });
                                        if socket.send(Message::Text(log_msg.to_string().into())).await.is_err() { break; }
                                    }
                                    file_positions.insert(path_str, current_size);
                                }
                            }
                        }
                    }
                }
            }
        }

        drop(manager);

        // system:stats every 4s (tick % 2 because loop sleeps 2s)
        if tick % 2 == 0 {
            if let Some(mut sys_stats) = state.stats.get_system_stats().await {
                sys_stats.process_count = statuses.len();
                let stats_msg = serde_json::json!({
                    "type": "system:stats",
                    "payload": {
                        "cpu": sys_stats.cpu_percent,
                        "memory": sys_stats.memory_used_bytes,
                        "memoryTotal": sys_stats.memory_total_bytes,
                        "uptime": sys_stats.uptime_secs,
                        "processCount": sys_stats.process_count,
                    }
                });
                if socket.send(Message::Text(stats_msg.to_string().into())).await.is_err() { break; }
            }
        }

        tick += 1;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

fn resolve_stdout(proc: &tspm_engine::ManagedProcess) -> Option<std::path::PathBuf> {
    proc.config().stdout.clone()
        .or_else(|| Some(tspm_core::get_default_log_path(&proc.config().name, "logs")))
}

fn resolve_stderr(proc: &tspm_engine::ManagedProcess) -> Option<std::path::PathBuf> {
    proc.config().stderr.clone()
        .or_else(|| Some(tspm_core::get_default_err_log_path(&proc.config().name, "logs")))
}
