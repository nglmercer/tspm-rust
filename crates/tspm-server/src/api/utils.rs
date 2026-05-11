use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct Wrapped<T: Serialize> {
    pub success: bool,
    pub data: T,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

pub fn wrap<T: Serialize>(data: T) -> Json<Wrapped<T>> {
    Json(Wrapped { success: true, data })
}

pub fn flat_json(fields: serde_json::Value) -> Json<serde_json::Value> {
    let mut map = serde_json::Map::new();
    map.insert("success".into(), serde_json::Value::Bool(true));
    if let serde_json::Value::Object(obj) = fields {
        map.extend(obj);
    }
    Json(serde_json::Value::Object(map))
}

pub fn not_found() -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { success: false, error: "not found".into() }))
}

pub fn server_err(e: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { success: false, error: e }))
}

pub fn bad_req(e: String) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { success: false, error: e }))
}

pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' || c == '␛' {
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

pub fn proc_json(s: &tspm_core::ProcessStatus, cpu: f64, mem: u64) -> serde_json::Value {
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

pub async fn collect_stats(
    collector: &tspm_monitor::StatsCollector,
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
