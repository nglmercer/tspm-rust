use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use tspm_core::HealthCheckConfig;

/// Health check result
#[derive(Debug, Clone)]
pub enum HealthCheckResult {
    Healthy,
    Unhealthy { reason: String },
}

/// Runner for periodic health checks on a process
pub struct HealthCheckRunner {
    config: HealthCheckConfig,
    consecutive_failures: u32,
    is_healthy: bool,
}

impl HealthCheckRunner {
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            consecutive_failures: 0,
            is_healthy: true,
        }
    }

    /// Perform a health check
    pub async fn check(&mut self) -> HealthCheckResult {
        let result = match self.config.protocol {
            tspm_core::HealthCheckProtocol::Http | tspm_core::HealthCheckProtocol::Https => {
                self.check_http().await
            }
            tspm_core::HealthCheckProtocol::Tcp => {
                self.check_tcp().await
            }
            tspm_core::HealthCheckProtocol::Command => {
                self.check_command().await
            }
            tspm_core::HealthCheckProtocol::None => {
                HealthCheckResult::Healthy
            }
        };

        match &result {
            HealthCheckResult::Healthy => {
                self.consecutive_failures = 0;
                self.is_healthy = true;
            }
            HealthCheckResult::Unhealthy { .. } => {
                self.consecutive_failures += 1;
                if self.consecutive_failures >= self.config.retries {
                    self.is_healthy = false;
                }
            }
        }

        result
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy
    }

    async fn check_http(&self) -> HealthCheckResult {
        let scheme = match self.config.protocol {
            tspm_core::HealthCheckProtocol::Https => "https",
            _ => "http",
        };

        let host = self.config.host.as_deref().unwrap_or("localhost");
        let port = self.config.port.unwrap_or(80);
        let path = self.config.path.as_deref().unwrap_or("/health");
        let url = format!("{scheme}://{host}:{port}{path}");

        let timeout = Duration::from_millis(self.config.timeout_ms);

        match tokio::time::timeout(timeout, reqwest::get(&url)).await {
            Ok(Ok(resp)) if resp.status().is_success() => HealthCheckResult::Healthy,
            Ok(Ok(resp)) => HealthCheckResult::Unhealthy {
                reason: format!("HTTP {}", resp.status()),
            },
            Ok(Err(e)) => HealthCheckResult::Unhealthy {
                reason: format!("HTTP error: {e}"),
            },
            Err(_) => HealthCheckResult::Unhealthy {
                reason: "Timeout".to_string(),
            },
        }
    }

    async fn check_tcp(&self) -> HealthCheckResult {
        let host = self.config.host.as_deref().unwrap_or("localhost");
        let port = self.config.port.unwrap_or(80);
        let addr = format!("{host}:{port}");

        let timeout = Duration::from_millis(self.config.timeout_ms);

        match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => HealthCheckResult::Healthy,
            Ok(Err(e)) => HealthCheckResult::Unhealthy {
                reason: format!("TCP error: {e}"),
            },
            Err(_) => HealthCheckResult::Unhealthy {
                reason: "Timeout".to_string(),
            },
        }
    }

    async fn check_command(&self) -> HealthCheckResult {
        let cmd = match &self.config.command {
            Some(cmd) => cmd.clone(),
            None => return HealthCheckResult::Unhealthy {
                reason: "No command configured".to_string(),
            },
        };

        let timeout = Duration::from_millis(self.config.timeout_ms);

        match tokio::time::timeout(timeout, async {
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
                .await
        }).await {
            Ok(Ok(output)) if output.status.success() => HealthCheckResult::Healthy,
            Ok(Ok(output)) => HealthCheckResult::Unhealthy {
                reason: format!("Command exit code: {:?}", output.status.code()),
            },
            Ok(Err(e)) => HealthCheckResult::Unhealthy {
                reason: format!("Command error: {e}"),
            },
            Err(_) => HealthCheckResult::Unhealthy {
                reason: "Timeout".to_string(),
            },
        }
    }

    /// Run periodic health checks until stopped
    pub async fn run_periodic(&mut self) {
        let interval = Duration::from_millis(self.config.interval_ms);

        // Initial delay
        if self.config.initial_delay_ms > 0 {
            sleep(Duration::from_millis(self.config.initial_delay_ms)).await;
        }

        loop {
            let result = self.check().await;
            match result {
                HealthCheckResult::Healthy => {
                    info!("[TSPM] Health check passed");
                }
                HealthCheckResult::Unhealthy { reason } => {
                    warn!("[TSPM] Health check failed ({} consecutive): {}", self.consecutive_failures, reason);
                }
            }
            sleep(interval).await;
        }
    }
}
