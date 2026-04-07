use autocli_core::CliError;
use std::io::Write;
use std::path::PathBuf;

const TOKEN_ENV: &str = "AUTOCLI_DAEMON_TOKEN";
const TOKEN_FILENAME: &str = "daemon-token";

fn home_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
}

fn normalize_token(raw: &str) -> Option<String> {
    let token = raw.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

pub fn daemon_token_path() -> PathBuf {
    home_dir().join(".autocli").join(TOKEN_FILENAME)
}

pub fn load_or_create_daemon_token() -> Result<String, CliError> {
    if let Ok(token) = std::env::var(TOKEN_ENV) {
        if let Some(token) = normalize_token(&token) {
            return Ok(token);
        }
    }

    let path = daemon_token_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| CliError::Config {
            message: format!("Failed to read daemon token from {}", path.display()),
            suggestions: vec![
                "Delete the token file so AutoCLI can recreate it".to_string(),
                format!("Path: {}", path.display()),
            ],
            source: Some(Box::new(e)),
        })?;
        if let Some(token) = normalize_token(&content) {
            return Ok(token);
        }
    }

    let token = uuid::Uuid::new_v4().simple().to_string();
    persist_token(&path, &token)?;
    Ok(token)
}

fn persist_token(path: &PathBuf, token: &str) -> Result<(), CliError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CliError::Config {
            message: format!("Failed to create config directory {}", parent.display()),
            suggestions: vec![],
            source: Some(Box::new(e)),
        })?;
    }

    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
    }
    .map_err(|e| CliError::Config {
        message: format!("Failed to write daemon token to {}", path.display()),
        suggestions: vec![],
        source: Some(Box::new(e)),
    })?;

    #[cfg(not(unix))]
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|e| CliError::Config {
            message: format!("Failed to write daemon token to {}", path.display()),
            suggestions: vec![],
            source: Some(Box::new(e)),
        })?;

    file.write_all(token.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|e| CliError::Config {
            message: format!("Failed to persist daemon token to {}", path.display()),
            suggestions: vec![],
            source: Some(Box::new(e)),
        })?;

    Ok(())
}
