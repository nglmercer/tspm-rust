use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Process lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Errored,
    Restarting,
}

/// Load balancing strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LoadBalanceStrategy {
    #[default]
    RoundRobin,
    Random,
    LeastConnections,
    LeastCpu,
    LeastMemory,
    IpHash,
    Weighted,
}

/// Health check protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckProtocol {
    Http,
    Https,
    Tcp,
    Command,
    #[default]
    None,
}

/// Stop reason constants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopReason {
    Manual,
    Error,
    Signal,
    Unknown,
}

/// Restart reason constants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RestartReason {
    Manual,
    Watch,
    Auto,
    Crash,
    Oom,
}

/// System stop reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemStopReason {
    Manual,
    Signal,
    Error,
}

/// Log type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogType {
    Stdout,
    Stderr,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    Stopping,
    Unknown,
}

/// Exit codes used by TSPM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    Error = 1,
    ConfigNotFound = 2,
    ConfigInvalid = 3,
    ProcessNotFound = 4,
    ProcessStartFailed = 5,
    PermissionDenied = 6,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> Self {
        code as i32
    }
}

// ============================================================
// Configuration types
// ============================================================

/// Watch configuration: either a boolean or a list of patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WatchConfig {
    Bool(bool),
    Patterns(Vec<String>),
}

impl Default for WatchConfig {
    fn default() -> Self {
        WatchConfig::Bool(false)
    }
}

/// Health check configuration for a process
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthCheckConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub protocol: HealthCheckProtocol,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default = "default_health_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_health_interval")]
    pub interval_ms: u64,
    #[serde(default = "default_retries")]
    pub retries: u32,
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub expected_status: Option<u16>,
    #[serde(default)]
    pub response_body: Option<String>,
}

fn default_health_timeout() -> u64 { 5000 }
fn default_health_interval() -> u64 { 30000 }
fn default_retries() -> u32 { 3 }
fn default_initial_delay() -> u64 { 5000 }

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_api_port")]
    pub port: u16,
    #[serde(default)]
    pub host: Option<String>,
}

fn default_api_port() -> u16 { 3000 }

/// Kubernetes configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KubernetesConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub pod_name: Option<String>,
    #[serde(default)]
    pub pod_namespace: Option<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
    #[serde(default)]
    pub container_name: Option<String>,
    #[serde(default)]
    pub liveness_probe: Option<String>,
    #[serde(default)]
    pub readiness_probe: Option<String>,
    #[serde(default)]
    pub startup_probe: Option<String>,
}

/// Docker configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DockerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub container_name: Option<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub restart_policy: Option<String>,
    #[serde(default)]
    pub memory_limit: Option<String>,
    #[serde(default)]
    pub cpu_limit: Option<String>,
}

/// Per-process configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessConfig {
    pub name: String,
    pub script: String,

    // Basic options
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub interpreter: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,

    // Restart options
    #[serde(default = "default_true")]
    pub autorestart: bool,
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    #[serde(default)]
    pub min_restart_delay_ms: u64,
    #[serde(default = "default_max_restart_delay")]
    pub max_restart_delay_ms: u64,
    #[serde(default = "default_backoff")]
    pub restart_backoff: f64,
    #[serde(default)]
    pub restart_delay_ms: Option<u64>,

    // Watch & reload
    #[serde(default)]
    pub watch: WatchConfig,
    #[serde(default)]
    pub ignore_watch: Vec<String>,
    #[serde(default = "default_watch_delay")]
    pub watch_delay_ms: u64,

    // Resource limits
    #[serde(default)]
    pub max_memory_bytes: u64,
    #[serde(default)]
    pub nice: Option<i32>,

    // Clustering
    #[serde(default)]
    pub instances: u32,
    #[serde(default)]
    pub lb_strategy: Option<LoadBalanceStrategy>,
    #[serde(default)]
    pub instance_weight: Option<u32>,
    #[serde(default)]
    pub instance_var: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub cluster_group: Option<String>,
    #[serde(default)]
    pub instance_id: u32,

    // Health checks
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,

    // Environment
    #[serde(default)]
    pub dot_env: Option<PathBuf>,

    // Lifecycle hooks
    #[serde(default)]
    pub pre_start: Option<String>,
    #[serde(default)]
    pub post_start: Option<String>,
    #[serde(default)]
    pub install: Option<String>,
    #[serde(default)]
    pub build: Option<String>,

    // Logging
    #[serde(default)]
    pub stdout: Option<PathBuf>,
    #[serde(default)]
    pub stderr: Option<PathBuf>,
    #[serde(default)]
    pub combine_logs: bool,
    #[serde(default)]
    pub merge_logs: bool,
    #[serde(default)]
    pub log_date_format: Option<String>,

    // Advanced
    #[serde(default = "default_kill_timeout")]
    pub kill_timeout_ms: u64,
    #[serde(default)]
    pub listen_timeout_ms: u64,
    #[serde(default)]
    pub wait_ready: bool,
    #[serde(default)]
    pub min_uptime_ms: u64,
    #[serde(default)]
    pub cron: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub group: Option<String>,

    // Metadata
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
    #[serde(default)]
    pub kubernetes: Option<KubernetesConfig>,
    #[serde(default)]
    pub docker: Option<DockerConfig>,
}

fn default_true() -> bool { true }
fn default_max_restarts() -> u32 { 10 }
fn default_max_restart_delay() -> u64 { 30000 }
fn default_backoff() -> f64 { 2.0 }
fn default_watch_delay() -> u64 { 100 }
fn default_kill_timeout() -> u64 { 5000 }

/// Deployment environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentEnvConfig {
    pub host: String,
    pub user: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    #[serde(default)]
    pub key: Option<String>,
    pub path: String,
    #[serde(default)]
    pub pre_deploy: Option<DeployScripts>,
    #[serde(default)]
    pub post_deploy: Option<DeployScripts>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub r#ref: Option<String>,
}

fn default_ssh_port() -> u16 { 22 }

/// Deployment scripts: string or array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeployScripts {
    Single(String),
    Multiple(Vec<String>),
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub environments: HashMap<String, DeploymentEnvConfig>,
}

/// Top-level TSPM configuration (TOML)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TspmConfig {
    pub processes: Vec<ProcessConfig>,

    #[serde(default)]
    pub defaults: Option<ProcessConfig>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
    #[serde(default = "default_pid_dir")]
    pub pid_dir: PathBuf,
    #[serde(default)]
    pub webhooks: Vec<WebhookConfig>,
    #[serde(default)]
    pub structured_logging: bool,
    #[serde(default)]
    pub api: Option<ApiConfig>,
    #[serde(default)]
    pub deploy: Option<DeploymentConfig>,
}

fn default_log_dir() -> PathBuf { PathBuf::from("logs") }
fn default_pid_dir() -> PathBuf { PathBuf::from(".pids") }

// ============================================================
// Runtime types
// ============================================================

/// Process status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub name: String,
    pub pid: Option<u32>,
    pub killed: bool,
    pub exit_code: Option<i32>,
    pub state: ProcessState,
    pub restart_count: u32,
    pub uptime_ms: Option<u64>,
    pub instance_id: u32,
    pub cluster_group: Option<String>,
    pub healthy: Option<bool>,
    pub namespace: Option<String>,
}

impl ProcessStatus {
    pub fn state_str(&self) -> &str {
        match self.state {
            ProcessState::Starting => "starting",
            ProcessState::Running => "running",
            ProcessState::Stopping => "stopping",
            ProcessState::Stopped => "stopped",
            ProcessState::Errored => "errored",
            ProcessState::Restarting => "restarting",
        }
    }
}

/// Instance information for clustering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: u32,
    pub name: String,
    pub connections: u64,
    pub cpu: f64,
    pub memory: u64,
    pub weight: u32,
    pub healthy: bool,
    pub state: Option<ProcessState>,
    pub pid: Option<u32>,
    pub started_at: Option<u64>,
}

/// Cluster information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub name: String,
    pub total_instances: usize,
    pub running_instances: usize,
    pub healthy_instances: usize,
    pub strategy: LoadBalanceStrategy,
    pub instances: Vec<InstanceInfo>,
}

/// Process group information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessGroup {
    pub name: String,
    pub namespace: String,
    pub process_count: usize,
    pub total_instances: usize,
    pub process_names: Vec<String>,
}

/// Process metrics collected at runtime
#[derive(Debug, Clone, Default)]
pub struct ProcessMetrics {
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub uptime_secs: u64,
    pub pid: Option<u32>,
}

/// Event types used in the event system
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    ProcessStart,
    ProcessStop,
    ProcessRestart,
    ProcessExit,
    ProcessError,
    ProcessStateChange,
    ProcessLog,
    ProcessOom,
    ProcessReady,
    InstanceAdd,
    InstanceRemove,
    InstanceHealthChange,
    MetricsUpdate,
    CpuHigh,
    MemoryHigh,
    SystemStart,
    SystemStop,
    SystemError,
    ConfigReload,
    ConfigChange,
}

/// TSPM event payload variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TspmEvent {
    ProcessStart {
        name: String,
        instance_id: u32,
        pid: Option<u32>,
    },
    ProcessStop {
        name: String,
        instance_id: u32,
        pid: Option<u32>,
        reason: StopReason,
    },
    ProcessRestart {
        name: String,
        instance_id: u32,
        restart_count: u32,
        delay_ms: Option<u64>,
        reason: Option<RestartReason>,
    },
    ProcessExit {
        name: String,
        instance_id: u32,
        exit_code: Option<i32>,
        signal: Option<i32>,
    },
    ProcessError {
        name: String,
        instance_id: u32,
        error: String,
    },
    ProcessStateChange {
        name: String,
        instance_id: u32,
        previous: ProcessState,
        current: ProcessState,
    },
    ProcessLog {
        name: String,
        instance_id: u32,
        message: String,
        log_type: LogType,
    },
    ProcessOom {
        name: String,
        instance_id: u32,
        memory_bytes: u64,
        limit_bytes: u64,
    },
    ProcessReady {
        name: String,
        instance_id: u32,
        pid: Option<u32>,
    },
    MetricsUpdate {
        name: String,
        instance_id: u32,
        cpu: f64,
        memory: u64,
        uptime_secs: u64,
    },
    SystemStart {
        config_file: String,
        process_count: usize,
    },
    SystemStop {
        reason: SystemStopReason,
        graceful: bool,
    },
    SystemError {
        error: String,
    },
    ConfigReload {
        config_file: String,
        changes: Vec<String>,
    },
}

impl TspmEvent {
    pub fn event_type(&self) -> EventType {
        match self {
            TspmEvent::ProcessStart { .. } => EventType::ProcessStart,
            TspmEvent::ProcessStop { .. } => EventType::ProcessStop,
            TspmEvent::ProcessRestart { .. } => EventType::ProcessRestart,
            TspmEvent::ProcessExit { .. } => EventType::ProcessExit,
            TspmEvent::ProcessError { .. } => EventType::ProcessError,
            TspmEvent::ProcessStateChange { .. } => EventType::ProcessStateChange,
            TspmEvent::ProcessLog { .. } => EventType::ProcessLog,
            TspmEvent::ProcessOom { .. } => EventType::ProcessOom,
            TspmEvent::ProcessReady { .. } => EventType::ProcessReady,
            TspmEvent::MetricsUpdate { .. } => EventType::MetricsUpdate,
            TspmEvent::SystemStart { .. } => EventType::SystemStart,
            TspmEvent::SystemStop { .. } => EventType::SystemStop,
            TspmEvent::SystemError { .. } => EventType::SystemError,
            TspmEvent::ConfigReload { .. } => EventType::ConfigReload,
        }
    }
}

/// Dump file format for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpData {
    pub processes: Vec<ProcessConfig>,
    pub timestamp: String,
    pub version: String,
}
