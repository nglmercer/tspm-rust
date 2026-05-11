use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, sse::{Event, KeepAlive, Sse}},
    Json,
};
use std::sync::Arc;
use std::convert::Infallible;
use tokio::io::AsyncBufReadExt;
use crate::AppState;
use super::utils::*;
use super::processes::LogsQuery;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ExecuteBody {
    pub command: String,
    pub cwd: Option<String>,
    pub stream: Option<bool>,
}

#[derive(Deserialize)]
pub struct AutocompleteBody {
    pub prefix: String,
    pub cwd: Option<String>,
}

pub async fn get_status(State(state): State<Arc<AppState>>) -> Json<Wrapped<serde_json::Value>> {
    let mgr = state.manager.lock().await;
    let statuses = mgr.get_statuses();
    let metrics = collect_stats(&state.stats, &statuses).await;
    let procs: Vec<serde_json::Value> = statuses.iter().enumerate().map(|(i, s)| {
        let (cpu, mem) = metrics.get(i).copied().unwrap_or((0.0, 0));
        proc_json(s, cpu, mem)
    }).collect();
    wrap(serde_json::json!({
        "processes": procs,
        "stats": { "totalProcesses": procs.len(), "clusters": mgr.cluster_count(), "version": tspm_core::APP_VERSION }
    }))
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> Json<Wrapped<serde_json::Value>> {
    let mgr = state.manager.lock().await;
    let statuses = mgr.get_statuses();
    let (mut running, mut stopped, mut errored) = (0u32, 0u32, 0u32);
    for s in &statuses {
        match s.state { 
            tspm_core::ProcessState::Running => running += 1, 
            tspm_core::ProcessState::Errored => errored += 1, 
            _ => stopped += 1 
        }
    }

    let sys_stats = state.stats.get_system_stats().await.unwrap_or_default();

    wrap(serde_json::json!({
        "cpu": sys_stats.cpu_percent,
        "memory": sys_stats.memory_used_bytes,
        "memoryTotal": sys_stats.memory_total_bytes,
        "processes": { "total": statuses.len(), "running": running, "stopped": stopped, "errored": errored },
        "clusters": mgr.cluster_count(), 
        "uptime": sys_stats.uptime_secs,
        "processCount": statuses.len(),
        "cwd": std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default()
    }))
}

pub async fn get_all_logs(
    State(state): State<Arc<AppState>>, Query(query): Query<LogsQuery>,
) -> Json<Wrapped<serde_json::Value>> {
    let mgr = state.manager.lock().await;
    let limit = query.limit.unwrap_or(200);
    let mut all: Vec<serde_json::Value> = Vec::new();
    for s in mgr.get_statuses() {
        if let Some(proc) = mgr.get_process(&s.name) {
            let stdout_path = proc.config().stdout.clone().or_else(|| Some(tspm_core::get_default_log_path(&s.name, "logs")));
            if let Some(path) = stdout_path {
                if path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let lines: Vec<&str> = content.lines().collect();
                        let start = lines.len().saturating_sub(limit);
                        for line in &lines[start..] {
                            all.push(serde_json::json!({"timestamp":"","type":"stdout","message":strip_ansi(line),"processName":s.name}));
                        }
                    }
                }
            }
        }
    }
    all.truncate(limit);
    wrap(serde_json::json!({"logs":all}))
}

pub async fn execute_command(
    Json(body): Json<ExecuteBody>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let cwd = body.cwd.clone().unwrap_or_else(|| ".".into());
    let stream_mode = body.stream.unwrap_or(true);

    if !stream_mode {
        let output = tokio::process::Command::new("sh").arg("-c").arg(&body.command)
            .current_dir(&cwd).output().await
            .map_err(|e| server_err(e.to_string()))?;
        return Ok(axum::response::Response::builder()
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(serde_json::json!({
                "success": true,
                "output": String::from_utf8_lossy(&output.stdout),
                "error": String::from_utf8_lossy(&output.stderr),
                "exitCode": output.status.code().unwrap_or(-1),
                "cwd": cwd,
            }).to_string()))
            .unwrap());
    }

    let cmd = body.command.clone();
    let cwd2 = cwd.clone();
    let stream = async_stream::stream! {
        if cmd.starts_with("cd ") || cmd == "cd" {
            let target = if cmd == "cd" {
                std::env::var("HOME").unwrap_or_else(|_| "/".into())
            } else {
                cmd[3..].trim().replace('~', &std::env::var("HOME").unwrap_or_else(|_| "/".into()))
            };
            let abs = std::path::Path::new(&cwd2).join(&target);
            let abs = std::fs::canonicalize(&abs).unwrap_or(abs);
            if abs.is_dir() {
                yield Ok::<Event, Infallible>(Event::default().event("cwd").data(abs.display().to_string()));
                yield Ok(Event::default().event("complete").data(r#"{"success":true,"exitCode":0,"error":""}"#));
            } else {
                yield Ok(Event::default().event("complete").data(format!(r#"{{"success":false,"exitCode":1,"error":"cd: {}: Not a directory"}}"#, target)));
            }
            return;
        }

        let mut child = match tokio::process::Command::new("sh")
            .arg("-c").arg(&cmd)
            .current_dir(&cwd2)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                yield Ok(Event::default().event("complete").data(format!(r#"{{"success":false,"exitCode":null,"error":"{}"}}"#, e)));
                return;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let data = serde_json::json!({"type":"stdout","data": strip_ansi(&line)});
                yield Ok(Event::default().event("output").data(serde_json::to_string(&data).unwrap_or_default()));
            }
        }

        if let Some(stderr) = child.stderr.take() {
            let mut reader = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let data = serde_json::json!({"type":"stderr","data": strip_ansi(&line)});
                yield Ok(Event::default().event("output").data(serde_json::to_string(&data).unwrap_or_default()));
            }
        }

        let status = child.wait().await;
        let exit_code = status.as_ref().ok().and_then(|s| s.code());
        let success = status.map(|s| s.success()).unwrap_or(false);
        yield Ok(Event::default().event("complete").data(format!(
            r#"{{"success":{},"exitCode":{},"error":""}}"#,
            success,
            exit_code.map_or("null".into(), |c| c.to_string())
        )));
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()).into_response())
}

pub async fn autocomplete(Json(body): Json<AutocompleteBody>) -> Json<serde_json::Value> {
    let cwd = body.cwd.unwrap_or_else(|| ".".into());
    let prefix = &body.prefix;
    let known_commands = [
        "ls", "cd", "pwd", "echo", "cat", "grep", "find", "mkdir", "rm", "cp", "mv",
        "touch", "chmod", "chown", "ps", "kill", "top", "df", "du", "tar", "gzip",
        "git", "npm", "npx", "yarn", "pnpm", "bun", "node", "python", "python3",
        "cargo", "rustc", "make", "cmake", "docker", "kubectl", "ssh", "scp", "curl",
        "wget", "vim", "nano", "code", "clear", "history", "which", "whoami",
        "tspm", "systemctl", "journalctl", "cargo run", "bun run", "npm run",
    ];

    let mut search_dir = std::path::PathBuf::from(&cwd);
    let mut file_prefix = prefix.to_string();

    if let Some(last_sep) = prefix.rfind('/') {
        let dir_part = &prefix[..=last_sep];
        file_prefix = prefix[last_sep + 1..].to_string();
        let resolved_dir = if dir_part.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
            dir_part.replacen('~', &home, 1)
        } else { dir_part.to_string() };
        let joined = std::path::Path::new(&resolved_dir);
        search_dir = if joined.is_absolute() { joined.to_path_buf() } else { std::path::PathBuf::from(&cwd).join(resolved_dir) };
    } else if prefix.starts_with('~') {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
        let resolved = if prefix.len() > 1 { prefix.replacen('~', &home, 1) } else { home };
        search_dir = std::path::PathBuf::from(&resolved);
        file_prefix = if prefix.len() == 1 { String::new() } else { prefix[2..].to_string() };
    }

    if let Ok(canon) = std::fs::canonicalize(&search_dir) { search_dir = canon; }

    let mut suggestions: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&file_prefix) {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                suggestions.push(if is_dir { format!("{name}/") } else { name });
            }
        }
    }

    suggestions.sort_by(|a, b| b.ends_with('/').cmp(&a.ends_with('/')).then(a.cmp(b)));

    if !prefix.contains('/') && !prefix.contains('\\') {
        let mut combined: Vec<String> = known_commands.iter().filter(|c| c.starts_with(prefix)).map(|c| c.to_string()).collect();
        combined.extend(suggestions);
        suggestions = combined;
    }
    suggestions.truncate(20);
    flat_json(serde_json::json!({"suggestions": suggestions}))
}

pub async fn health_check() -> Json<serde_json::Value> {
    flat_json(serde_json::json!({"status":"healthy","timestamp":""}))
}
