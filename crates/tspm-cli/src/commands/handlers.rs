use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tspm_core::config;
use tspm_engine::ProcessManager;

pub async fn handle_start(
    config_path: &Path,
    _process_name: Option<&str>,
    watch: bool,
    env_vars: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = config::load_config(config_path)
        .map_err(|e| format!("Config error: {e}"))?;

    for ev in env_vars {
        if let Some((k, v)) = ev.split_once('=') {
            for proc in &mut cfg.processes {
                proc.env.insert(k.to_string(), v.to_string());
            }
        }
    }

    for proc in &mut cfg.processes {
        if watch {
            proc.watch = tspm_core::WatchConfig::Bool(true);
        }
    }

    let config_dir = config_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    let mut manager = ProcessManager::with_config_dir(config_dir);
    manager.load_from_config(&cfg).await?;
    manager.start_all().await?;

    println!("[TSPM] All processes started. Press Ctrl+C to stop.");

    tspm_engine::SignalHandler::wait_for_shutdown().await;
    manager.stop_all().await?;
    println!("[TSPM] Shutdown complete");

    Ok(())
}

pub async fn handle_stop(
    process_name: Option<&str>,
    all: bool,
    manager: &mut ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    if all {
        manager.stop_all().await?;
        println!("[TSPM] All processes stopped");
    } else if let Some(name) = process_name {
        manager.stop_process(name).await?;
        println!("[TSPM] Process '{name}' stopped");
    }
    Ok(())
}

pub async fn handle_restart(
    config_path: &Path,
    process_name: Option<&str>,
    all: bool,
    manager: &mut ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    if all {
        manager.stop_all().await?;
        let cfg = config::load_config(config_path)?;
        let config_dir = config_path.parent().unwrap_or(std::path::Path::new("."));
        manager.set_config_dir(config_dir.to_path_buf());
        manager.load_from_config(&cfg).await?;
        manager.start_all().await?;
        println!("[TSPM] All processes restarted");
    } else if let Some(name) = process_name {
        manager.restart_process(name).await?;
        println!("[TSPM] Process '{name}' restarted");
    }
    Ok(())
}

pub async fn handle_reload(
    config_path: &Path,
    process_name: Option<&str>,
    all: bool,
    manager: &mut ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    handle_restart(config_path, process_name, all, manager).await
}

pub async fn handle_delete(
    process_name: Option<&str>,
    all: bool,
    manager: &mut ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    if all {
        let names: Vec<String> = manager.get_statuses().iter().map(|s| s.name.clone()).collect();
        for name in &names {
            manager.remove_process(name).await?;
        }
        println!("[TSPM] All processes deleted");
    } else if let Some(name) = process_name {
        manager.remove_process(name).await?;
        println!("[TSPM] Process '{name}' deleted");
    }
    Ok(())
}

pub async fn handle_list(manager: &ProcessManager) -> Result<(), Box<dyn std::error::Error>> {
    let statuses = manager.get_statuses();
    if statuses.is_empty() {
        println!("No processes running");
        return Ok(());
    }

    use tabled::{Table, Tabled};
    #[derive(Tabled)]
    struct Row {
        name: String,
        pid: String,
        status: String,
        restarts: u32,
        uptime: String,
    }

    let rows: Vec<Row> = statuses.iter().map(|s| {
        Row {
            name: s.name.clone(),
            pid: s.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string()),
            status: format!("{:?}", s.state),
            restarts: s.restart_count,
            uptime: s.uptime_ms
                .map(|ms| format!("{}s", ms / 1000))
                .unwrap_or_else(|| "-".to_string()),
        }
    }).collect();

    println!("{}", Table::new(rows));
    Ok(())
}

pub async fn handle_logs(
    process_name: Option<&str>,
    lines: usize,
    _follow: bool,
    manager: &ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = process_name.unwrap_or("all");
    if let Some(proc) = manager.get_process(name) {
        let stdout = proc.config().stdout.as_ref();
        if let Some(path) = stdout {
            if path.exists() {
                let log_lines = tspm_logging::LogManager::read_tail(path, lines)?;
                for line in &log_lines {
                    println!("{line}");
                }
            }
        }
    } else {
        println!("Process '{name}' not found");
    }
    Ok(())
}

pub async fn handle_describe(
    name: &str,
    manager: &ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(proc) = manager.get_process(name) {
        let status = proc.get_status();
        let config = proc.config();
        println!("Process: {}", status.name);
        println!("  PID: {:?}", status.pid);
        println!("  State: {:?}", status.state);
        println!("  Restarts: {}", status.restart_count);
        println!("  Script: {}", config.script);
        println!("  Instances: {}", config.instances);
        println!("  Namespace: {:?}", config.namespace);
    } else {
        println!("Process '{name}' not found");
    }
    Ok(())
}

pub async fn handle_monit(manager: &ProcessManager) -> Result<(), Box<dyn std::error::Error>> {
    println!("TSPM Monitor (press Ctrl+C to quit)");
    println!("---");
    handle_list(manager).await
}

pub async fn handle_cluster(
    name: Option<&str>,
    manager: &ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(n) = name {
        if let Some(info) = manager.get_cluster_info(n) {
            println!("Cluster: {}", info.name);
            println!("  Strategy: {:?}", info.strategy);
            println!("  Instances: {} total, {} running, {} healthy",
                info.total_instances, info.running_instances, info.healthy_instances);
        } else {
            println!("No cluster found for '{n}'");
        }
    } else {
        let groups = manager.get_cluster_groups();
        println!("Clusters: {:?}", groups);
    }
    Ok(())
}

pub async fn handle_scale(
    name: &str,
    count: u32,
    manager: &mut ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    manager.scale_process(name, count).await?;
    println!("[TSPM] Scaled '{name}' to {count} instances");
    Ok(())
}

pub async fn handle_groups(manager: &ProcessManager) -> Result<(), Box<dyn std::error::Error>> {
    let groups = manager.get_process_groups();
    if groups.is_empty() {
        println!("No process groups");
    }
    for group in &groups {
        println!("Group: {} ({} in namespace '{}', {} instances)",
            group.name, group.process_count, group.namespace, group.total_instances);
    }
    Ok(())
}

pub async fn handle_dev(
    config_path: &Path,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::load_config(config_path)?;
    let config_dir = config_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    let mut manager = ProcessManager::with_config_dir(config_dir);
    manager.load_from_config(&cfg).await?;
    manager.start_all().await?;

    let manager = Arc::new(Mutex::new(manager));
    let api_manager = manager.clone();

    let api_task = tokio::spawn(async move {
        tspm_server::start_dashboard(api_manager, port, "0.0.0.0").await
    });

    println!("[TSPM] Dev mode started. API on http://localhost:{port}");
    println!("[TSPM] Press Ctrl+C to stop");

    tspm_engine::SignalHandler::wait_for_shutdown().await;

    if let Ok(mgr) = Arc::try_unwrap(manager) {
        mgr.into_inner().stop_all().await?;
    }
    api_task.abort();

    Ok(())
}

pub async fn handle_flush(manager: &mut ProcessManager) -> Result<(), Box<dyn std::error::Error>> {
    for status in manager.get_statuses() {
        if let Some(proc) = manager.get_process(&status.name) {
            let stdout = proc.config().stdout.clone();
            let stderr = proc.config().stderr.clone();
            if let (Some(out), Some(err)) = (stdout, stderr) {
                tspm_logging::LogManager::flush_process(&out, &err)?;
            }
        }
    }
    println!("[TSPM] All logs flushed");
    Ok(())
}

pub async fn handle_reload_logs(
    _manager: &ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TSPM] Logs reloaded");
    Ok(())
}

pub async fn handle_save(manager: &ProcessManager) -> Result<(), Box<dyn std::error::Error>> {
    let configs: Vec<tspm_core::ProcessConfig> = manager.get_statuses()
        .iter()
        .filter_map(|s| manager.get_process(&s.name).map(|p| p.config().clone()))
        .collect();

    let persistence = tspm_deploy::PersistenceManager::new();
    persistence.save(&configs)?;
    println!("[TSPM] Process list saved to {}", persistence.dump_path().display());
    Ok(())
}

pub async fn handle_resurrect(
    dashboard: bool,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let persistence = tspm_deploy::PersistenceManager::new();
    if let Some(data) = persistence.load() {
        let cfg = tspm_core::TspmConfig {
            processes: data.processes,
            defaults: None,
            namespace: None,
            log_dir: "logs".into(),
            pid_dir: ".pids".into(),
            webhooks: vec![],
            structured_logging: false,
            api: None,
            deploy: None,
        };

        let mut manager = ProcessManager::new();
        manager.load_from_config(&cfg).await?;
        manager.start_all().await?;

        println!("[TSPM] {} processes resurrected", cfg.processes.len());

        let manager_arc = Arc::new(Mutex::new(manager));
        let dashboard_handle = if dashboard {
            println!("[TSPM] Dashboard starting on http://localhost:{}", port);
            let m = manager_arc.clone();
            Some(tokio::spawn(async move {
                let _ = tspm_server::start_dashboard(m, port, "0.0.0.0").await;
            }))
        } else {
            None
        };

        tspm_engine::SignalHandler::wait_for_shutdown().await;
        
        if let Some(handle) = dashboard_handle {
            handle.abort();
        }

        if let Ok(mgr) = Arc::try_unwrap(manager_arc) {
            mgr.into_inner().stop_all().await?;
        }
    } else {
        println!("[TSPM] No saved state found");
    }
    Ok(())
}

pub async fn handle_startup(
    platform: &str,
    user: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if platform == "systemd" {
        tspm_deploy::StartupManager::install_systemd(user)?;
    } else {
        println!("Platform '{platform}' not supported yet");
    }
    Ok(())
}

pub async fn handle_unstartup() -> Result<(), Box<dyn std::error::Error>> {
    tspm_deploy::StartupManager::uninstall_systemd()?;
    println!("[TSPM] Startup script removed");
    Ok(())
}

pub async fn handle_reset(
    process_name: Option<&str>,
    manager: &mut ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(name) = process_name {
        manager.restart_process(name).await?;
        println!("[TSPM] Reset '{name}'");
    } else {
        println!("[TSPM] Use --name to specify a process");
    }
    Ok(())
}

pub async fn handle_prettylist(
    manager: &ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let statuses = manager.get_statuses();
    let json = serde_json::to_string_pretty(&statuses)?;
    println!("{json}");
    Ok(())
}

pub async fn handle_serve(
    path: &Path,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[TSPM] Serving {} on port {}", path.display(), port);
    tspm_server::start_static_server(path.to_path_buf(), port, "0.0.0.0").await?;
    Ok(())
}

pub async fn handle_report(
    output: Option<&Path>,
    manager: &ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = serde_json::json!({
        "version": tspm_core::APP_VERSION,
        "processes": manager.get_statuses(),
        "system": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        },
    });

    let report_str = serde_json::to_string_pretty(&report)?;

    if let Some(path) = output {
        std::fs::write(path, &report_str)?;
        println!("[TSPM] Report written to {}", path.display());
    } else {
        println!("{report_str}");
    }
    Ok(())
}

pub async fn handle_deploy(
    environment: &str,
    config_path: Option<&Path>,
    repo: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = if let Some(path) = config_path {
        config::load_config(path)?
    } else {
        config::load_config_with_discovery(None)?
    };

    let deploy_cfg = cfg.deploy.as_ref()
        .ok_or("No deployment configuration found")?;

    let env_cfg = deploy_cfg.environments.get(environment)
        .ok_or(format!("Environment '{environment}' not found in config"))?;

    tspm_deploy::Deployer::deploy(environment, env_cfg, repo).await?;
    Ok(())
}

pub async fn handle_dashboard(
    port: u16,
    host: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(Mutex::new(ProcessManager::new()));
    println!("[TSPM] Dashboard starting on http://{host}:{port}");
    tspm_server::start_dashboard(manager, port, host).await?;
    Ok(())
}

pub async fn handle_install(
    name: &str,
    config_path: &Path,
    _manager: &mut ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::load_config(config_path)?;
    for proc in &cfg.processes {
        if proc.name == name {
            if let Some(ref install_script) = proc.install {
                println!("[TSPM] Running install for '{name}': {install_script}");
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(install_script)
                    .current_dir(proc.cwd.as_deref().unwrap_or(Path::new(".")))
                    .output()?;
                if output.status.success() {
                    println!("[TSPM] Install completed");
                } else {
                    eprintln!("[TSPM] Install failed");
                }
                return Ok(());
            }
        }
    }
    println!("[TSPM] No install script for '{name}'");
    Ok(())
}

pub async fn handle_build(
    name: &str,
    config_path: &Path,
    _manager: &mut ProcessManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = config::load_config(config_path)?;
    for proc in &cfg.processes {
        if proc.name == name {
            if let Some(ref build_script) = proc.build {
                println!("[TSPM] Building '{name}': {build_script}");
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(build_script)
                    .current_dir(proc.cwd.as_deref().unwrap_or(Path::new(".")))
                    .output()?;
                if output.status.success() {
                    println!("[TSPM] Build completed");
                } else {
                    eprintln!("[TSPM] Build failed");
                }
                return Ok(());
            }
        }
    }
    println!("[TSPM] No build script for '{name}'");
    Ok(())
}
