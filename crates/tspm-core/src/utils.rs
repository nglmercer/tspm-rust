use std::path::{Path, PathBuf};
use std::env;

/// Get the augmented PATH that includes common runtime directories
pub fn get_augmented_path() -> String {
    let mut paths = vec![];

    // Current PATH
    if let Ok(path) = env::var("PATH") {
        for p in env::split_paths(&path) {
            paths.push(p);
        }
    }

    // Common locations
    let mut extra_paths = vec![];

    if let Some(home) = dirs::home_dir() {
        // Bun
        extra_paths.push(home.join(".bun/bin"));
        // Cargo
        extra_paths.push(home.join(".cargo/bin"));
        // Local bin
        extra_paths.push(home.join(".local/bin"));
        // NVM (common locations)
        let nvm_dir = home.join(".nvm/versions/node");
        if nvm_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        extra_paths.push(entry.path().join("bin"));
                    }
                }
            }
        }
        // FNM (Fast Node Manager)
        extra_paths.push(home.join(".fnm"));
        // Volta
        extra_paths.push(home.join(".volta/bin"));
    }

    // System common locations that might be missing in some environments
    extra_paths.push(PathBuf::from("/usr/local/bin"));
    extra_paths.push(PathBuf::from("/usr/bin"));
    extra_paths.push(PathBuf::from("/bin"));
    extra_paths.push(PathBuf::from("/usr/local/sbin"));
    extra_paths.push(PathBuf::from("/usr/sbin"));
    extra_paths.push(PathBuf::from("/sbin"));

    for p in extra_paths {
        if p.exists() && !paths.contains(&p) {
            paths.push(p);
        }
    }

    env::join_paths(paths).unwrap_or_default().to_string_lossy().to_string()
}

/// Detect if a directory has a package.json and return its content if it does
pub fn read_package_json(dir: &Path) -> Option<serde_json::Value> {
    let path = dir.join("package.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            return serde_json::from_str(&content).ok();
        }
    }
    None
}
