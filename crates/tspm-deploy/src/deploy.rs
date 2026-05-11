use tracing::info;
use tspm_core::DeploymentEnvConfig;

/// Handles remote deployment via SSH
pub struct Deployer;

impl Deployer {
    /// Deploy to a remote environment
    pub async fn deploy(
        env_name: &str,
        env_config: &DeploymentEnvConfig,
        repo: Option<&str>,
    ) -> Result<(), String> {
        let host = &env_config.host;
        let user = &env_config.user;
        let port = env_config.port;
        let path = &env_config.path;

        info!("[TSPM] Deploying to '{env_name}' at {user}@{host}:{port}{path}");

        // Run pre-deploy hooks
        if let Some(ref pre) = env_config.pre_deploy {
            Self::run_scripts(pre).await?;
        }

        // Determine the repository URL
        let repo_url = repo.or(env_config.r#ref.as_deref().map(|_| "")).unwrap_or("");

        if repo_url.is_empty() {
            return Err("No repository URL specified for deployment".to_string());
        }

        let ref_name = env_config.r#ref.as_deref().unwrap_or("main");

        let deploy_script = format!(
            r#"set -e
ssh -p {port} {user}@{host} "
    mkdir -p {path} &&
    cd {path} &&
    if [ -d .git ]; then
        git fetch && git checkout {ref_name} && git pull origin {ref_name};
    else
        git clone -b {ref_name} {repo_url} . 2>/dev/null || git clone {repo_url} . && git checkout {ref_name};
    fi
"
"#
        );

        info!("[TSPM] Running deploy script...");

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&deploy_script)
            .output()
            .await
            .map_err(|e| format!("Failed to run deploy: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Deploy failed: {stderr}"));
        }

        // Run post-deploy hooks
        if let Some(ref post) = env_config.post_deploy {
            Self::run_scripts(post).await?;
        }

        info!("[TSPM] Deployment to '{env_name}' completed");
        Ok(())
    }

    async fn run_scripts(scripts: &tspm_core::DeployScripts) -> Result<(), String> {
        let cmds = match scripts {
            tspm_core::DeployScripts::Single(s) => vec![s.clone()],
            tspm_core::DeployScripts::Multiple(v) => v.clone(),
        };

        for cmd in &cmds {
            info!("[TSPM] Running: {cmd}");
            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .await
                .map_err(|e| format!("Hook failed: {e}"))?;

            if !output.status.success() {
                tracing::warn!(
                    "[TSPM] Script failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        Ok(())
    }
}
