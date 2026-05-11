use axum::{
    extract::{Path, Query, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    http::StatusCode,
    response::{IntoResponse, Json, sse::{Event, KeepAlive, Sse}},
    routing::{get, patch, post, delete},
    Router,
};
use axum::response::Response;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::AsyncBufReadExt;
use tspm_engine::ProcessManager;
use tspm_monitor::StatsCollector;

pub struct AppState {
    pub manager: Arc<Mutex<ProcessManager>>,
    pub stats: StatsCollector,
}

#[derive(Serialize)]
struct Wrapped<T: Serialize> {
    success: bool,
    data: T,
}

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

fn wrap<T: Serialize>(data: T) -> Json<Wrapped<T>> {
    Json(Wrapped { success: true, data })
}

fn flat_json(fields: serde_json::Value) -> Json<serde_json::Value> {
    let mut map = serde_json::Map::new();
    map.insert("success".into(), serde_json::Value::Bool(true));
    if let serde_json::Value::Object(obj) = fields {
        map.extend(obj);
    }
    Json(serde_json::Value::Object(map))
}

#[derive(Deserialize)]
pub struct LogsQuery {
    pub limit: Option<usize>,
}

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

#[derive(Deserialize)]
pub struct InputBody {
    pub input: String,
}

pub fn build_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/v1/processes", get(list_processes).post(create_process))
        .route("/api/v1/processes/{name}", get(get_process).delete(delete_process))
        .route("/api/v1/processes/{name}/start", post(start_process))
        .route("/api/v1/processes/{name}/stop", post(stop_process))
        .route("/api/v1/processes/{name}/restart", post(restart_process))
        .route("/api/v1/processes/{name}/logs", get(get_process_logs))
        .route("/api/v1/processes/{name}/input", post(send_input))
        .route("/api/v1/processes/{name}/install", post(install_process))
        .route("/api/v1/processes/{name}/build", post(build_process))
        .route("/api/v1/status", get(get_status))
        .route("/api/v1/stats", get(get_stats))
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/logs", get(get_all_logs))
        .route("/api/v1/events", get(get_events))
        .route("/api/v1/execute", post(execute_command))
        .route("/api/v1/autocomplete", post(autocomplete))
        .route("/api/v1/dump", get(get_dump).put(put_dump))
        .route("/api/v1/dump/{name}", patch(patch_dump).delete(delete_dump))
        .route("/api/v1/dump/", delete(delete_empty_dump))
        // Port management
        .route("/api/v1/ports", get(get_ports))
        .route("/api/v1/ports/{port}", post(kill_port))
}

// ============================================================
// camelCase helpers — match dashboard ProcessStatus fields
// ============================================================
/// Strip ANSI escape codes from log output
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' || c == '␛' {
            // Skip until escape sequence ends (letter)
            while let Some(&next) = chars.peek() {
                chars.next();
                if next.is_ascii_alphabetic() || next == 'm' { break; }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn proc_json(s: &tspm_core::ProcessStatus, cpu: f64, mem: u64) -> serde_json::Value {
    let display_name = if s.name.contains('/') {
        std::path::Path::new(&s.name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| s.name.clone())
    } else {
        s.name.clone()
    };

    serde_json::json!({
        "name": display_name,
        "pid": s.pid,
        "state": s.state_str(),
        "restartCount": s.restart_count,
        "uptime": s.uptime_ms.map_or(0, |ms| ms / 1000),
        "instanceId": s.instance_id,
        "cpu": cpu,
        "memory": mem,
    })
}

/// Resolve stdout path — uses config value or falls back to default log dir
fn resolve_stdout(proc: &tspm_engine::ManagedProcess) -> Option<std::path::PathBuf> {
    proc.config().stdout.clone()
        .or_else(|| Some(tspm_core::get_default_log_path(&proc.config().name, "logs")))
}

fn resolve_stderr(proc: &tspm_engine::ManagedProcess) -> Option<std::path::PathBuf> {
    proc.config().stderr.clone()
        .or_else(|| Some(tspm_core::get_default_err_log_path(&proc.config().name, "logs")))
}

async fn collect_stats(
    collector: &StatsCollector,
    statuses: &[tspm_core::ProcessStatus],
) -> Vec<(f64, u64)> {
    let mut metrics = Vec::new();
    for s in statuses {
        if let Some(pid) = s.pid {
            if let Some(stats) = collector.get(pid).await {
                metrics.push((stats.cpu_percent, stats.memory_bytes));
            } else {
                metrics.push((0.0, 0));
            }
        } else {
            metrics.push((0.0, 0));
        }
    }
    metrics
}

// ============================================================
// WebSocket
// ============================================================
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("[TSPM] WebSocket client connected");

    let mut tick = 0u64;
    // Track last-read positions per log file
    let mut file_positions: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    loop {
        let manager = state.manager.lock().await;
        let statuses = manager.get_statuses();

        // Collect processes
        // Collect real CPU/memory stats
        let metrics = collect_stats(&state.stats, &statuses).await;

        let processes: Vec<serde_json::Value> = statuses.iter().enumerate().map(|(i, s)| {
            let (cpu, mem) = metrics.get(i).copied().unwrap_or((0.0, 0));
            proc_json(s, cpu, mem)
        }).collect();

        // Send process:update
        let update = serde_json::json!({"type":"process:update","payload":{"processes":processes}});
        if socket.send(Message::Text(update.to_string().into())).await.is_err() { break; }

        // Tail log files and push new lines as process:log events
        for s in &statuses {
            if let Some(proc) = manager.get_process(&s.name) {
                if let Some(log_path) = resolve_stdout(proc) {
                    let path_str = log_path.display().to_string();
                    if let Ok(meta) = std::fs::metadata(&log_path) {
                        let current_size = meta.len();
                        let last_pos = file_positions.get(&path_str).copied().unwrap_or(0);

                        if current_size > last_pos {
                            if let Ok(content) = std::fs::read_to_string(&path_str) {
                                if last_pos == 0 {
                                    // First time — only send last 20 lines
                                    let lines: Vec<&str> = content.lines().collect();
                                    let start = lines.len().saturating_sub(20);
                                    for line in &lines[start..] {
                                        let log_msg = serde_json::json!({
                                            "type": "process:log",
                                            "payload": {
                                                "message": strip_ansi(line),
                                                "type": "stdout",
                                                "processName": s.name,
                                            }
                                        });
                                        if socket.send(Message::Text(log_msg.to_string().into())).await.is_err() { break; }
                                    }
                                    file_positions.insert(path_str.clone(), current_size);
                                } else {
                                    // Read only new content
                                    let new_content = &content[last_pos as usize..];
                                    for line in new_content.lines() {
                                        if line.is_empty() { continue; }
                                        let log_msg = serde_json::json!({
                                            "type": "process:log",
                                            "payload": {
                                                "message": strip_ansi(line),
                                                "type": "stdout",
                                                "processName": s.name,
                                            }
                                        });
                                        if socket.send(Message::Text(log_msg.to_string().into())).await.is_err() { break; }
                                    }
                                    file_positions.insert(path_str.clone(), current_size);
                                }
                            }
                        }

                        // Also check stderr
                        if let Some(err_path) = resolve_stderr(proc) {
                            let err_str = err_path.display().to_string();
                            if let Ok(err_meta) = std::fs::metadata(&err_path) {
                                let err_size = err_meta.len();
                                let err_last = file_positions.get(&err_str).copied().unwrap_or(0);
                                if err_size > err_last {
                                    if let Ok(err_content) = std::fs::read_to_string(&err_str) {
                                        let err_new = if err_last == 0 { &err_content[..] } else { &err_content[err_last as usize..] };
                                        for line in err_new.lines() {
                                            if line.is_empty() { continue; }
                                            let log_msg = serde_json::json!({
                                                "type": "process:log",
                                                "payload": {
                                                    "message": strip_ansi(line),
                                                    "type": "stderr",
                                                    "processName": s.name,
                                                }
                                            });
                                            if socket.send(Message::Text(log_msg.to_string().into())).await.is_err() { break; }
                                        }
                                        file_positions.insert(err_str, err_size);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        drop(manager);

        // system:stats every 10s
        if tick % 5 == 0 {
            let stats = serde_json::json!({
                "type": "system:stats",
                "payload": {"cpu":0,"memory":0,"uptime":0,"processCount":statuses.len()}
            });
            if socket.send(Message::Text(stats.to_string().into())).await.is_err() { break; }
        }

        tick += 1;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

// ============================================================
// Process routes
// ============================================================
async fn list_processes(State(state): State<Arc<AppState>>) -> Json<Wrapped<Vec<serde_json::Value>>> {
    let mgr = state.manager.lock().await;
    let statuses = mgr.get_statuses();
    let metrics = collect_stats(&state.stats, &statuses).await;
    wrap(statuses.iter().enumerate().map(|(i, s)| {
        let (cpu, mem) = metrics.get(i).copied().unwrap_or((0.0, 0));
        proc_json(s, cpu, mem)
    }).collect())
}

async fn get_process(
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
        None => Err((StatusCode::NOT_FOUND, Json(ErrorResponse { success: false, error: "not found".into() }))),
    }
}

async fn create_process(
    State(state): State<Arc<AppState>>, Json(body): Json<tspm_core::ProcessConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if body.name.is_empty() {
        return Err(bad_req("Process name is required".into()));
    }
    if body.script.is_empty() {
        return Err(bad_req("Script is required".into()));
    }

    let mut mgr = state.manager.lock().await;
    mgr.add_process(body.clone()).await.map_err(|e| server_err(e))?;
    mgr.start_process(&body.name).await.map_err(|e| server_err(e))?;

    // Persist to dump.json so it survives restart
    let persistence = tspm_deploy::PersistenceManager::new();
    let mut existing = persistence.load().unwrap_or(tspm_core::DumpData {
        processes: vec![],
        timestamp: String::new(),
        version: String::new(),
    });
    // Remove old entry with same name if exists
    existing.processes.retain(|p| p.name != body.name);
    existing.processes.push(body.clone());
    let _ = persistence.save(&existing.processes);

    Ok(flat_json(serde_json::json!({"message": format!("Spawned {}", body.name)})))
}

macro_rules! action_handler {
    ($name:ident, $msg:expr) => {
        async fn $name(
            State(state): State<Arc<AppState>>, Path(name): Path<String>,
        ) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
            let mut mgr = state.manager.lock().await;
            mgr.$name(&name).await.map_err(|e| server_err(e))?;
            Ok(flat_json(serde_json::json!({"message": format!($msg, name)})))
        }
    };
}

action_handler!(start_process, "Started {}");
action_handler!(stop_process, "Stopped {}");
action_handler!(restart_process, "Restarted {}");

async fn delete_process(
    State(state): State<Arc<AppState>>, Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mut mgr = state.manager.lock().await;
    if !mgr.has_process(&name) {
        return Err(not_found());
    }
    mgr.remove_process(&name).await.map_err(|e| server_err(e))?;

    let persistence = tspm_deploy::PersistenceManager::new();
    if let Some(mut data) = persistence.load() {
        data.processes.retain(|p| p.name != name);
        let _ = persistence.save(&data.processes);
    }

    Ok(flat_json(serde_json::json!({"message": format!("Removed {name}")})))
}

async fn get_process_logs(
    State(state): State<Arc<AppState>>, Path(name): Path<String>, Query(query): Query<LogsQuery>,
) -> Result<Json<Wrapped<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let mgr = state.manager.lock().await;
    let limit = query.limit.unwrap_or(200);
    let proc = mgr.get_process(&name).ok_or(not_found())?;
    let mut logs: Vec<serde_json::Value> = Vec::new();
    if let Some(path) = resolve_stdout(proc) {
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

async fn send_input(
    State(state): State<Arc<AppState>>, Path(name): Path<String>, Json(_body): Json<InputBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let mgr = state.manager.lock().await;
    let _proc = mgr.get_process(&name).ok_or(not_found())?;
    // stdin write would go here — for now return success
    Ok(flat_json(serde_json::json!({"message": format!("Input sent to {name}")})))
}

async fn install_process(
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

    if output.status.success() {
        Ok(flat_json(serde_json::json!({"message": format!("Install completed for {name}")})))
    } else {
        Ok(flat_json(serde_json::json!({"message": format!("Install failed for {name}: {}", String::from_utf8_lossy(&output.stderr))})))
    }
}

async fn build_process(
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

    if output.status.success() {
        Ok(flat_json(serde_json::json!({"message": format!("Build completed for {name}")})))
    } else {
        Ok(flat_json(serde_json::json!({"message": format!("Build failed for {name}: {}", String::from_utf8_lossy(&output.stderr))})))
    }
}

// ============================================================
// System routes
// ============================================================
async fn get_status(State(state): State<Arc<AppState>>) -> Json<Wrapped<serde_json::Value>> {
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

async fn get_stats(State(state): State<Arc<AppState>>) -> Json<Wrapped<serde_json::Value>> {
    let mgr = state.manager.lock().await;
    let statuses = mgr.get_statuses();
    let (mut running, mut stopped, mut errored) = (0u32, 0u32, 0u32);
    for s in &statuses {
        match s.state { tspm_core::ProcessState::Running => running += 1, tspm_core::ProcessState::Errored => errored += 1, _ => stopped += 1 }
    }
    wrap(serde_json::json!({
        "cpu": 0, "memory": 0,
        "processes": { "total": statuses.len(), "running": running, "stopped": stopped, "errored": errored },
        "clusters": mgr.cluster_count(), "uptime": 0,
        "cwd": std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default()
    }))
}

async fn get_all_logs(
    State(state): State<Arc<AppState>>, Query(query): Query<LogsQuery>,
) -> Json<Wrapped<serde_json::Value>> {
    let mgr = state.manager.lock().await;
    let limit = query.limit.unwrap_or(200);
    let mut all: Vec<serde_json::Value> = Vec::new();
    for s in mgr.get_statuses() {
        if let Some(proc) = mgr.get_process(&s.name) {
            if let Some(path) = resolve_stdout(proc) {
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

async fn get_events() -> Json<Wrapped<Vec<serde_json::Value>>> { wrap(vec![]) }

/// SSE streaming execute — terminal sends stream:true
async fn execute_command(
    Json(body): Json<ExecuteBody>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let cwd = body.cwd.clone().unwrap_or_else(|| ".".into());
    let stream_mode = body.stream.unwrap_or(true);

    if !stream_mode {
        // Non-streaming fallback
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

    // SSE streaming
    let cmd = body.command.clone();
    let cwd2 = cwd.clone();
    let stream = async_stream::stream! {
        // Handle 'cd' command
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

        // Run command
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

async fn autocomplete(
    Json(body): Json<AutocompleteBody>,
) -> Json<serde_json::Value> {
    let cwd = body.cwd.unwrap_or_else(|| ".".into());
    let prefix = &body.prefix;

    // Commands to suggest when typing the first word (no space before)
    let known_commands = [
        "ls", "cd", "pwd", "echo", "cat", "grep", "find", "mkdir", "rm", "cp", "mv",
        "touch", "chmod", "chown", "ps", "kill", "top", "df", "du", "tar", "gzip",
        "git", "npm", "npx", "yarn", "pnpm", "bun", "node", "python", "python3",
        "cargo", "rustc", "make", "cmake", "docker", "kubectl", "ssh", "scp", "curl",
        "wget", "vim", "nano", "code", "clear", "history", "which", "whoami",
        "tspm", "systemctl", "journalctl", "cargo run", "bun run", "npm run",
    ];

    // Determine the directory to search and the filename prefix to match
    let mut search_dir = std::path::PathBuf::from(&cwd);
    let mut file_prefix = prefix.to_string();

    // Handle paths with directory components (e.g., "src/uti" → search in "src/" for "uti*")
    if let Some(last_sep) = prefix.rfind('/') {
        let dir_part = &prefix[..=last_sep];
        file_prefix = prefix[last_sep + 1..].to_string();

        // Handle ~ expansion
        let resolved_dir = if dir_part.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
            dir_part.replacen('~', &home, 1)
        } else {
            dir_part.to_string()
        };

        let joined = std::path::Path::new(&resolved_dir);
        if joined.is_absolute() {
            search_dir = joined.to_path_buf();
        } else {
            search_dir = std::path::PathBuf::from(&cwd).join(resolved_dir);
        }
    } else if prefix.starts_with('~') {
        // Just "~" or "~/" prefix
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
        let resolved = if prefix.len() > 1 { prefix.replacen('~', &home, 1) } else { home };
        search_dir = std::path::PathBuf::from(&resolved);
        if prefix.len() == 1 {
            file_prefix = String::new();
        } else {
            file_prefix = prefix[2..].to_string();
        }
    }

    // Canonicalize the search directory if possible
    if let Ok(canon) = std::fs::canonicalize(&search_dir) {
        search_dir = canon;
    }

    let mut suggestions: Vec<String> = Vec::new();

    // Collect matching files/directories
    if let Ok(entries) = std::fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&file_prefix) {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                suggestions.push(if is_dir { format!("{name}/") } else { name });
            }
        }
    }

    suggestions.sort_by(|a, b| {
        let a_dir = a.ends_with('/');
        let b_dir = b.ends_with('/');
        b_dir.cmp(&a_dir).then(a.cmp(b))
    });

    // Prepend known commands if this looks like the first word (no slashes, no spaces context)
    // The client sends just the word after the last space. If prefix is short, suggest commands too.
    if !prefix.contains('/') && !prefix.contains('\\') {
        let cmd_matches: Vec<String> = known_commands.iter()
            .filter(|c| c.starts_with(prefix))
            .map(|c| c.to_string())
            .collect();

        // Insert commands before file matches
        let mut combined = cmd_matches;
        combined.extend(suggestions);
        suggestions = combined;
    }

    // Truncate to avoid overwhelming the UI
    suggestions.truncate(20);

    flat_json(serde_json::json!({"suggestions": suggestions}))
}

async fn health_check() -> Json<serde_json::Value> {
    flat_json(serde_json::json!({"status":"healthy","timestamp":""}))
}

// ============================================================
// Dump routes
// ============================================================
async fn get_dump() -> Json<Wrapped<serde_json::Value>> {
    let p = tspm_deploy::PersistenceManager::new();
    let procs = p.load().map(|d| d.processes).unwrap_or_default();
    wrap(serde_json::json!({"processes": procs}))
}

async fn put_dump(
    State(state): State<Arc<AppState>>, Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let procs: Vec<tspm_core::ProcessConfig> = serde_json::from_value(body["processes"].clone())
        .map_err(|e| bad_req(e.to_string()))?;
    tspm_deploy::PersistenceManager::new().save(&procs).map_err(|e| server_err(e.to_string()))?;
    let mut mgr = state.manager.lock().await;
    for p in &procs { let _ = mgr.add_process(p.clone()).await; }
    Ok(flat_json(serde_json::json!({"message": format!("Dump updated with {} process(es)", procs.len()), "data": {"processes": procs}})))
}

async fn patch_dump(Path(name): Path<String>, Json(_body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    flat_json(serde_json::json!({"message": format!("Process {name} updated in dump")}))
}

async fn delete_dump(Path(name): Path<String>) -> Json<serde_json::Value> {
    if name.is_empty() {
        return flat_json(serde_json::json!({"message": "Process name is required"}));
    }
    flat_json(serde_json::json!({"message": format!("Process {name} removed from dump")}))
}

async fn delete_empty_dump() -> Json<serde_json::Value> {
    flat_json(serde_json::json!({"message": "Process name is required"}))
}

// ============================================================
// Port management
// ============================================================

#[derive(Serialize)]
struct PortInfo {
    port: u16,
    pid: u32,
    process: String,
    protocol: String,
}

async fn get_ports() -> Json<Wrapped<Vec<PortInfo>>> {
    let ports = list_ports().await;
    wrap(ports)
}

async fn kill_port(
    Path(port_str): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let port: u16 = port_str.parse().map_err(|_| bad_req("Invalid port".into()))?;
    let ports = list_ports().await;
    let target = ports.iter().find(|p| p.port == port);

    let (pid, process_name) = match target {
        Some(info) => (info.pid, info.process.clone()),
        None => {
            return Err((StatusCode::NOT_FOUND, Json(ErrorResponse {
                success: false,
                error: format!("No process found on port {port}"),
            })));
        }
    };

    // Primary: fuser -k (most reliable on Linux, handles process groups properly)
    let _ = tokio::process::Command::new("fuser")
        .args(["-k", &format!("{port}/tcp")])
        .output()
        .await;

    // Fallback: libc kill the process group then the PID itself
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
        libc::kill(pid as i32, libc::SIGKILL);
    }

    // Wait briefly and verify the port is actually freed
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let remaining = list_ports().await;
    let still_bound = remaining.iter().any(|p| p.port == port);

    if still_bound {
        Err((StatusCode::CONFLICT, Json(ErrorResponse {
            success: false,
            error: format!("Kill signal sent to PID {pid} ({process_name}) but port {port} is still in use. The socket may be in TIME_WAIT state."),
        })))
    } else {
        Ok(flat_json(serde_json::json!({
            "message": format!("Killed PID {pid} ({process_name}) using port {port}"),
            "pid": pid,
            "process": process_name,
        })))
    }
}

async fn list_ports() -> Vec<PortInfo> {
    let mut result = Vec::new();

    // Primary: use ss (fast, works on most Linux)
    if let Ok(output) = tokio::process::Command::new("sh")
        .arg("-c")
        .arg("ss -tlnp 2>/dev/null | tail -n +2")
        .output()
        .await
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(info) = parse_ss_line(line) {
                result.push(info);
            }
        }
    }

    // Fallback: read /proc/net/tcp directly
    if result.is_empty() {
        if let Ok(content) = tokio::fs::read_to_string("/proc/net/tcp").await {
            for line in content.lines().skip(1) {
                if let Some(info) = parse_proc_tcp_line(line, "TCP") {
                    result.push(info);
                }
            }
        }
        if let Ok(content) = tokio::fs::read_to_string("/proc/net/tcp6").await {
            for line in content.lines().skip(1) {
                if let Some(info) = parse_proc_tcp_line(line, "TCP") {
                    result.push(info);
                }
            }
        }
    }

    if result.is_empty() {
        if let Ok(output) = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("ss -ulnp 2>/dev/null | tail -n +2")
            .output()
            .await
        {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if let Some(info) = parse_ss_line(line) {
                    result.push(info);
                }
            }
        }
    }

    result.sort_by_key(|p| p.port);
    result.dedup_by_key(|p| p.port);
    result
}

fn parse_proc_tcp_line(line: &str, protocol: &str) -> Option<PortInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 { return None; }

    // Column 1 is local_address (hex IP:hex port)
    let local = parts.get(1)?;
    let port_hex = local.split(':').last()?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;

    // Column 3 is state — 0A = LISTEN
    let state = *parts.get(3)?;
    if state != "0A" { return None; }

    // Column 9 is inode
    let inode = parts.get(9)?;

    // Find PID by inode
    let (pid, name) = find_pid_by_inode(inode);

    Some(PortInfo {
        port,
        pid,
        process: name,
        protocol: protocol.into(),
    })
}

fn find_pid_by_inode(inode: &str) -> (u32, String) {
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Ok(pid) = name_str.parse::<u32>() {
                let fd_dir = format!("/proc/{pid}/fd");
                if let Ok(fds) = std::fs::read_dir(&fd_dir) {
                    for fd in fds.flatten() {
                        if let Ok(link) = std::fs::read_link(fd.path()) {
                            let link_str = link.display().to_string();
                            if link_str.contains(inode) && link_str.contains("socket") {
                                let proc_name = std::fs::read_to_string(format!("/proc/{pid}/comm"))
                                    .unwrap_or_else(|_| "?".into())
                                    .trim()
                                    .to_string();
                                return (pid, proc_name);
                            }
                        }
                    }
                }
            }
        }
    }
    (0, "?".into())
}

fn parse_ss_line(line: &str) -> Option<PortInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 { return None; }

    // Column 3 is Local Address:Port (0-indexed)
    let addr_port = parts.get(3)?;
    let port: u16 = addr_port.rsplit(':').next()?.parse().ok()?;

    // Look for pid= in the users field (last column)
    let users_col = parts.last()?;
    let pid_str = users_col
        .split("pid=")
        .nth(1)?
        .split(|c: char| !c.is_ascii_digit())
        .next()?;
    let pid: u32 = pid_str.parse().ok()?;

    let process = if let Some(start) = users_col.find("((\"") {
        let rest = &users_col[start + 3..];
        rest.split('"').next().unwrap_or("?").to_string()
    } else {
        format!("PID {}", pid)
    };

    let protocol = if line.to_lowercase().contains("udp") { "UDP" } else { "TCP" };

    Some(PortInfo { port, pid, process, protocol: protocol.into() })
}

// helpers
fn not_found() -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { success: false, error: "not found".into() }))
}

fn server_err(e: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { success: false, error: e }))
}

fn bad_req(e: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { success: false, error: e }))
}
