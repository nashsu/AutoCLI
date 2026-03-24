use opencli_rs_core::{CliError, IPage};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::daemon_client::DaemonClient;
use crate::page::DaemonPage;

const DEFAULT_PORT: u16 = 19825;
const READY_TIMEOUT: Duration = Duration::from_secs(10);
const READY_POLL_INTERVAL: Duration = Duration::from_millis(200);
const EXTENSION_TIMEOUT: Duration = Duration::from_secs(30);
const EXTENSION_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// High-level bridge that manages the Daemon process and provides IPage instances.
/// The daemon runs as a detached background process with its own idle-shutdown lifecycle.
pub struct BrowserBridge {
    port: u16,
}

impl BrowserBridge {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Create a bridge using the default port.
    pub fn default_port() -> Self {
        Self::new(DEFAULT_PORT)
    }

    /// Connect to the daemon, starting it if necessary, and return a page.
    pub async fn connect(&mut self) -> Result<Arc<dyn IPage>, CliError> {
        let client = Arc::new(DaemonClient::new(self.port));

        if client.is_running().await {
            // A daemon is already running on the port — reuse it.
            // Both opencli and opencli-rs daemons share the same protocol
            // (HTTP + WebSocket + Chrome extension), so either one works.
            debug!(port = self.port, "daemon already running, reusing");
        } else {
            info!(port = self.port, "daemon not running, spawning");
            self.spawn_daemon().await?;
            self.wait_for_ready(&client).await?;
        }

        // Wait for extension to connect (it may need time to reconnect after daemon restart)
        if !self.wait_for_extension(&client).await {
            warn!("Chrome extension is not connected to the daemon");
            return Err(CliError::BrowserConnect {
                message: "Chrome extension not connected".into(),
                suggestions: vec![
                    "Install the OpenCLI Chrome extension".into(),
                    "Make sure Chrome is running with the extension enabled".into(),
                    format!("The daemon is listening on port {}", self.port),
                ],
                source: None,
            });
        }

        let page = DaemonPage::new(client, "default");
        Ok(Arc::new(page))
    }

    /// Spawn the daemon as a child process using --daemon flag on the current binary.
    async fn spawn_daemon(&mut self) -> Result<(), CliError> {
        let exe = std::env::current_exe().map_err(|e| {
            CliError::browser_connect(format!("Cannot determine current executable: {e}"))
        })?;

        let child = tokio::process::Command::new(exe)
            .arg("--daemon")
            .arg("--port")
            .arg(self.port.to_string())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| CliError::browser_connect(format!("Failed to spawn daemon: {e}")))?;

        info!(port = self.port, pid = ?child.id(), "daemon process spawned (detached)");
        // Detach: don't store the child handle so we don't kill it on drop.
        // The daemon manages its own lifecycle (5-min idle shutdown).
        // We need to forget the child to prevent tokio from killing it when dropped.
        std::mem::forget(child);
        Ok(())
    }

    /// Wait for the Chrome extension to connect to the daemon.
    /// The extension uses exponential backoff (2s, 4s, 8s, 16s, ..., max 60s) so
    /// we may need to wait up to 30s if it's in a long backoff cycle.
    async fn wait_for_extension(&self, client: &DaemonClient) -> bool {
        let start = tokio::time::Instant::now();
        let deadline = start + EXTENSION_TIMEOUT;
        let mut printed_waiting = false;

        while tokio::time::Instant::now() < deadline {
            if client.is_extension_connected().await {
                if printed_waiting {
                    eprintln!(); // newline after the dots
                }
                info!("Chrome extension connected");
                return true;
            }

            let elapsed = start.elapsed().as_secs();
            if elapsed >= 2 && !printed_waiting {
                eprint!("Waiting for Chrome extension to connect");
                printed_waiting = true;
            } else if printed_waiting && elapsed % 3 == 0 {
                eprint!(".");
            }

            tokio::time::sleep(EXTENSION_POLL_INTERVAL).await;
        }

        if printed_waiting {
            eprintln!();
        }
        false
    }

    /// Wait for the daemon to become ready by polling /health.
    async fn wait_for_ready(&self, client: &DaemonClient) -> Result<(), CliError> {
        let deadline = tokio::time::Instant::now() + READY_TIMEOUT;

        while tokio::time::Instant::now() < deadline {
            if client.is_running().await {
                info!("daemon is ready");
                return Ok(());
            }
            tokio::time::sleep(READY_POLL_INTERVAL).await;
        }

        Err(CliError::timeout(format!(
            "Daemon did not become ready within {}s",
            READY_TIMEOUT.as_secs()
        )))
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_construction() {
        let bridge = BrowserBridge::new(19825);
        assert_eq!(bridge.port, 19825);
    }

    #[test]
    fn test_bridge_default_port() {
        let bridge = BrowserBridge::default_port();
        assert_eq!(bridge.port, DEFAULT_PORT);
    }
}
