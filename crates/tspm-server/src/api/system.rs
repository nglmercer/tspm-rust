use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, sse::{Event, KeepAlive, Sse}},
    Json,
};
use std::sync::Arc;
use std::convert::Infallible;
use tokio::io::AsyncReadExt;
use crate::AppState;
use super::utils::*;
use super::processes::LogsQuery;
use serde::Deserialize;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

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
                            all.push(serde_json::json!({"timestamp":"","type":"stdout","message":line,"processName":s.name}));
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
    let cmd = body.command.clone();

    // Use portable-pty for better terminal support
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| server_err(e.to_string()))?;

    let mut cmd_builder = CommandBuilder::new("sh");
    cmd_builder.arg("-c");
    cmd_builder.arg(&cmd);
    cmd_builder.cwd(&cwd);
    cmd_builder.env("TERM", "xterm-256color");
    cmd_builder.env("CLICOLOR_FORCE", "1");
    cmd_builder.env("FORCE_COLOR", "1");

    let mut _child = pair.slave.spawn_command(cmd_builder).map_err(|e| server_err(e.to_string()))?;
    
    // pair.slave is dropped here, which is important for the PTY to close correctly
    drop(pair.slave);

    let reader = pair.master.try_clone_reader().map_err(|e| server_err(e.to_string()))?;
    let mut reader = std::io::BufReader::new(reader);

    let stream = async_stream::stream! {
        // Special case for cd to keep state in UI
        if cmd.starts_with("cd ") || cmd == "cd" {
            let target = if cmd == "cd" {
                std::env::var("HOME").unwrap_or_else(|_| "/".into())
            } else {
                cmd[3..].trim().replace('~', &std::env::var("HOME").unwrap_or_else(|_| "/".into()))
            };
            let abs = std::path::Path::new(&cwd).join(&target);
            let abs = std::fs::canonicalize(&abs).unwrap_or(abs);
            if abs.is_dir() {
                yield Ok::<Event, Infallible>(Event::default().event("cwd").data(abs.display().to_string()));
                yield Ok(Event::default().event("complete").data(r#"{"success":true,"exitCode":0,"error":""}"#));
            } else {
                yield Ok(Event::default().event("complete").data(format!(r#"{{"success":false,"exitCode":1,"error":"cd: {}: Not a directory"}}"#, target)));
            }
            return;
        }

        // Reading from PTY is blocking, so we use spawn_blocking or a separate thread
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
        
        std::thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            while let Ok(n) = std::io::Read::read(&mut reader, &mut buffer) {
                if n == 0 { break; }
                if tx.blocking_send(buffer[..n].to_vec()).is_err() { break; }
            }
        });

        while let Some(chunk) = rx.recv().await {
            let data = String::from_utf8_lossy(&chunk).to_string();
            let json = serde_json::json!({"type":"stdout","data": data});
            yield Ok(Event::default().event("output").data(json.to_string()));
        }

        yield Ok(Event::default().event("complete").data(r#"{"success":true,"exitCode":0,"error":""}"#));
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
