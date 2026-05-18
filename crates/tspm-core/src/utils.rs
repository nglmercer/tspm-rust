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

    // Common system locations
    let system_paths = [
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/usr/local/sbin",
        "/usr/sbin",
        "/sbin",
        "/snap/bin",
        "/usr/lib/node_modules/.bin",
    ];
    for p in system_paths {
        let pb = PathBuf::from(p);
        if pb.exists() && !paths.contains(&pb) {
            paths.push(pb);
        }
    }

    // Current user home
    if let Some(home) = dirs::home_dir() {
        add_user_paths(&home, &mut paths);
    }

    // If running under sudo, also check the original user's home
    if let Ok(sudo_user) = env::var("SUDO_USER") {
        let user_home = PathBuf::from("/home").join(sudo_user);
        if user_home.exists() {
            add_user_paths(&user_home, &mut paths);
        }
    }

    // Scan ALL /home/*/ directories for common tool paths
    // This ensures bun/node/etc are found regardless of which user installed them
    if let Ok(entries) = std::fs::read_dir("/home") {
        for entry in entries.flatten() {
            let home = entry.path();
            if home.is_dir() {
                add_user_paths(&home, &mut paths);
            }
        }
    }

    // Also check /root for root user installations
    let root_home = PathBuf::from("/root");
    if root_home.exists() {
        add_user_paths(&root_home, &mut paths);
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.clone()));

    env::join_paths(paths).unwrap_or_default().to_string_lossy().to_string()
}

fn add_user_paths(home: &Path, paths: &mut Vec<PathBuf>) {
    // Bun
    paths.push(home.join(".bun/bin"));
    // Cargo
    paths.push(home.join(".cargo/bin"));
    // Local bin
    paths.push(home.join(".local/bin"));
    // NVM (common locations)
    let nvm_dir = home.join(".nvm/versions/node");
    if nvm_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    paths.push(entry.path().join("bin"));
                }
            }
        }
    }
    // FNM (Fast Node Manager)
    paths.push(home.join(".fnm"));
    // Volta
    paths.push(home.join(".volta/bin"));
    // PNPM
    paths.push(home.join(".local/share/pnpm"));
    // Generic bin
    paths.push(home.join("bin"));
    // NPM global
    paths.push(home.join(".npm-global/bin"));
    // Yarn global
    paths.push(home.join(".config/yarn/global/node_modules/.bin"));
    // Corepack
    paths.push(home.join(".local/share/corepack"));
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
