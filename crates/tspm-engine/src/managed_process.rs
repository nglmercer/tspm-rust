use std::collections::HashMap;
use tokio::process::{Child, Command};
use tokio::time::{sleep, Duration};
use tokio::sync::Mutex as AsyncMutex;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use tspm_core::{
    ProcessConfig, ProcessState, ProcessStatus, RestartReason, get_default_log_path,
};

pub struct LogEntry {
    pub timestamp: String,
    pub log_type: String,
    pub message: String,
    pub process_name: String,
}

/// Thread-safe ring buffer for process logs
pub struct LogBuffer {
    entries: Vec<LogEntry>,
    max_size: usize,
}

impl LogBuffer {
    pub fn new(max_size: usize) -> Self {
        Self { entries: Vec::with_capacity(max_size), max_size }
    }

    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_size {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    pub fn get(&self, limit: Option<usize>) -> Vec<&LogEntry> {
        let limit = limit.unwrap_or(self.max_size);
        let start = self.entries.len().saturating_sub(limit);
        self.entries[start..].iter().collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// A single managed process instance
pub struct ManagedProcess {
    config: ProcessConfig,
    instance_id: u32,
    child: Option<Child>,
    restart_count: u32,
    manually_stopped: bool,
    current_state: ProcessState,
    started_at: Option<std::time::Instant>,
    pid: Option<u32>,
    exit_code: Option<i32>,
    pub log_buffer: Arc<AsyncMutex<LogBuffer>>,
    /// Base directory for resolving relative paths — typically the config file's directory
    config_dir: std::path::PathBuf,
}

impl ManagedProcess {
    pub fn new(config: ProcessConfig, instance_id: u32, config_dir: std::path::PathBuf) -> Self {
        Self {
            config,
            instance_id,
            child: None,
            restart_count: 0,
            manually_stopped: false,
            current_state: ProcessState::Stopped,
            started_at: None,
            pid: None,
            exit_code: None,
            log_buffer: Arc::new(AsyncMutex::new(LogBuffer::new(1000))),
            config_dir,
        }
    }

    /// Get the process display name (with instance suffix if > 0)
    pub fn display_name(&self) -> String {
        if self.instance_id > 0 {
            format!("{}-{}", self.config.name, self.instance_id)
        } else {
            self.config.name.clone()
        }
    }

    /// Start the process
    pub async fn start(&mut self) -> Result<(), String> {
        self.manually_stopped = false;
        let name = self.display_name();

        self.set_state(ProcessState::Starting);

        // Run install and build scripts if defined
        if let Some(ref script) = self.config.install {
            self.run_script("install", script).await;
        }
        if let Some(ref script) = self.config.build {
            self.run_script("build", script).await;
        }

        // Run pre-start script
        if let Some(ref script) = self.config.pre_start {
            self.run_script("pre-start", script).await;
        }

        info!("{} Starting process: {}", "[TSPM]", name);

        // Determine command
        let (program, args) = self.resolve_command();

        // Build environment
        let env_vars = self.build_environment();

        // Prepare log paths
        let stdout_path = self.config.stdout.clone().unwrap_or_else(|| {
            get_default_log_path(&self.config.name, "logs")
        });
        let stderr_path = self.config.stderr.clone().unwrap_or_else(|| {
            let mut p = stdout_path.clone();
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            p.set_file_name(format!("{}-err.log", stem));
            p
        });

        // Ensure log directories exist
        if let Some(parent) = stdout_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        // Open log files
        let stdout_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stdout_path)
            .await
            .map_err(|e| format!("Failed to open stdout log: {e}"))?;

        let stderr_file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stderr_path)
            .await
            .map_err(|e| format!("Failed to open stderr log: {e}"))?;

        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .envs(&env_vars)
            .stdout(std::process::Stdio::from(stdout_file.into_std().await))
            .stderr(std::process::Stdio::from(stderr_file.into_std().await))
            .stdin(std::process::Stdio::piped());

        // Resolve working directory:
        // 1. If cwd is explicitly set and absolute → use as-is
        // 2. If cwd is relative → resolve relative to config_dir
        // 3. If cwd is not set → default to script's parent directory
        let cwd = if let Some(ref configured_cwd) = self.config.cwd {
            if configured_cwd.is_absolute() {
                configured_cwd.clone()
            } else {
                self.config_dir.join(configured_cwd)
            }
        } else {
            // Default: parent directory of the script
            let script_path = std::path::Path::new(&self.config.script);
            if script_path.is_absolute() {
                script_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| self.config_dir.clone())
            } else {
                let abs_script = self.config_dir.join(script_path);
                abs_script.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| self.config_dir.clone())
            }
        };

        // Ensure the cwd exists
        if !cwd.exists() {
            let _ = tokio::fs::create_dir_all(&cwd).await;
        }

        cmd.current_dir(&cwd);

        if self.config.combine_logs {
            cmd.stderr(std::process::Stdio::piped());
        }

        // Put child in its own process group so we can kill the whole tree
        #[cfg(unix)]
        {
            #[allow(unused_imports)]
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn process '{name}': {e}"))?;

        self.pid = child.id();
        self.child = Some(child);
        self.started_at = Some(std::time::Instant::now());
        self.set_state(ProcessState::Running);

        info!("{} Process '{}' started (PID: {:?})", "[TSPM]", name, self.pid);

        // Run post-start script in background
        if let Some(ref script) = self.config.post_start {
            let script = script.clone();
            tokio::spawn(async move {
                Self::run_script_static("post-start", &script).await;
            });
        }

        Ok(())
    }

    /// Stop the process
    pub async fn stop(&mut self) -> Result<(), String> {
        self.manually_stopped = true;
        let name = self.display_name();

        self.set_state(ProcessState::Stopping);

        if let Some(ref mut child) = self.child {
            let pid = child.id().ok_or("No PID for child process")?;

            // Kill the entire process group (negative PID = process group)
            #[cfg(unix)]
            {
                unsafe { libc::kill(-(pid as i32), libc::SIGTERM); }
                info!("{} Sent SIGTERM to process group {}", "[TSPM]", pid);
            }

            let kill_timeout = Duration::from_millis(self.config.kill_timeout_ms);

            match tokio::time::timeout(kill_timeout, child.wait()).await {
                Ok(Ok(status)) => {
                    self.exit_code = status.code();
                    info!("{} Process '{}' stopped gracefully", "[TSPM]", name);
                }
                _ => {
                    // Force kill the entire process group
                    #[cfg(unix)]
                    {
                        unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
                    }
                    let _ = child.kill().await;
                    warn!("{} Process '{}' force killed after timeout", "[TSPM]", name);
                }
            }
        }

        self.child = None;
        self.pid = None;
        self.set_state(ProcessState::Stopped);
        info!("{} Stopped process: {}", "[TSPM]", name);

        Ok(())
    }

    /// Restart the process
    pub async fn restart(&mut self, reason: RestartReason) -> Result<(), String> {
        self.set_state(ProcessState::Restarting);
        self.restart_count += 1;

        info!("{} Restarting process '{}' (reason: {:?}, count: {})",
            "[TSPM]", self.display_name(), reason, self.restart_count);

        // Stop the process
        if self.child.is_some() {
            self.manually_stopped = true;
            if let Some(ref mut child) = self.child {
                let pid = child.id();
                #[cfg(unix)]
                if let Some(pid) = pid {
                    unsafe { libc::kill(-(pid as i32), libc::SIGTERM); }
                    sleep(Duration::from_millis(100)).await;
                    unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
                }
                let _ = child.kill().await;
            }
            sleep(Duration::from_millis(500)).await;
            self.child = None;
        }

        self.manually_stopped = false;
        self.start().await
    }

    /// Update process configuration
    pub fn update_config(&mut self, config: ProcessConfig) {
        self.config = config;
    }

    /// Get current status
    pub fn get_status(&self) -> ProcessStatus {
        let uptime_ms = self.started_at.map(|t| t.elapsed().as_millis() as u64);

        ProcessStatus {
            name: self.display_name(),
            pid: self.pid,
            killed: self.child.is_none() && self.current_state != ProcessState::Running,
            exit_code: self.exit_code,
            state: self.current_state,
            restart_count: self.restart_count,
            uptime_ms,
            instance_id: self.instance_id,
            cluster_group: self.config.cluster_group.clone(),
            healthy: Some(self.current_state == ProcessState::Running),
            namespace: self.config.namespace.clone(),
        }
    }

    /// Get the state
    pub fn state(&self) -> ProcessState {
        self.current_state
    }

    /// Get instance ID
    pub fn instance_id(&self) -> u32 {
        self.instance_id
    }

    /// Get the config
    pub fn config(&self) -> &ProcessConfig {
        &self.config
    }

    /// Get PID
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    // ============================================================
    // Private helpers
    // ============================================================

    fn set_state(&mut self, new_state: ProcessState) {
        let old = self.current_state;
        self.current_state = new_state;
        debug!("{} State change: {:?} -> {:?}", self.display_name(), old, new_state);
    }

    fn resolve_command(&self) -> (String, Vec<String>) {
        let script = &self.config.script;
        let args = &self.config.args;

        match &self.config.interpreter {
            Some(interp) if interp == "none" => {
                let mut cmd_args = vec![script.clone()];
                cmd_args.extend(args.clone());
                (script.clone(), cmd_args)
            }
            Some(interp) if interp == "sh" || interp == "bash" => {
                (interp.clone(), vec!["-c".to_string(), script.clone()])
            }
            Some(interp) => {
                let mut cmd_args = vec![script.clone()];
                cmd_args.extend(args.clone());
                (interp.clone(), cmd_args)
            }
            None => {
                if script.contains(' ') {
                    ("sh".to_string(), vec!["-c".to_string(), script.clone()])
                } else {
                    let mut cmd_args = vec![script.clone()];
                    cmd_args.extend(args.clone());
                    (script.clone(), cmd_args)
                }
            }
        }
    }

    fn build_environment(&self) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = HashMap::new();

        // Inherit process environment
        for (k, v) in std::env::vars() {
            env.insert(k, v);
        }

        // Add custom env vars
        for (k, v) in &self.config.env {
            env.insert(k.clone(), v.clone());
        }

        // Augment PATH if not explicitly overridden by custom env
        if !self.config.env.contains_key("PATH") {
            env.insert("PATH".to_string(), tspm_core::get_augmented_path());
        }

        // Add instance variable
        let instance_var = self.config.instance_var.as_deref().unwrap_or("NODE_APP_INSTANCE");
        env.insert(instance_var.to_string(), self.instance_id.to_string());
        env.insert("TSPM_PROCESS_NAME".to_string(), self.display_name());

        env
    }

    async fn run_script(&self, label: &str, script: &str) {
        info!("{} Running {} script for '{}': {}",
            "[TSPM]", label, self.display_name(), script);

        let result = Command::new("sh")
            .arg("-c")
            .arg(script)
            .env("PATH", tspm_core::get_augmented_path())
            .current_dir(self.config.cwd.as_deref().unwrap_or(std::path::Path::new(".")))
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                info!("{} {} script completed successfully", "[TSPM]", label);
            }
            Ok(output) => {
                warn!("{} {} script failed with code {:?}", "[TSPM]", label, output.status.code());
            }
            Err(e) => {
                error!("{} Error running {} script: {}", "[TSPM]", label, e);
            }
        }
    }

    async fn run_script_static(label: &str, script: &str) {
        let result = Command::new("sh")
            .arg("-c")
            .arg(script)
            .env("PATH", tspm_core::get_augmented_path())
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                info!("{} {} script completed", "[TSPM]", label);
            }
            Ok(output) => {
                warn!("{} {} script failed with code {:?}", "[TSPM]", label, output.status.code());
            }
            Err(e) => {
                error!("{} {} script error: {}", "[TSPM]", label, e);
            }
        }
    }
}
