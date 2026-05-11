use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;
use tspm_engine::*;

mod cli;
mod commands;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = cli::Cli::parse();
    let manager = Arc::new(Mutex::new(ProcessManager::new()));

    match cli.command {
        cli::Commands::Start { config, name, watch, daemon, env } => {
            if daemon {
                tracing::warn!("Daemon mode not fully implemented yet");
            }
            commands::handle_start(&config, name.as_deref(), watch, &env).await?;
        }
        cli::Commands::Stop { name, all } => {
            let mut mgr = manager.lock().await;
            commands::handle_stop(name.as_deref(), all, &mut mgr).await?;
        }
        cli::Commands::Restart { config, name, all } => {
            let mut mgr = manager.lock().await;
            commands::handle_restart(&config, name.as_deref(), all, &mut mgr).await?;
        }
        cli::Commands::Reload { config, name, all } => {
            let mut mgr = manager.lock().await;
            commands::handle_reload(&config, name.as_deref(), all, &mut mgr).await?;
        }
        cli::Commands::Delete { name, all } => {
            let mut mgr = manager.lock().await;
            commands::handle_delete(name.as_deref(), all, &mut mgr).await?;
        }
        cli::Commands::List => {
            let mgr = manager.lock().await;
            commands::handle_list(&mgr).await?;
        }
        cli::Commands::Logs { name, lines, follow, timestamp: _ } => {
            let mgr = manager.lock().await;
            commands::handle_logs(name.as_deref(), lines, follow, &mgr).await?;
        }
        cli::Commands::Describe { name } => {
            let mgr = manager.lock().await;
            commands::handle_describe(&name, &mgr).await?;
        }
        cli::Commands::Monit => {
            let mgr = manager.lock().await;
            commands::handle_monit(&mgr).await?;
        }
        cli::Commands::Cluster { name } => {
            let mgr = manager.lock().await;
            commands::handle_cluster(name.as_deref(), &mgr).await?;
        }
        cli::Commands::Scale { name, count } => {
            let mut mgr = manager.lock().await;
            commands::handle_scale(&name, count, &mut mgr).await?;
        }
        cli::Commands::Groups => {
            let mgr = manager.lock().await;
            commands::handle_groups(&mgr).await?;
        }
        cli::Commands::Dev { config, port } => {
            commands::handle_dev(&config, port).await?;
        }
        cli::Commands::Flush => {
            let mut mgr = manager.lock().await;
            commands::handle_flush(&mut mgr).await?;
        }
        cli::Commands::ReloadLogs { name: _, all: _ } => {
            let mgr = manager.lock().await;
            commands::handle_reload_logs(&mgr).await?;
        }
        cli::Commands::Save => {
            let mgr = manager.lock().await;
            commands::handle_save(&mgr).await?;
        }
        cli::Commands::Resurrect { dashboard, port } => {
            commands::handle_resurrect(dashboard, port).await?;
        }
        cli::Commands::Daemon { port } => {
            commands::handle_resurrect(true, port).await?;
        }
        cli::Commands::Startup { platform, user } => {
            commands::handle_startup(&platform, user.as_deref()).await?;
        }
        cli::Commands::Unstartup => {
            commands::handle_unstartup().await?;
        }
        cli::Commands::Reset { name, all: _ } => {
            let mut mgr = manager.lock().await;
            commands::handle_reset(name.as_deref(), &mut mgr).await?;
        }
        cli::Commands::Prettylist { name: _ } => {
            let mgr = manager.lock().await;
            commands::handle_prettylist(&mgr).await?;
        }
        cli::Commands::Serve { path, port } => {
            commands::handle_serve(&path, port).await?;
        }
        cli::Commands::Report { output } => {
            let mgr = manager.lock().await;
            commands::handle_report(output.as_deref(), &mgr).await?;
        }
        cli::Commands::Deploy { environment, config, repo, verbose: _ } => {
            commands::handle_deploy(&environment, config.as_deref(), repo.as_deref()).await?;
        }
        cli::Commands::Dashboard { port, host } => {
            commands::handle_dashboard(port, &host).await?;
        }
        cli::Commands::Install { name, config } => {
            let mut mgr = manager.lock().await;
            commands::handle_install(&name, &config, &mut mgr).await?;
        }
        cli::Commands::Build { name, config } => {
            let mut mgr = manager.lock().await;
            commands::handle_build(&name, &config, &mut mgr).await?;
        }
    }

    Ok(())
}
