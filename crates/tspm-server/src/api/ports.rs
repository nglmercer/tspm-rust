use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use serde::Serialize;
use super::utils::*;

#[derive(Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub process: String,
    pub protocol: String,
}

pub async fn get_ports() -> Json<Wrapped<Vec<PortInfo>>> {
    let ports = list_ports().await;
    wrap(ports)
}

pub async fn kill_port(
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

    let _ = tokio::process::Command::new("fuser")
        .args(["-k", &format!("{port}/tcp")])
        .output()
        .await;

    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGKILL);
        libc::kill(pid as i32, libc::SIGKILL);
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let remaining = list_ports().await;
    let still_bound = remaining.iter().any(|p| p.port == port);

    if still_bound {
        Err((StatusCode::CONFLICT, Json(ErrorResponse {
            success: false,
            error: format!("Kill signal sent to PID {pid} ({process_name}) but port {port} is still in use."),
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
    if let Ok(output) = tokio::process::Command::new("sh")
        .arg("-c").arg("ss -tlnp 2>/dev/null | tail -n +2")
        .output().await
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(info) = parse_ss_line(line) { result.push(info); }
        }
    }
    if result.is_empty() {
        if let Ok(content) = tokio::fs::read_to_string("/proc/net/tcp").await {
            for line in content.lines().skip(1) {
                if let Some(info) = parse_proc_tcp_line(line, "TCP") { result.push(info); }
            }
        }
    }
    result.sort_by_key(|p| p.port);
    result.dedup_by_key(|p| p.port);
    result
}

fn parse_ss_line(line: &str) -> Option<PortInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 { return None; }
    let addr_port = parts.get(3)?;
    let port: u16 = addr_port.rsplit(':').next()?.parse().ok()?;
    let users_col = parts.last()?;
    let pid_str = users_col.split("pid=").nth(1)?.split(|c: char| !c.is_ascii_digit()).next()?;
    let pid: u32 = pid_str.parse().ok()?;
    let process = if let Some(start) = users_col.find("((\"") {
        let rest = &users_col[start + 3..];
        rest.split('"').next().unwrap_or("?").to_string()
    } else { format!("PID {}", pid) };
    let protocol = if line.to_lowercase().contains("udp") { "UDP" } else { "TCP" };
    Some(PortInfo { port, pid, process, protocol: protocol.into() })
}

fn parse_proc_tcp_line(line: &str, protocol: &str) -> Option<PortInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 { return None; }
    let local = parts.get(1)?;
    let port_hex = local.split(':').last()?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    if *parts.get(3)? != "0A" { return None; }
    let inode = parts.get(9)?;
    let (pid, name) = find_pid_by_inode(inode);
    Some(PortInfo { port, pid, process: name, protocol: protocol.into() })
}

fn find_pid_by_inode(inode: &str) -> (u32, String) {
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Ok(pid) = name.to_string_lossy().parse::<u32>() {
                if let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) {
                    for fd in fds.flatten() {
                        if let Ok(link) = std::fs::read_link(fd.path()) {
                            let link_str = link.display().to_string();
                            if link_str.contains(inode) && link_str.contains("socket") {
                                let proc_name = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_else(|_| "?".into()).trim().to_string();
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
