use autocli_core::CliError;
use std::path::PathBuf;

use crate::types::ExternalCli;

/// Embedded default external CLIs from resources/external-clis.yaml
const BUILTIN_EXTERNAL_CLIS: &str = include_str!("../resources/external-clis.yaml");

/// Return the path to the user's external-clis.yaml override file.
pub fn user_external_clis_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".autocli")
        .join("external-clis.yaml")
}

fn load_user_external_clis() -> Result<Vec<ExternalCli>, CliError> {
    let user_path = user_external_clis_path();
    if !user_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&user_path).map_err(|e| CliError::Config {
        message: format!("Failed to read {}", user_path.display()),
        suggestions: vec![],
        source: Some(Box::new(e)),
    })?;

    if content.trim().is_empty() {
        return Ok(vec![]);
    }

    serde_yaml::from_str::<Vec<ExternalCli>>(&content).map_err(|e| CliError::Config {
        message: format!("Failed to parse {}", user_path.display()),
        suggestions: vec![
            "Fix the YAML syntax or remove the broken file".to_string(),
            format!("Path: {}", user_path.display()),
        ],
        source: Some(Box::new(e)),
    })
}

fn write_user_external_clis(clis: &[ExternalCli]) -> Result<PathBuf, CliError> {
    let user_path = user_external_clis_path();
    if let Some(parent) = user_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CliError::Config {
            message: format!("Failed to create {}", parent.display()),
            suggestions: vec![],
            source: Some(Box::new(e)),
        })?;
    }

    let yaml = serde_yaml::to_string(clis).map_err(|e| CliError::Config {
        message: format!("Failed to serialize {}", user_path.display()),
        suggestions: vec![],
        source: Some(Box::new(e)),
    })?;

    std::fs::write(&user_path, yaml).map_err(|e| CliError::Config {
        message: format!("Failed to write {}", user_path.display()),
        suggestions: vec![],
        source: Some(Box::new(e)),
    })?;

    Ok(user_path)
}

/// Load external CLI definitions from the embedded resource and optionally
/// from the user's `~/.autocli/external-clis.yaml`.
///
/// User definitions are merged on top: if a user defines a CLI with the same
/// `name` as a builtin one, the user version wins.
pub fn load_external_clis() -> Result<Vec<ExternalCli>, CliError> {
    let mut clis: Vec<ExternalCli> = serde_yaml::from_str(BUILTIN_EXTERNAL_CLIS)?;

    match load_user_external_clis() {
        Ok(user_clis) => {
            for ucli in user_clis {
                if let Some(pos) = clis.iter().position(|c| c.name == ucli.name) {
                    clis[pos] = ucli;
                } else {
                    clis.push(ucli);
                }
            }
            tracing::debug!(path = ?user_external_clis_path(), "Loaded user external CLIs");
        }
        Err(e) => {
            tracing::warn!(error = %e, path = ?user_external_clis_path(), "Failed to load user external-clis.yaml");
        }
    }

    Ok(clis)
}

pub fn upsert_external_cli(cli: ExternalCli) -> Result<(PathBuf, bool), CliError> {
    let mut clis = load_user_external_clis()?;
    let updated = if let Some(existing) = clis.iter_mut().find(|existing| existing.name == cli.name)
    {
        *existing = cli;
        true
    } else {
        clis.push(cli);
        false
    };

    clis.sort_by(|a, b| a.name.cmp(&b.name));
    let path = write_user_external_clis(&clis)?;
    Ok((path, updated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_builtin_external_clis() {
        let clis = load_external_clis().unwrap();
        assert!(!clis.is_empty());
        // gh should be present in builtins
        assert!(clis.iter().any(|c| c.name == "gh"));
        assert!(clis.iter().any(|c| c.name == "docker"));
    }

    #[test]
    fn test_builtin_yaml_parses() {
        let clis: Vec<ExternalCli> = serde_yaml::from_str(BUILTIN_EXTERNAL_CLIS).unwrap();
        assert!(clis.len() >= 6);
        let gh = clis.iter().find(|c| c.name == "gh").unwrap();
        assert_eq!(gh.binary, "gh");
        assert!(!gh.tags.is_empty());
    }

    #[test]
    fn test_upsert_external_cli_replaces_by_name() {
        let mut existing = vec![ExternalCli {
            name: "gh".to_string(),
            binary: "gh".to_string(),
            description: "old".to_string(),
            homepage: None,
            tags: vec![],
            install: Default::default(),
        }];

        let replacement = ExternalCli {
            name: "gh".to_string(),
            binary: "/usr/local/bin/gh".to_string(),
            description: "new".to_string(),
            homepage: Some("https://cli.github.com".to_string()),
            tags: vec!["github".to_string()],
            install: Default::default(),
        };

        let updated = if let Some(entry) = existing
            .iter_mut()
            .find(|entry| entry.name == replacement.name)
        {
            *entry = replacement;
            true
        } else {
            false
        };

        assert!(updated);
        assert_eq!(existing[0].binary, "/usr/local/bin/gh");
        assert_eq!(existing[0].description, "new");
    }
}
