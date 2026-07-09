use crate::response::Suggestion;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct LastResponse {
    pub query: String,
    pub suggestion: Suggestion,
    pub saved_at: u64,
}

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

pub fn state_dir() -> PathBuf {
    match std::env::var("XDG_STATE_HOME") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir).join("howto"),
        _ => home().join(".local/state/howto"),
    }
}

fn last_path() -> PathBuf {
    state_dir().join("last.json")
}

pub fn save(query: &str, suggestion: &Suggestion) -> Result<()> {
    fs::create_dir_all(state_dir())?;
    let entry = LastResponse {
        query: query.to_string(),
        suggestion: suggestion.clone(),
        saved_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    fs::write(last_path(), serde_json::to_string_pretty(&entry)?)?;
    Ok(())
}

pub fn load() -> Result<LastResponse> {
    let raw = fs::read_to_string(last_path())
        .map_err(|_| anyhow::anyhow!("no previous response yet — ask something first"))?;
    serde_json::from_str(&raw)
        .map_err(|_| anyhow::anyhow!("the previous response is unreadable — ask something new"))
}

pub fn hint_pending() -> bool {
    !state_dir().join("hint-shown").exists()
}

pub fn mark_hint_shown() {
    let _ = fs::create_dir_all(state_dir());
    let _ = fs::write(state_dir().join("hint-shown"), "");
}
