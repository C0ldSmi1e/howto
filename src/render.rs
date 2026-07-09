use crate::response::{Danger, Suggestion};
use crate::state::LastResponse;
use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Structured JSON on stdout (`-j`).
    Json,
    /// Bare command on stdout, nothing else (`-p`, or stdout is a pipe).
    Print,
    /// Called from the shell wrapper: prose on stderr, command on captured stdout.
    Wrapped,
    /// Human at a terminal without the wrapper: everything numbered, plus install hint.
    Tty,
}

pub fn resolve(json: bool, print: bool) -> Mode {
    if json {
        Mode::Json
    } else if print {
        Mode::Print
    } else if std::env::var("__HOWTO_WRAP").map(|v| v == "1").unwrap_or(false) {
        Mode::Wrapped
    } else if std::io::stdout().is_terminal() {
        Mode::Tty
    } else {
        Mode::Print
    }
}

pub struct Style {
    on: bool,
}

impl Style {
    pub fn stderr() -> Self {
        Style { on: color_ok(std::io::stderr().is_terminal()) }
    }

    pub fn stdout() -> Self {
        Style { on: color_ok(std::io::stdout().is_terminal()) }
    }

    fn wrap(&self, code: &str, s: &str) -> String {
        if self.on {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    pub fn dim(&self, s: &str) -> String {
        self.wrap("2", s)
    }
    pub fn bold(&self, s: &str) -> String {
        self.wrap("1", s)
    }
    pub fn red(&self, s: &str) -> String {
        self.wrap("31", s)
    }
    pub fn yellow(&self, s: &str) -> String {
        self.wrap("33", s)
    }
}

fn color_ok(is_terminal: bool) -> bool {
    is_terminal
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
}

pub fn suggestion(mode: Mode, s: &Suggestion) {
    match mode {
        Mode::Json => println!("{}", serde_json::to_string_pretty(s).expect("serializable")),
        Mode::Print => println!("{}", s.command),
        Mode::Wrapped => render_wrapped(s),
        Mode::Tty => render_tty(s),
    }
}

fn reason(s: &Suggestion) -> &str {
    if s.danger_reason.is_empty() {
        "flagged as risky"
    } else {
        &s.danger_reason
    }
}

fn print_danger(s: &Suggestion, err: &Style) {
    match s.danger {
        Danger::Medium => eprintln!("{}", err.yellow(&format!("  ⚠ {}", reason(s)))),
        Danger::High => eprintln!("{}", err.red(&format!("  ⚠ {}", reason(s)))),
        _ => {}
    }
}

fn print_breakdown(s: &Suggestion, err: &Style) {
    if s.breakdown.is_empty() {
        return;
    }
    let width = s.breakdown.iter().map(|p| p.part.chars().count()).max().unwrap_or(0);
    for part in &s.breakdown {
        eprintln!("    {:width$}  {}", part.part, err.dim(&part.meaning), width = width);
    }
}

fn print_alternatives(s: &Suggestion, err: &Style) {
    for (i, alt) in s.alternatives.iter().enumerate() {
        let label = format!("  [{}]", i + 2);
        if alt.note.is_empty() {
            eprintln!("{}", err.dim(&label));
        } else {
            eprintln!("{} {}", err.dim(&label), err.dim(&alt.note));
        }
        eprintln!("{}", alt.command);
    }
}

// The command is echoed into the answer as well as injected at the prompt:
// the injected line gets edited or executed away, and the scrollback should
// keep a complete record of what was suggested.
fn render_wrapped(s: &Suggestion) {
    let err = Style::stderr();
    let inject = s.danger < Danger::High;
    eprintln!("{}", err.bold(&s.command));
    eprintln!("{}", err.dim(&format!("  {}", s.explanation)));
    print_danger(s, &err);
    if !inject {
        eprintln!(
            "{}",
            err.red("  ⚠ not typed at your prompt — run `howto 1` to load it deliberately")
        );
    }
    print_breakdown(s, &err);
    print_alternatives(s, &err);
    if inject {
        println!("{}", s.command);
    }
}

fn render_tty(s: &Suggestion) {
    let err = Style::stderr();
    let out = Style::stdout();
    eprintln!("{} {}", err.dim("  [1]"), s.explanation);
    println!("{}", out.bold(&s.command));
    print_danger(s, &err);
    print_breakdown(s, &err);
    print_alternatives(s, &err);
    install_hint(&err);
}

pub fn menu(last: &LastResponse) {
    let err = Style::stderr();
    let s = &last.suggestion;
    eprintln!("{}", err.dim(&format!("  howto {}", last.query)));
    eprintln!("{} {}", err.dim("  [1]"), s.explanation);
    eprintln!("{}", s.command);
    print_danger(s, &err);
    print_breakdown(s, &err);
    print_alternatives(s, &err);
    eprintln!("{}", err.dim("  ↪ howto <number> to use one"));
}

pub fn recalled(mode: Mode, command: &str, note: &str, warning: Option<&str>) {
    let err = Style::stderr();
    match mode {
        Mode::Json => unreachable!("json recall handled by caller"),
        Mode::Print => println!("{command}"),
        Mode::Wrapped => {
            if let Some(w) = warning {
                eprintln!("{}", err.red(&format!("  ⚠ {w}")));
            }
            println!("{command}");
        }
        Mode::Tty => {
            if !note.is_empty() {
                eprintln!("{}", err.dim(&format!("  {note}")));
            }
            if let Some(w) = warning {
                eprintln!("{}", err.red(&format!("  ⚠ {w}")));
            }
            println!("{}", Style::stdout().bold(command));
        }
    }
}

fn install_hint(err: &Style) {
    if !crate::state::hint_pending() {
        return;
    }
    let (shell, _) = crate::context::shell_name(None);
    if shell != "zsh" && shell != "bash" {
        return;
    }
    eprintln!(
        "{}",
        err.dim(&format!(
            "  ↪ add  eval \"$(howto --init {shell})\"  to ~/.{shell}rc to get commands typed at your prompt"
        ))
    );
    crate::state::mark_hint_shown();
}
