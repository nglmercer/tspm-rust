use std::collections::HashMap;
use std::sync::Mutex;
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
            Some(ProcessMetrics { cpu_percent: 0.0, memory_bytes: 0, uptime_secs: 0, pid: Some(pid) })
        }
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

/// Legacy stateless API — returns average CPU since process start
pub struct ProcessStats;

impl ProcessStats {
    pub async fn get(pid: u32) -> Option<ProcessMetrics> {
        #[cfg(target_os = "linux")]
        {
            let stat = tokio::fs::read_to_string(format!("/proc/{pid}/stat")).await.ok()?;
            let parts: Vec<&str> = stat.split_whitespace().collect();
            let utime: u64 = parts.get(13)?.parse().ok()?;
            let stime: u64 = parts.get(14)?.parse().ok()?;
            let _total_time = utime + stime;

            let status = tokio::fs::read_to_string(format!("/proc/{pid}/status")).await.ok()?;
            let mut rss = 0u64;
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    rss = line.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024;
                    break;
                }
            }

            let uptime_str = tokio::fs::read_to_string("/proc/uptime").await.ok()?;
            let uptime_secs: u64 = uptime_str.split_whitespace().next()?.split('.').next()?.parse().ok()?;
            let starttime: u64 = parts.get(21)?.parse().ok()?;
            let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 };
            let proc_uptime = if clk_tck > 0 { uptime_secs.saturating_sub(starttime / clk_tck) } else { 0 };

            Some(ProcessMetrics { cpu_percent: 0.0, memory_bytes: rss, uptime_secs: proc_uptime, pid: Some(pid) })
        }
        #[cfg(not(target_os = "linux"))]
        { Some(ProcessMetrics { cpu_percent: 0.0, memory_bytes: 0, uptime_secs: 0, pid: Some(pid) }) }
    }
}
