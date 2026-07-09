use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Danger {
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub command: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownPart {
    pub part: String,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub command: String,
    pub explanation: String,
    pub danger: Danger,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub danger_reason: String,
    #[serde(default)]
    pub alternatives: Vec<Alternative>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breakdown: Vec<BreakdownPart>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cannot_help: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_without_optional_fields() {
        let s: Suggestion = serde_json::from_str(
            r#"{"command":"ls","explanation":"lists files","danger":"none"}"#,
        )
        .unwrap();
        assert!(s.alternatives.is_empty());
        assert!(s.cannot_help.is_empty());
    }

    #[test]
    fn danger_levels_are_ordered() {
        assert!(Danger::High > Danger::Medium);
        assert!(Danger::Medium > Danger::Low);
        assert!(Danger::Low > Danger::None);
    }
}
