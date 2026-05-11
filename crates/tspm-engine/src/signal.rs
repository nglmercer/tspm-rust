use tokio::signal;
use tracing::info;

/// Handle OS signals for graceful shutdown
pub struct SignalHandler;

impl SignalHandler {
    /// Wait for shutdown signal (SIGTERM or SIGINT)
    pub async fn wait_for_shutdown() {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
            info!("Received SIGINT (Ctrl+C)");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
            info!("Received SIGTERM");
        };

        #[cfg(unix)]
        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        #[cfg(not(unix))]
        ctrl_c.await;
    }
}
