use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tspm")]
#[command(version = tspm_core::APP_VERSION)]
#[command(about = "TSPM - Rust Process Manager (PM2 alternative)")]
#[command(long_about = "A CLI for managing processes with clustering, health checks, deployment, and more.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a process or ecosystem file
    Start {
        /// Configuration file path (TOML or package.json)
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,

        /// Start only the specified process by name
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Enable file watching for auto-restart
        #[arg(short = 'w', long)]
        watch: bool,

        /// Run in daemon mode (background)
        #[arg(short = 'd', long)]
        daemon: bool,

        /// Environment variables to set
        #[arg(short = 'e', long)]
        env: Vec<String>,
    },

    /// Stop a running process
    Stop {
        /// Stop only the specified process by name
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Stop all running processes
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Restart a running process
    Restart {
        /// Configuration file path
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,

        /// Restart only the specified process by name
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Restart all processes
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Reload process(es) without downtime
    Reload {
        /// Configuration file path
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,

        /// Reload only the specified process by name
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Reload all processes
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Delete a process from the list
    Delete {
        /// Delete the specified process by name
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Delete all processes
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// List all managed processes
    List,

    /// Show logs for a process
    Logs {
        /// Show logs for the specified process
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Number of lines to show
        #[arg(short = 'l', long, default_value = "50")]
        lines: usize,

        /// Follow log output
        #[arg(short = 'f', long)]
        follow: bool,

        /// Show timestamps
        #[arg(short = 't', long)]
        timestamp: bool,
    },

    /// Show detailed information about a process
    Describe {
        /// Process name
        name: String,
    },

    /// Real-time process monitoring
    Monit,

    /// Show cluster information for a process
    Cluster {
        /// Process name
        name: Option<String>,
    },

    /// Scale cluster instances
    Scale {
        /// Process name to scale
        name: String,

        /// Number of instances
        count: u32,
    },

    /// Show process groups and namespaces
    Groups,

    /// Start processes in development mode with hot-reload
    Dev {
        /// Configuration file path
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,

        /// API port
        #[arg(short = 'p', long, default_value = "3000")]
        port: u16,
    },

    /// Flush all logs (clear log files)
    Flush,

    /// Reload log files (reopen for external rotation)
    ReloadLogs {
        /// Reload logs for the specified process
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Reload logs for all processes
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Save current process list to dump file
    Save,

    /// Restore processes from dump file
    Resurrect {
        /// Start the dashboard as well
        #[arg(short = 'd', long)]
        dashboard: bool,

        /// Dashboard port
        #[arg(short = 'p', long, default_value = "3000")]
        port: u16,
    },

    /// Run as a daemon (resurrect + dashboard)
    Daemon {
        /// Dashboard port
        #[arg(short = 'p', long, default_value = "3000")]
        port: u16,
    },

    /// Generate system startup script
    Startup {
        /// Platform (systemd only for now)
        #[arg(default_value = "systemd")]
        platform: String,

        /// User to run the service as
        #[arg(short = 'u', long)]
        user: Option<String>,
    },

    /// Remove startup script
    Unstartup,

    /// Reset all metrics for a process
    Reset {
        /// Reset metrics for the specified process
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Reset metrics for all processes
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Pretty-printed JSON process list
    Prettylist {
        /// Show details for the specified process
        #[arg(short = 'n', long)]
        name: Option<String>,
    },

    /// Serve static files from a directory
    Serve {
        /// Directory to serve
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Port to listen on
        #[arg(short = 'p', long, default_value = "8080")]
        port: u16,
    },

    /// Generate diagnostic report
    Report {
        /// Output file path
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },

    /// Deploy application to remote server via SSH
    Deploy {
        /// Environment to deploy to
        #[arg(default_value = "production")]
        environment: String,

        /// Configuration file path
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,

        /// Git repository URL
        #[arg(long)]
        repo: Option<String>,

        /// Verbose output
        #[arg(short = 'v', long)]
        verbose: bool,
    },

    /// Start the TSPM Web Dashboard
    Dashboard {
        /// Port to listen on
        #[arg(short = 'p', long, default_value = "3000")]
        port: u16,

        /// Host to listen on
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },

    /// Install dependencies for a process
    Install {
        /// Process name
        name: String,
        /// Configuration file path
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,
    },

    /// Build a process
    Build {
        /// Process name
        name: String,
        /// Configuration file path
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,
    },

    /// Get the active dashboard port
    Port,
}
