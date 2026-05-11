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

        // Get the home directory of the user to ensure config discovery works
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());

        let unit = format!(
            r#"[Unit]
Description=TSPM Process Manager
After=network.target

[Service]
Type=simple
User={user}
Environment=HOME={home}
ExecStart={exe} daemon
ExecStop={exe} stop --all
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
"#,
            user = user,
            home = home,
            exe = current_exe.display(),
        );

        Ok(unit)
    }

    /// Install the systemd service file
    pub fn install_systemd(user: Option<&str>) -> Result<(), String> {
        let unit_content = Self::generate_systemd(user)?;
        let systemd_dir = PathBuf::from("/etc/systemd/system");
        let unit_path = systemd_dir.join("tspm.service");

        // Try to write the file. This will fail if not root, which is expected.
        fs::write(&unit_path, unit_content)
            .map_err(|e| format!("Failed to write service file to {}. Error: {}. (Hint: Run with sudo)", unit_path.display(), e))?;

        info!("[TSPM] systemd service installed at {}", unit_path.display());
        info!("[TSPM] Run the following commands to start:");
        info!("       sudo systemctl daemon-reload");
        info!("       sudo systemctl enable --now tspm");

        Ok(())
    }

    /// Remove the systemd service file
    pub fn uninstall_systemd() -> Result<(), String> {
        let unit_path = PathBuf::from("/etc/systemd/system/tspm.service");

        if unit_path.exists() {
            fs::remove_file(&unit_path)
                .map_err(|e| format!("Failed to remove service file. Error: {}. (Hint: Run with sudo)", e))?;
            info!("[TSPM] systemd service removed");
        }

        Ok(())
    }
}
