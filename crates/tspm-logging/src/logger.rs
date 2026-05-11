use crate::rotation::RotatingLog;
use std::fs;
use std::path::{Path, PathBuf};

/// Manages process log files with rotation
pub struct LogManager {
    rotator: RotatingLog,
}

impl LogManager {
    pub fn new() -> Self {
        Self {
            rotator: RotatingLog::default(),
        }
    }

    /// Flush (clear) a log file
    pub fn flush(path: &Path) -> Result<(), std::io::Error> {
        if path.exists() {
            fs::write(path, "")?;
        }
        Ok(())
    }

    /// Flush all logs for a process
    pub fn flush_process(stdout_path: &Path, stderr_path: &Path) -> Result<(), std::io::Error> {
        Self::flush(stdout_path)?;
        Self::flush(stderr_path)?;
        Ok(())
    }

    /// Read last N lines from a log file
    pub fn read_tail(path: &Path, lines: usize) -> Result<Vec<String>, std::io::Error> {
        if !path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(path)?;
        let all_lines: Vec<&str> = content.lines().collect();
        let start = if all_lines.len() > lines {
            all_lines.len() - lines
        } else {
            0
        };

        Ok(all_lines[start..].iter().map(|s| s.to_string()).collect())
    }

    /// Check and rotate log if needed
    pub fn rotate_if_needed(&self, path: &Path) {
        self.rotator.rotate(path);
    }

    /// Get all log files for a path
    pub fn get_log_files(&self, path: &Path) -> Vec<PathBuf> {
        self.rotator.get_log_files(path)
    }
}

impl Default for LogManager {
    fn default() -> Self {
        Self::new()
    }
}
