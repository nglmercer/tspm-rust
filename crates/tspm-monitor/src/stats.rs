use std::collections::HashMap;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tspm_core::ProcessMetrics;

/// Previous CPU sample for delta calculation
#[derive(Debug, Clone)]
struct CpuSample {
    total_jiffies: u64,
    wall_clock_ms: u64,
}

/// Tracks per-PID CPU samples for delta-based percentage calculation
pub struct StatsCollector {
    samples: Mutex<HashMap<u32, CpuSample>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub uptime_secs: u64,
    pub process_count: usize,
}

impl StatsCollector {
    pub fn new() -> Self {
        Self { samples: Mutex::new(HashMap::new()) }
    }

    /// Get real-time metrics for a PID using delta-based CPU calculation.
    /// Aggregates stats across the process and its children.
    pub async fn get(&self, pid: u32) -> Option<ProcessMetrics> {
        #[cfg(target_os = "linux")]
        {
            let (cpu_total, rss, proc_uptime) = self.collect_tree_stats(pid).await?;
            let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 };
            if clk_tck == 0 { return None; }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let mut samples = self.samples.lock().unwrap();
            let cpu_percent = if let Some(prev) = samples.get(&pid) {
                let delta_jiffies = cpu_total.saturating_sub(prev.total_jiffies);
                let delta_ms = now.saturating_sub(prev.wall_clock_ms);
                if delta_ms > 0 {
                    let cpu_seconds = delta_jiffies as f64 / clk_tck as f64;
                    (cpu_seconds / (delta_ms as f64 / 1000.0)) * 100.0
                } else {
                    0.0
                }
            } else {
                0.0
            };

            samples.insert(pid, CpuSample { total_jiffies: cpu_total, wall_clock_ms: now });

            Some(ProcessMetrics {
                cpu_percent,
                memory_bytes: rss,
                uptime_secs: proc_uptime,
                pid: Some(pid),
            })
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = pid;
            Some(ProcessMetrics { cpu_percent: 0.0, memory_bytes: 0, uptime_secs: 0, pid: Some(pid) })
        }
    }

    pub async fn get_system_stats(&self) -> Option<SystemMetrics> {
        #[cfg(target_os = "linux")]
        {
            let cpu = self.read_system_cpu().await.unwrap_or(0.0);
            let (mem_used, mem_total) = self.read_system_memory().await.unwrap_or((0, 0));
            let uptime = tokio::fs::read_to_string("/proc/uptime").await.ok()?
                .split_whitespace().next()?.split('.').next()?.parse::<u64>().ok()?;

            Some(SystemMetrics {
                cpu_percent: cpu,
                memory_used_bytes: mem_used,
                memory_total_bytes: mem_total,
                uptime_secs: uptime,
                process_count: 0, // Will be filled by the caller (manager)
            })
        }
        #[cfg(not(target_os = "linux"))]
        { None }
    }

    #[cfg(target_os = "linux")]
    async fn read_system_cpu(&self) -> Option<f64> {
        let stat = tokio::fs::read_to_string("/proc/stat").await.ok()?;
        let line = stat.lines().next()?;
        let parts: Vec<u64> = line.split_whitespace().skip(1).filter_map(|s| s.parse().ok()).collect();
        if parts.len() < 4 { return None; }

        let idle = parts[3];
        let total: u64 = parts.iter().sum();

        // Need delta for real percentage
        let mut samples = self.samples.lock().unwrap();
        // Use a special key (u32::MAX) for system CPU
        let res = if let Some(prev) = samples.get(&u32::MAX) {
            let prev_idle = prev.total_jiffies; // Hijack fields for system stats
            let prev_total = prev.wall_clock_ms;

            let delta_idle = idle.saturating_sub(prev_idle);
            let delta_total = total.saturating_sub(prev_total);

            if delta_total > 0 {
                let usage = 100.0 * (1.0 - (delta_idle as f64 / delta_total as f64));
                usage.max(0.0).min(100.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        samples.insert(u32::MAX, CpuSample { total_jiffies: idle, wall_clock_ms: total });
        Some(res)
    }

    #[cfg(target_os = "linux")]
    async fn read_system_memory(&self) -> Option<(u64, u64)> {
        let meminfo = tokio::fs::read_to_string("/proc/meminfo").await.ok()?;
        let mut total = 0u64;
        let mut free = 0u64;
        let mut buffers = 0u64;
        let mut cached = 0u64;

        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total = line.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024;
            } else if line.starts_with("MemFree:") {
                free = line.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024;
            } else if line.starts_with("Buffers:") {
                buffers = line.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024;
            } else if line.starts_with("Cached:") {
                cached = line.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024;
            }
        }

        let used = total.saturating_sub(free).saturating_sub(buffers).saturating_sub(cached);
        Some((used, total))
    }

    #[cfg(target_os = "linux")]
    async fn collect_tree_stats(&self, pid: u32) -> Option<(u64, u64, u64)> {
        let (jiffies, rss) = self.read_proc_stat(pid).await?;
        let mut total_jiffies = jiffies;
        let mut total_rss = rss;

        // Collect child processes
        if let Ok(children) = tokio::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")).await {
            for child_pid_str in children.split_whitespace() {
                if let Ok(child_pid) = child_pid_str.parse::<u32>() {
                    if let Some((child_jiffies, child_rss)) = self.read_proc_stat(child_pid).await {
                        total_jiffies += child_jiffies;
                        total_rss += child_rss;
                    }
                }
            }
        }

        let uptime = tokio::fs::read_to_string("/proc/uptime").await.ok()?;
        let sys_uptime: u64 = uptime.split_whitespace().next()?.split('.').next()?.parse().ok()?;

        let stat = tokio::fs::read_to_string(format!("/proc/{pid}/stat")).await.ok()?;
        let parts: Vec<&str> = stat.split_whitespace().collect();
        let starttime: u64 = parts.get(21)?.parse().ok()?;
        let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 };
        let proc_uptime = if clk_tck > 0 { sys_uptime.saturating_sub(starttime / clk_tck) } else { 0 };

        Some((total_jiffies, total_rss, proc_uptime))
    }

    #[cfg(target_os = "linux")]
    async fn read_proc_stat(&self, pid: u32) -> Option<(u64, u64)> {
        let stat = tokio::fs::read_to_string(format!("/proc/{pid}/stat")).await.ok()?;
        let parts: Vec<&str> = stat.split_whitespace().collect();
        let utime: u64 = parts.get(13)?.parse().ok()?;
        let stime: u64 = parts.get(14)?.parse().ok()?;

        let status = tokio::fs::read_to_string(format!("/proc/{pid}/status")).await.ok()?;
        let mut rss = 0u64;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                rss = line.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024;
                break;
            }
        }

        Some((utime + stime, rss))
    }
}

impl Default for StatsCollector {
    fn default() -> Self { Self::new() }
}
