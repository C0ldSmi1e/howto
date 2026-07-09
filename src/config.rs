use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_MODEL: &str = "claude-haiku-4-5";

#[derive(Debug, Default, Deserialize)]
pub struct FileConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub shell: Option<String>,
}

#[derive(Debug)]
pub struct Config {
    pub api_key: Option<String>,
    pub api_key_source: &'static str,
    pub model: String,
    pub model_source: &'static str,
    pub shell_override: Option<String>,
}

pub fn config_dir() -> PathBuf {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir).join("howto"),
        _ => crate::state::home().join(".config/howto"),
    }
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

const TEMPLATE: &str = r#"# howto configuration
# Environment variables and flags override anything set here.

# Anthropic API key. Prefer the ANTHROPIC_API_KEY environment variable.
# api_key = ""

# Model used to generate commands (env override: HOWTO_MODEL).
# model = "claude-haiku-4-5"

# Override shell detection: zsh, bash, or fish. Default: basename of $SHELL.
# shell = ""
"#;

/// Creates the commented config template on first run. Returns true if created now.
pub fn ensure_template() -> Result<bool> {
    let path = config_path();
    if path.exists() {
        return Ok(false);
    }
    fs::create_dir_all(config_dir())?;
    fs::write(&path, TEMPLATE)?;
    Ok(true)
}

pub fn load() -> Result<Config> {
    let file: FileConfig = match fs::read_to_string(config_path()) {
        Ok(raw) => toml::from_str(&raw)
            .with_context(|| format!("invalid config at {}", config_path().display()))?,
        Err(_) => FileConfig::default(),
    };

    let (api_key, api_key_source) = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) if !k.is_empty() => (Some(k), "env ANTHROPIC_API_KEY"),
        _ => match file.api_key.filter(|k| !k.is_empty()) {
            Some(k) => (Some(k), "config file"),
            None => (None, "set ANTHROPIC_API_KEY, or api_key in the config file"),
        },
    };

    let (model, model_source) = match std::env::var("HOWTO_MODEL") {
        Ok(m) if !m.is_empty() => (m, "env HOWTO_MODEL"),
        _ => match file.model.filter(|m| !m.is_empty()) {
            Some(m) => (m, "config file"),
            None => (DEFAULT_MODEL.to_string(), "default"),
        },
    };

    Ok(Config {
        api_key,
        api_key_source,
        model,
        model_source,
        shell_override: file.shell.filter(|s| !s.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_is_valid_toml_with_all_keys_commented() {
        let cfg: FileConfig = toml::from_str(TEMPLATE).unwrap();
        assert!(cfg.api_key.is_none());
        assert!(cfg.model.is_none());
        assert!(cfg.shell.is_none());
    }

    #[test]
    fn file_config_parses_set_values() {
        let cfg: FileConfig = toml::from_str("model = \"claude-sonnet-5\"\nshell = \"fish\"").unwrap();
        assert_eq!(cfg.model.as_deref(), Some("claude-sonnet-5"));
        assert_eq!(cfg.shell.as_deref(), Some("fish"));
    }
}
