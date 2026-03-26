//! Launch a Chromium browser with CDP (Chrome DevTools Protocol) enabled.
//!
//! Discovers an existing CDP endpoint or launches the detected browser with
//! `--remote-debugging-port` and `--user-data-dir` so session cookies are
//! available without a Chrome extension.

use opencli_rs_core::CliError;
use std::path::PathBuf;
use std::time::Duration;

use crate::browser_detection::{self, BrowserInfo};

/// Default port range to scan/use for CDP.
const CDP_PORT_START: u16 = 9222;
const CDP_PORT_END: u16 = 9232;

/// How long to wait for the browser to become CDP-ready.
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(15);

/// Poll interval when waiting for CDP readiness.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// A discovered or launched CDP endpoint.
#[derive(Debug, Clone)]
pub struct CdpEndpoint {
    /// WebSocket URL for the first available page (e.g. `ws://127.0.0.1:9222/devtools/page/...`).
    pub ws_url: String,
    /// The CDP port.
    pub port: u16,
    /// Whether we launched the browser (true) or found an existing one (false).
    pub launched: bool,
}

/// Try to discover an existing CDP endpoint on the given port.
///
/// Checks `http://127.0.0.1:{port}/json` for available pages.
pub async fn discover_existing_cdp(port: u16) -> Option<CdpEndpoint> {
    let url = format!("http://127.0.0.1:{port}/json");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let pages: Vec<serde_json::Value> = resp.json().await.ok()?;
    // Find the first page-type target
    for page in &pages {
        if page.get("type").and_then(|t| t.as_str()) == Some("page") {
            if let Some(ws_url) = page.get("webSocketDebuggerUrl").and_then(|u| u.as_str()) {
                return Some(CdpEndpoint {
                    ws_url: ws_url.to_string(),
                    port,
                    launched: false,
                });
            }
        }
    }

    // Fall back to any target with a webSocketDebuggerUrl
    for page in &pages {
        if let Some(ws_url) = page.get("webSocketDebuggerUrl").and_then(|u| u.as_str()) {
            return Some(CdpEndpoint {
                ws_url: ws_url.to_string(),
                port,
                launched: false,
            });
        }
    }

    None
}

/// Find an available port in the CDP range.
fn find_available_port() -> Option<u16> {
    for port in CDP_PORT_START..CDP_PORT_END {
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Some(port);
        }
    }
    None
}

/// Fallback profile directory when the user's browser profile is locked.
fn fallback_profile_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".opencli-rs")
        .join("chrome-profile")
}

/// Launch a browser with CDP enabled on the given port.
fn launch_browser(
    info: &BrowserInfo,
    port: u16,
) -> Result<std::process::Child, CliError> {
    let profile_dir = match &info.user_data_dir {
        Some(dir) if !browser_detection::is_profile_locked(dir) => dir.clone(),
        _ => {
            let fb = fallback_profile_dir();
            std::fs::create_dir_all(&fb).ok();
            tracing::info!(
                "Browser profile locked or unavailable, using fallback: {}",
                fb.display()
            );
            fb
        }
    };

    tracing::info!(
        "Launching {} with CDP on port {} (profile: {})",
        info.name,
        port,
        profile_dir.display()
    );

    let child = std::process::Command::new(&info.path)
        .arg(format!("--remote-debugging-port={port}"))
        .arg(format!("--user-data-dir={}", profile_dir.display()))
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-blink-features=AutomationControlled")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| CliError::browser_connect(format!("Failed to launch {}: {e}", info.name)))?;

    Ok(child)
}

/// Wait for CDP to become ready on the given port.
async fn wait_for_cdp_ready(port: u16) -> Result<CdpEndpoint, CliError> {
    let deadline = tokio::time::Instant::now() + LAUNCH_TIMEOUT;

    while tokio::time::Instant::now() < deadline {
        if let Some(endpoint) = discover_existing_cdp(port).await {
            return Ok(CdpEndpoint {
                ws_url: endpoint.ws_url,
                port,
                launched: true,
            });
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }

    Err(CliError::browser_connect(format!(
        "Browser did not become CDP-ready on port {port} within {}s",
        LAUNCH_TIMEOUT.as_secs()
    )))
}

/// Discover an existing CDP-enabled browser or launch one.
///
/// 1. Scan ports for an existing CDP endpoint
/// 2. If not found, detect browser, find available port, launch, wait for ready
pub async fn connect_or_launch() -> Result<CdpEndpoint, CliError> {
    // 1. Check for existing CDP endpoint
    for port in CDP_PORT_START..CDP_PORT_END {
        if let Some(endpoint) = discover_existing_cdp(port).await {
            tracing::info!("Found existing CDP endpoint on port {port}");
            return Ok(endpoint);
        }
    }

    // 2. Detect browser
    let info = browser_detection::detect_browser().ok_or_else(|| {
        CliError::browser_connect(
            "No Chromium-based browser found. Install Chrome, Brave, Edge, or Chromium.",
        )
    })?;

    // 3. Find available port and launch
    let port = find_available_port().ok_or_else(|| {
        CliError::browser_connect(format!(
            "No available port in range {CDP_PORT_START}-{CDP_PORT_END}"
        ))
    })?;

    let mut child = launch_browser(&info, port)?;

    // 4. Wait for CDP readiness
    match wait_for_cdp_ready(port).await {
        Ok(endpoint) => {
            // Detach the child — browser should outlive the CLI
            std::mem::forget(child);
            Ok(endpoint)
        }
        Err(e) => {
            // Clean up on failure
            let _ = child.kill();
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_existing_cdp_no_server() {
        // Port 19999 should have nothing listening
        let result = discover_existing_cdp(19999).await;
        assert!(result.is_none());
    }

    #[test]
    fn test_find_available_port() {
        let port = find_available_port();
        // Should find at least one port (unless all 10 are in use)
        // Don't assert Some — CI might have them occupied
        if let Some(p) = port {
            assert!(p >= CDP_PORT_START && p < CDP_PORT_END);
        }
    }

    #[test]
    fn test_fallback_profile_dir() {
        let dir = fallback_profile_dir();
        assert!(dir.ends_with("chrome-profile"));
        assert!(dir.to_string_lossy().contains(".opencli-rs"));
    }

    #[test]
    fn test_cdp_endpoint_construction() {
        let endpoint = CdpEndpoint {
            ws_url: "ws://127.0.0.1:9222/devtools/page/abc".to_string(),
            port: 9222,
            launched: false,
        };
        assert_eq!(endpoint.port, 9222);
        assert!(!endpoint.launched);
        assert!(endpoint.ws_url.starts_with("ws://"));
    }
}
