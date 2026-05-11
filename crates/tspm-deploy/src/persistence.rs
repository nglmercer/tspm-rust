use std::path::PathBuf;
use tspm_core::{DumpData, ProcessConfig};

/// Manages save/resurrect of process state
pub struct PersistenceManager {
    home: PathBuf,
}

impl PersistenceManager {
    pub fn new() -> Self {
        let home = tspm_core::get_tspm_home();
        Self { home }
    }

    pub fn dump_path(&self) -> PathBuf {
        self.home.join("dump.json")
    }

    /// Save process configs to dump file
    pub fn save(&self, processes: &[ProcessConfig]) -> Result<(), String> {
        let data = DumpData {
            processes: processes.to_vec(),
            timestamp: chrono_local_now(),
            version: tspm_core::APP_VERSION.to_string(),
        };

        std::fs::create_dir_all(&self.home)
            .map_err(|e| format!("Failed to create TSPM home: {e}"))?;

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| format!("Failed to serialize: {e}"))?;

        std::fs::write(self.dump_path(), json)
            .map_err(|e| format!("Failed to write dump: {e}"))?;

        Ok(())
    }

    /// Load process configs from dump file
    pub fn load(&self) -> Option<DumpData> {
        let path = self.dump_path();
        if !path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }
}

impl Default for PersistenceManager {
    fn default() -> Self {
        Self::new()
    }
}

fn chrono_local_now() -> String {
    // Simple ISO timestamp without chrono dependency
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    
    // Rough ISO-like format
    format!("{}", secs)
}
