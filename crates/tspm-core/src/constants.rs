use std::path::PathBuf;

// ============================================================
// Process states
// ============================================================
pub const PROCESS_STATE_STARTING: &str = "starting";
pub const PROCESS_STATE_RUNNING: &str = "running";
pub const PROCESS_STATE_STOPPING: &str = "stopping";
pub const PROCESS_STATE_STOPPED: &str = "stopped";
pub const PROCESS_STATE_ERRORED: &str = "errored";
pub const PROCESS_STATE_RESTARTING: &str = "restarting";

// ============================================================
// Default config files to discover
// ============================================================
pub const DEFAULT_CONFIG_FILES: &[&str] = &[
    "tspm.toml",
];

// ============================================================
// Default process configuration
// ============================================================
pub const DEFAULT_AUTORESTART: bool = true;
pub const DEFAULT_MAX_RESTARTS: u32 = 10;
pub const DEFAULT_MIN_RESTART_DELAY_MS: u64 = 100;
pub const DEFAULT_MAX_RESTART_DELAY_MS: u64 = 30000;
pub const DEFAULT_RESTART_BACKOFF: f64 = 2.0;
pub const DEFAULT_KILL_TIMEOUT_MS: u64 = 5000;
pub const DEFAULT_LISTEN_TIMEOUT_MS: u64 = 0;
pub const DEFAULT_WAIT_READY: bool = false;
pub const DEFAULT_LOG_DIR: &str = "logs";
pub const DEFAULT_PID_DIR: &str = ".pids";
pub const DEFAULT_MAX_MEMORY_BYTES: u64 = 0;
pub const DEFAULT_MIN_UPTIME_MS: u64 = 0;
pub const DEFAULT_WATCH_DELAY_MS: u64 = 100;
pub const DEFAULT_MERGE_LOGS: bool = false;
pub const DEFAULT_INSTANCE_VAR: &str = "NODE_APP_INSTANCE";

// ============================================================
// Restart config
// ============================================================
pub const RESTART_MIN_DELAY_MS: u64 = 100;
pub const RESTART_MAX_DELAY_MS: u64 = 30000;
pub const RESTART_BASE_DELAY_MS: u64 = 1000;
pub const RESTART_BACKOFF_MULTIPLIER: f64 = 2.0;

// ============================================================
// Watch config
// ============================================================
pub const WATCH_DEBOUNCE_MS: u64 = 100;
pub const WATCH_DEFAULT_IGNORE: &[&str] = &[
    "node_modules/**",
    ".git/**",
    "logs/**",
    "*.log",
    ".pids/**",
];

// ============================================================
// Log config
// ============================================================
pub const LOG_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
pub const LOG_MAX_FILES: usize = 5;

// ============================================================
// Exit codes
// ============================================================
pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_ERROR: i32 = 1;
pub const EXIT_CONFIG_NOT_FOUND: i32 = 2;
pub const EXIT_CONFIG_INVALID: i32 = 3;
pub const EXIT_PROCESS_NOT_FOUND: i32 = 4;
pub const EXIT_PROCESS_START_FAILED: i32 = 5;
pub const EXIT_PERMISSION_DENIED: i32 = 6;

// ============================================================
// Signals
// ============================================================
pub const SIG_GRACEFUL_SHUTDOWN: i32 = libc::SIGTERM;
pub const SIG_FORCEFUL_SHUTDOWN: i32 = libc::SIGKILL;
pub const SIG_RELOAD: i32 = libc::SIGHUP;
pub const SIG_INTERRUPT: i32 = libc::SIGINT;

// ============================================================
// Environment vars
// ============================================================
pub const ENV_TSPM_CONFIG_PATH: &str = "TSPM_CONFIG_PATH";
pub const ENV_TSPM_LOG_LEVEL: &str = "TSPM_LOG_LEVEL";
pub const ENV_TSPM_HOME: &str = "TSPM_HOME";
pub const ENV_TSPM_PROCESS_NAME: &str = "TSPM_PROCESS_NAME";
pub const ENV_TSPM_INSTANCE_ID: &str = "TSPM_INSTANCE_ID";

// ============================================================
// Cluster config
// ============================================================
pub const CLUSTER_DEFAULT_INSTANCES: u32 = 1;
pub const CLUSTER_MAX_INSTANCES: u32 = 32;

// ============================================================
// Memory monitoring
// ============================================================
pub const MEMORY_CHECK_INTERVAL_MS: u64 = 5000;
pub const LOG_ROTATE_THRESHOLD: u64 = 64 * 1024; // 64KB incremental

// ============================================================
// Timeouts
// ============================================================
pub const TIMEOUT_GRACEFUL_STOP_MS: u64 = 500;
pub const TIMEOUT_STARTUP_WAIT_MS: u64 = 1000;

// ============================================================
// App constants
// ============================================================
pub const APP_NAME: &str = "TSPM";
pub const APP_LOG_PREFIX: &str = "[TSPM]";
pub const APP_DEFAULT_NAMESPACE: &str = "default";
pub const APP_VERSION: &str = "0.1.0";

// ============================================================
// Helper functions
// ============================================================

/// Get the default log file path for a process
pub fn get_default_log_path(process_name: &str, log_dir: &str) -> PathBuf {
    let safe_name = process_name
        .trim_end_matches(|c: char| c.is_ascii_digit() || c == '-')
        .trim_end_matches('-')
        .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
    PathBuf::from(log_dir).join(format!("{safe_name}.log"))
}

/// Get the default error log file path for a process
pub fn get_default_err_log_path(process_name: &str, log_dir: &str) -> PathBuf {
    let safe_name = process_name
        .trim_end_matches(|c: char| c.is_ascii_digit() || c == '-')
        .trim_end_matches('-')
        .replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_");
    PathBuf::from(log_dir).join(format!("{safe_name}-err.log"))
}

/// Get the default PID file path for a process
pub fn get_default_pid_path(process_name: &str, pid_dir: &str) -> PathBuf {
    PathBuf::from(pid_dir).join(format!("{process_name}.pid"))
}

/// Calculate restart delay with exponential backoff
pub fn calculate_restart_delay(restart_count: u32) -> u64 {
    let delay = RESTART_BASE_DELAY_MS as f64 * RESTART_BACKOFF_MULTIPLIER.powi(restart_count as i32);
    (delay as u64).min(RESTART_MAX_DELAY_MS)
}

/// Get the TSPM home directory
pub fn get_tspm_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".tspm")
}
