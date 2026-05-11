use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// Log rotation manager
pub struct RotatingLog {
    max_size: u64,
    max_files: usize,
}

impl RotatingLog {
    pub fn new(max_size: u64, max_files: usize) -> Self {
        Self { max_size, max_files }
    }

    pub fn rotate(&self, path: &Path) {
        let Ok(meta) = fs::metadata(path) else { return };
        if meta.len() < self.max_size {
            return;
        }

        for i in (1..self.max_files).rev() {
            let old = if i == 1 {
                path.to_path_buf()
            } else {
                path.with_extension(format!("{}.log", i - 1))
            };
            let new = path.with_extension(format!("{}.log", i));

            if old.exists() {
                let _ = fs::rename(&old, &new);
            }
        }

        // Truncate the original
        let _ = fs::File::create(path);
        info!("[TSPM] Rotated log: {}", path.display());
    }

    pub fn get_log_files(&self, path: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if path.exists() {
            files.push(path.to_path_buf());
        }
        for i in 1..self.max_files {
            let p = path.with_extension(format!("{}.log", i));
            if p.exists() {
                files.push(p);
            }
        }
        files
    }
}

impl Default for RotatingLog {
    fn default() -> Self {
        Self::new(10 * 1024 * 1024, 5) // 10MB, 5 files
    }
}
