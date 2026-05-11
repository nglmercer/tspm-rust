use std::process::Command;
use std::path::Path;

fn main() {
    // Only build if the web-preact directory exists
    let web_preact_dir = Path::new("../../web-preact");
    if !web_preact_dir.exists() {
        return;
    }

    println!("cargo:rerun-if-changed=../../web-preact/src");
    println!("cargo:rerun-if-changed=../../web-preact/index.html");
    println!("cargo:rerun-if-changed=../../web-preact/package.json");
    println!("cargo:rerun-if-changed=../../web-preact/vite.config.ts");

    // Check if we should skip the build (e.g. for CI or if forced)
    if std::env::var("SKIP_DASHBOARD_BUILD").is_ok() {
        return;
    }

    println!("cargo:warning=[TSPM] Building dashboard...");
    
    // Try bun first, then npm
    let status = if Command::new("bun").arg("--version").output().is_ok() {
        Command::new("bun")
            .args(&["run", "build"])
            .current_dir(web_preact_dir)
            .status()
    } else {
        Command::new("npm")
            .args(&["run", "build"])
            .current_dir(web_preact_dir)
            .status()
    };

    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=[TSPM] Dashboard built successfully");
        }
        _ => {
            println!("cargo:warning=[TSPM] Dashboard build failed. Please build manually in web-preact/");
        }
    }
}
