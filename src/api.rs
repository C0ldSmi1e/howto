use crate::config::Config;
use crate::response::Suggestion;
use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const TOOL_NAME: &str = "suggest_command";

pub fn suggest(
    cfg: &Config,
    machine: &str,
    query: &str,
    explain: bool,
    verbose: bool,
) -> Result<Suggestion> {
    let Some(api_key) = cfg.api_key.as_deref() else {
        bail!(
            "no API key — set ANTHROPIC_API_KEY, or api_key in {} (howto --config)",
            crate::config::config_path().display()
        );
    };

    let body = json!({
        "model": cfg.model,
        "max_tokens": 1024,
        "system": system_prompt(explain),
        "messages": [{ "role": "user", "content": format!("Environment:\n{machine}\n\nRequest: {query}") }],
        "tools": [tool_definition()],
        "tool_choice": { "type": "tool", "name": TOOL_NAME },
    });

    let started = std::time::Instant::now();
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(60))
        .build();
    let response = agent
        .post(API_URL)
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_json(&body);

    let value: Value = match response {
        Ok(r) => r.into_json().context("invalid JSON from the API")?,
        Err(ureq::Error::Status(code, r)) => {
            let raw = r.into_string().unwrap_or_default();
            let message = serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
                .unwrap_or(raw);
            let hint = match code {
                401 => " (check your API key)",
                404 => " (unknown model — check `model` in howto --config)",
                429 => " (rate limited — retry in a moment)",
                _ => "",
            };
            bail!("API error {code}{hint}: {message}");
        }
        Err(e) => {
            return Err(anyhow!(e)).context("network error reaching api.anthropic.com");
        }
    };

    if verbose {
        eprintln!(
            "howto: model={} latency={}ms",
            cfg.model,
            started.elapsed().as_millis()
        );
    }

    let input = value["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|b| b["type"] == "tool_use"))
        .map(|b| b["input"].clone())
        .ok_or_else(|| anyhow!("the API response contained no suggestion"))?;

    if verbose {
        eprintln!("howto: raw suggestion: {input}");
    }

    serde_json::from_value(input).context("could not parse the model's suggestion")
}

fn system_prompt(explain: bool) -> String {
    let mut p = String::from(
        "You are the backend of `howto`, a CLI that turns a natural-language request into ONE shell command.

Rules:
- Target exactly the user's environment (OS, shell, and the listed available tools). Never suggest a binary listed as not installed when an installed one can do the job.
- `command` is a single line for the user's shell: no leading `$`, no markdown, no comments. Pipes, `&&` and `;` are fine.
- Prefer safe defaults; do not add `sudo` unless the task requires it.
- No interactive programs (vim, top, less) unless the request asks for one.
- `explanation` is one short plain-language sentence.
- `alternatives`: at most 2, and only genuinely different approaches (a different tool or strategy, not a flag variation). Usually zero or one. Never repeat the main command.
- `danger`: none = read-only; low = creates or modifies files reversibly; medium = kills processes, overwrites files, or affects the network; high = irreversible destruction or system-wide risk (recursive force-deletes outside the working directory, writing to raw devices, mkfs, piping a download into a shell, force-pushing shared branches). Include `danger_reason` (one short sentence) only when danger is not none.
- If no sensible single command exists or the request is not something a shell command can do, set `command` to an empty string and `cannot_help` to a one-line reason. Otherwise do not include `cannot_help`.",
    );
    if explain {
        p.push_str("\n- `breakdown`: split the command into its meaningful parts, in order, with a plain-English meaning for each. Cover every pipe stage and significant flag.");
    } else {
        p.push_str("\n- Do not include `breakdown`.");
    }
    p
}

fn tool_definition() -> Value {
    // Only always-applicable fields are required; the rest are omitted when
    // inapplicable. Forcing "empty string" placeholders made the model emit
    // tool-syntax garbage into them.
    json!({
        "name": TOOL_NAME,
        "description": "Return the shell command that fulfils the user's request, with metadata.",
        "input_schema": {
            "type": "object",
            "additionalProperties": false,
            "required": ["command", "explanation", "danger"],
            "properties": {
                "command": { "type": "string" },
                "explanation": { "type": "string" },
                "danger": { "type": "string", "enum": ["none", "low", "medium", "high"] },
                "danger_reason": { "type": "string" },
                "alternatives": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["command", "note"],
                        "properties": {
                            "command": { "type": "string" },
                            "note": { "type": "string" }
                        }
                    }
                },
                "breakdown": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["part", "meaning"],
                        "properties": {
                            "part": { "type": "string" },
                            "meaning": { "type": "string" }
                        }
                    }
                },
                "cannot_help": { "type": "string" }
            }
        }
    })
}
