use std::fs;
use std::path::PathBuf;
use tracing::info;

/// Manages system startup scripts (systemd only for now)
pub struct StartupManager;

impl StartupManager {
    /// Generate a systemd service file
    pub fn generate_systemd(user: Option<&str>) -> Result<String, String> {
        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Failed to get current exe: {e}"))?;

        // Determine which user to use
        let user = user.map(|u| u.to_string()).unwrap_or_else(|| {
            std::env::var("USER").unwrap_or_else(|_| "root".to_string())
        });

        let unit = format!(
            r#"[Unit]
Description=TSPM Process Manager
After=network.target

[Service]
Type=forking
User={user}
ExecStart={exe} resurrect
ExecStop={exe} stop --all
ExecReload={exe} reload --all
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
"#,
            user = user,
            exe = current_exe.display(),
        );

        Ok(unit)
    }

    /// Install the systemd service file
    pub fn install_systemd(user: Option<&str>) -> Result<(), String> {
        let unit_content = Self::generate_systemd(user)?;
        let systemd_dir = PathBuf::from("/etc/systemd/system");
        let unit_path = systemd_dir.join("tspm.service");

        // On most systems this requires sudo
        fs::create_dir_all(&systemd_dir)
            .map_err(|e| format!("Cannot create systemd dir (need sudo?): {e}"))?;

        fs::write(&unit_path, unit_content)
            .map_err(|e| format!("Cannot write systemd unit (need sudo?): {e}"))?;

        info!("[TSPM] systemd service installed at {}", unit_path.display());
        info!("[TSPM] Run: sudo systemctl daemon-reload && sudo systemctl enable tspm");

        Ok(())
    }

    /// Remove the systemd service file
    pub fn uninstall_systemd() -> Result<(), String> {
        let unit_path = PathBuf::from("/etc/systemd/system/tspm.service");

        if unit_path.exists() {
            fs::remove_file(&unit_path)
                .map_err(|e| format!("Cannot remove systemd unit (need sudo?): {e}"))?;
            info!("[TSPM] systemd service removed");
        }

        Ok(())
    }
}
