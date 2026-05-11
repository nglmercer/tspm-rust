use crate::error::{TspmError, TspmResult};
use crate::types::{ProcessConfig, TspmConfig};
use std::fs;
use std::path::{Path, PathBuf};

/// Load TSPM configuration from a TOML file
pub fn load_config(path: &Path) -> TspmResult<TspmConfig> {
    if !path.exists() {
        return Err(TspmError::ConfigNotFound {
            path: path.display().to_string(),
        });
    }

    let content = fs::read_to_string(path).map_err(|e| TspmError::ConfigRead {
        path: path.display().to_string(),
        source: e,
    })?;

    let config: TspmConfig = toml::from_str(&content).map_err(|e| TspmError::ConfigParse {
        path: path.display().to_string(),
        source: e,
    })?;

    let config = normalize_config(config);
    validate_config(&config)?;

    Ok(config)
}

/// Discover a config file in the current directory
pub fn discover_config_file() -> Option<PathBuf> {
    let candidates = ["tspm.toml"];

    for name in &candidates {
        let path = PathBuf::from(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Load config with auto-discovery
pub fn load_config_with_discovery(path: Option<&Path>) -> TspmResult<TspmConfig> {
    match path {
        Some(p) => load_config(p),
        None => {
            let discovered = discover_config_file().ok_or(TspmError::ConfigNotFound {
                path: "tspm.toml".to_string(),
            })?;
            load_config(&discovered)
        }
    }
}

/// Validate a TSPM configuration
pub fn validate_config(config: &TspmConfig) -> TspmResult<()> {
    if config.processes.is_empty() {
        return Err(TspmError::ConfigValidation {
            message: "No processes defined in configuration".to_string(),
        });
    }

    // Check for duplicate process names
    let mut names = std::collections::HashSet::new();
    for (i, proc) in config.processes.iter().enumerate() {
        if proc.name.is_empty() {
            return Err(TspmError::ConfigValidation {
                message: format!("processes[{i}]: name is required"),
            });
        }
        if proc.script.is_empty() {
            return Err(TspmError::ConfigValidation {
                message: format!("processes[{i}] '{}': script is required", proc.name),
            });
        }
        if !names.insert(&proc.name) {
            return Err(TspmError::ConfigValidation {
                message: format!("Duplicate process name: '{}'", proc.name),
            });
        }

        // Validate name format
        if !proc.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(TspmError::ConfigValidation {
                message: format!(
                    "processes[{i}] '{}': name can only contain letters, numbers, underscores, and hyphens",
                    proc.name
                ),
            });
        }
    }

    Ok(())
}

/// Normalize a config: apply defaults to all processes
pub fn normalize_config(config: TspmConfig) -> TspmConfig {
    let defaults = config.defaults.clone();
    let log_dir = config
        .log_dir
        .to_str()
        .unwrap_or("logs")
        .to_string();
    TspmConfig {
        processes: config
            .processes
            .into_iter()
            .map(|proc| apply_defaults(proc, &defaults, &log_dir))
            .collect(),
        ..config
    }
}

/// Apply defaults to a single process config
fn apply_defaults(proc: ProcessConfig, defaults: &Option<ProcessConfig>, log_dir: &str) -> ProcessConfig {
    let mut merged = proc.clone();

    if let Some(ref d) = defaults {
        if merged.autorestart == true && !d.autorestart {
            merged.autorestart = d.autorestart;
        }
        if merged.max_restarts == 0 { merged.max_restarts = d.max_restarts; }
        if merged.min_restart_delay_ms == 0 { merged.min_restart_delay_ms = d.min_restart_delay_ms; }
        if merged.max_restart_delay_ms == 0 { merged.max_restart_delay_ms = d.max_restart_delay_ms; }
        if merged.kill_timeout_ms == 0 { merged.kill_timeout_ms = d.kill_timeout_ms; }
        if merged.watch_delay_ms == 0 { merged.watch_delay_ms = d.watch_delay_ms; }
        if merged.namespace.is_none() { merged.namespace = d.namespace.clone(); }
        if merged.instances == 0 { merged.instances = d.instances; }
        if merged.env.is_empty() {
            merged.env = d.env.clone();
        } else {
            for (k, v) in &d.env {
                merged.env.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }

    // Apply defaults for unset fields
    if merged.autorestart { merged.autorestart = crate::constants::DEFAULT_AUTORESTART; }
    if merged.max_restarts == 0 { merged.max_restarts = crate::constants::DEFAULT_MAX_RESTARTS; }
    if merged.max_restart_delay_ms == 0 { merged.max_restart_delay_ms = crate::constants::DEFAULT_MAX_RESTART_DELAY_MS; }
    if merged.kill_timeout_ms == 0 { merged.kill_timeout_ms = crate::constants::DEFAULT_KILL_TIMEOUT_MS; }
    if merged.watch_delay_ms == 0 { merged.watch_delay_ms = crate::constants::DEFAULT_WATCH_DELAY_MS; }
    if merged.namespace.is_none() { merged.namespace = Some(crate::constants::APP_DEFAULT_NAMESPACE.to_string()); }
    if merged.instances == 0 { merged.instances = 1; }
    if merged.instance_var.is_none() { merged.instance_var = Some(crate::constants::DEFAULT_INSTANCE_VAR.to_string()); }

    // Auto-generate log paths if not set
    if merged.stdout.is_none() {
        merged.stdout = Some(crate::constants::get_default_log_path(&merged.name, &log_dir));
    }
    if merged.stderr.is_none() {
        merged.stderr = Some(crate::constants::get_default_err_log_path(&merged.name, &log_dir));
    }

    merged
}

/// Parse a TOML string into a TspmConfig
pub fn parse_config_toml(content: &str) -> TspmResult<TspmConfig> {
    let config: TspmConfig = toml::from_str(content).map_err(|e| TspmError::ConfigParse {
        path: "<inline>".to_string(),
        source: e,
    })?;
    validate_config(&config)?;
    Ok(normalize_config(config))
}
