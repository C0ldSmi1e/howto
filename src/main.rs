mod api;
mod cli;
mod config;
mod context;
mod danger;
mod init;
mod render;
mod response;
mod state;

use anyhow::{bail, Result};
use clap::Parser;
use render::Mode;
use response::Danger;

fn main() {
    let cli = match cli::Cli::try_parse() {
        Ok(cli) => cli,
        // Map clap's usage errors to exit 1 (2 is reserved for "model could not help").
        Err(e) => {
            let code = if e.use_stderr() { 1 } else { 0 };
            let _ = e.print();
            std::process::exit(code);
        }
    };
    match run(cli) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("howto: {e:#}");
            std::process::exit(1);
        }
    }
}

fn run(cli: cli::Cli) -> Result<i32> {
    if let Some(shell) = cli.init.as_deref() {
        print!("{}", init::wrapper(shell)?);
        return Ok(0);
    }

    let config_created = config::ensure_template().unwrap_or(false);
    let cfg = config::load()?;

    if cli.config {
        return show_config(&cfg, config_created);
    }

    let mode = render::resolve(cli.json, cli.print);

    if cli.last {
        return show_last(mode);
    }

    if cli.query.is_empty() {
        eprintln!("usage: howto <what you want to do>    (howto --help for details)");
        return Ok(1);
    }

    // Flags come before the query; a leading dash here is a typo'd flag, not a
    // request — reject it instead of spending an API call on it.
    if cli.query[0].starts_with('-') {
        eprintln!(
            "howto: unknown option '{}' — flags go before the query (howto --help)",
            cli.query[0]
        );
        return Ok(1);
    }

    if cli.query.len() == 1 {
        if let Ok(n) = cli.query[0].parse::<usize>() {
            return recall(mode, n);
        }
    }

    let query = cli.query.join(" ");
    let machine = context::gather(cfg.shell_override.as_deref());
    if cli.verbose {
        eprintln!(
            "howto: mode={mode:?} model={} shell={} os={} ({})",
            cfg.model, machine.shell, machine.os, machine.arch
        );
        eprintln!("howto: available: {}", machine.available.join(", "));
        if !machine.markers.is_empty() {
            eprintln!("howto: cwd markers: {}", machine.markers.join(", "));
        }
    }

    let mut suggestion = api::suggest(&cfg, &machine.to_prompt(), &query, cli.explain, cli.verbose)?;

    // A usable command trumps a stray cannot_help — gate on the command itself.
    if suggestion.command.trim().is_empty() {
        if mode == Mode::Json {
            println!("{}", serde_json::to_string_pretty(&suggestion)?);
        } else {
            let why: &str = if suggestion.cannot_help.is_empty() {
                "the model returned no usable command"
            } else {
                suggestion.cannot_help.as_str()
            };
            eprintln!("howto: {why}");
        }
        return Ok(2);
    }

    danger::escalate(&mut suggestion);
    if let Err(e) = state::save(&query, &suggestion) {
        if cli.verbose {
            eprintln!("howto: could not save last response: {e}");
        }
    }
    render::suggestion(mode, &suggestion);
    Ok(0)
}

fn show_last(mode: Mode) -> Result<i32> {
    let last = state::load()?;
    match mode {
        Mode::Json => println!("{}", serde_json::to_string_pretty(&last)?),
        Mode::Print => println!("{}", last.suggestion.command),
        Mode::Wrapped | Mode::Tty => render::menu(&last),
    }
    Ok(0)
}

fn recall(mode: Mode, n: usize) -> Result<i32> {
    let last = state::load()?;
    let s = &last.suggestion;
    let total = 1 + s.alternatives.len();
    if n == 0 || n > total {
        bail!("the last response has {total} command(s) — pick 1 to {total} (`howto --last` shows them)");
    }
    let (command, note) = if n == 1 {
        (s.command.as_str(), s.explanation.as_str())
    } else {
        let alt = &s.alternatives[n - 2];
        (alt.command.as_str(), alt.note.as_str())
    };

    let warning: Option<&str> = if n == 1 && s.danger >= Danger::Medium {
        Some(if s.danger_reason.is_empty() { "flagged as risky" } else { s.danger_reason.as_str() })
    } else {
        danger::check(command)
    };

    if mode == Mode::Json {
        let mut obj = serde_json::json!({ "command": command, "note": note });
        if let Some(w) = warning {
            obj["warning"] = serde_json::Value::String(w.to_string());
        }
        println!("{}", serde_json::to_string_pretty(&obj)?);
        return Ok(0);
    }
    render::recalled(mode, command, note, warning);
    Ok(0)
}

fn show_config(cfg: &config::Config, just_created: bool) -> Result<i32> {
    let (shell, shell_source) = context::shell_name(cfg.shell_override.as_deref());
    println!("config   {}", config::config_path().display());
    println!("state    {}", state::state_dir().join("last.json").display());
    println!();
    println!("model    {}   ({})", cfg.model, cfg.model_source);
    println!("shell    {shell}   ({shell_source})");
    println!(
        "api key  {}   ({})",
        if cfg.api_key.is_some() { "set" } else { "not set" },
        cfg.api_key_source
    );
    if just_created {
        println!();
        println!("created the config file with commented defaults — edit it to change settings");
    }
    Ok(0)
}
