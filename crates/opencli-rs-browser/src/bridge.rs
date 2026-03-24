use opencli_rs_core::{CliError, IPage};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::daemon_client::DaemonClient;
use crate::page::DaemonPage;

const DEFAULT_PORT: u16 = 19825;
const READY_TIMEOUT: Duration = Duration::from_secs(10);
const READY_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// High-level bridge that manages the Daemon process and provides IPage instances.
pub struct BrowserBridge {
    port: u16,
    daemon_process: Option<tokio::process::Child>,
}

impl BrowserBridge {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            daemon_process: None,
        }
    }

    /// Create a bridge using the default port.
    pub fn default_port() -> Self {
        Self::new(DEFAULT_PORT)
    }

    /// Connect to the daemon, starting it if necessary, and return a page.
    pub async fn connect(&mut self) -> Result<Arc<dyn IPage>, CliError> {
        let client = Arc::new(DaemonClient::new(self.port));

        // Check if daemon is already running
        if !client.is_running().await {
            info!(port = self.port, "daemon not running, spawning");
            self.spawn_daemon().await?;
            self.wait_for_ready(&client).await?;
        } else {
            debug!(port = self.port, "daemon already running");
        }

        // Check extension
        if !client.is_extension_connected().await {
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

        info!(port = self.port, pid = ?child.id(), "daemon process spawned");
        self.daemon_process = Some(child);
        Ok(())
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

    /// Close the bridge and kill the daemon process if we spawned it.
    pub async fn close(&mut self) -> Result<(), CliError> {
        if let Some(ref mut child) = self.daemon_process {
            debug!("killing daemon process");
            let _ = child.kill().await;
            self.daemon_process = None;
        }
        Ok(())
    }
}

impl Drop for BrowserBridge {
    fn drop(&mut self) {
        // Best effort: try to kill the child process if still running.
        if let Some(ref mut child) = self.daemon_process {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_construction() {
        let bridge = BrowserBridge::new(19825);
        assert_eq!(bridge.port, 19825);
        assert!(bridge.daemon_process.is_none());
    }

    #[test]
    fn test_bridge_default_port() {
        let bridge = BrowserBridge::default_port();
        assert_eq!(bridge.port, DEFAULT_PORT);
    }
}
