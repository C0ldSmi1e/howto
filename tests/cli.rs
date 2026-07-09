//! End-to-end tests against the compiled binary.
//!
//! Offline tests run hermetically: config and state point at per-test temp dirs
//! and `ANTHROPIC_API_KEY` is scrubbed, so nothing touches the network or the
//! user's real files. Live tests (`#[ignore]`) use the real config so the key
//! resolves however the user set it up: `cargo test -- --ignored`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_howto");

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct TestEnv {
    root: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("howto-test-{}-{n}", std::process::id()));
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("state")).unwrap();
        TestEnv { root }
    }

    fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    fn state_dir(&self) -> PathBuf {
        self.root.join("state")
    }

    /// Hermetic command: temp config + state, no API key, no wrapper env.
    fn cmd(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env_remove("ANTHROPIC_API_KEY")
            .env_remove("HOWTO_MODEL")
            .env_remove("__HOWTO_WRAP")
            .env("XDG_CONFIG_HOME", self.config_dir())
            .env("XDG_STATE_HOME", self.state_dir());
        c
    }

    fn seed_last(&self, json: &str) {
        let dir = self.state_dir().join("howto");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("last.json"), json).unwrap();
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run(cmd: &mut Command, args: &[&str]) -> Output {
    cmd.args(args).output().expect("binary should run")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn code(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

const SEED: &str = r#"{
  "query": "kill whatever is using port 3000",
  "suggestion": {
    "command": "lsof -ti:3000 | xargs kill -9",
    "explanation": "force-kills the process listening on port 3000",
    "danger": "medium",
    "danger_reason": "SIGKILL gives the process no chance to clean up",
    "alternatives": [
      { "command": "kill $(lsof -ti:3000)", "note": "graceful SIGTERM first" }
    ]
  },
  "saved_at": 1770000000
}"#;

const SEED_HIGH: &str = r#"{
  "query": "zero out disk9",
  "suggestion": {
    "command": "dd if=/dev/zero of=/dev/disk9",
    "explanation": "overwrites disk9 with zeros",
    "danger": "high",
    "danger_reason": "irreversibly destroys everything on the disk",
    "alternatives": []
  },
  "saved_at": 1770000000
}"#;

// ---- surface & docs -------------------------------------------------------

#[test]
fn help_documents_the_agent_contract() {
    let env = TestEnv::new();
    let out = run(&mut env.cmd(), &["--help"]);
    assert_eq!(code(&out), 0);
    let text = stdout(&out);
    assert!(text.contains("--json"), "help should document -j");
    assert!(text.contains("Exit codes"), "help should document exit codes");
    assert!(
        text.contains("stdout carries only a command"),
        "help should state the stream contract"
    );
}

#[test]
fn version_prints_and_exits_zero() {
    let env = TestEnv::new();
    let out = run(&mut env.cmd(), &["--version"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).starts_with("howto "));
}

// ---- wrappers ---------------------------------------------------------------

#[test]
fn init_emits_shell_wrappers() {
    let env = TestEnv::new();

    let zsh = run(&mut env.cmd(), &["--init", "zsh"]);
    assert_eq!(code(&zsh), 0);
    assert!(stdout(&zsh).contains("print -z"), "zsh wrapper injects via print -z");
    assert!(stdout(&zsh).contains("__HOWTO_WRAP=1"));
    assert!(stderr(&zsh).is_empty());

    let bash = run(&mut env.cmd(), &["--init", "bash"]);
    assert_eq!(code(&bash), 0);
    assert!(stdout(&bash).contains("read -r -e"), "bash wrapper pre-fills via read -e -i");
}

#[test]
fn init_fish_is_a_friendly_error() {
    let env = TestEnv::new();
    let out = run(&mut env.cmd(), &["--init", "fish"]);
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).is_empty());
    assert!(stderr(&out).contains("fish"));
}

// ---- config -----------------------------------------------------------------

#[test]
fn config_creates_the_template_exactly_once() {
    let env = TestEnv::new();

    let first = run(&mut env.cmd(), &["--config"]);
    assert_eq!(code(&first), 0);
    assert!(stdout(&first).contains("created the config file"));
    assert!(stdout(&first).contains("claude-haiku-4-5"), "shows the default model");

    let template = fs::read_to_string(env.config_dir().join("howto/config.toml")).unwrap();
    assert!(template.contains("# api_key"), "template keys ship commented out");

    let second = run(&mut env.cmd(), &["--config"]);
    assert!(!stdout(&second).contains("created"), "second run must not re-create");
}

#[test]
fn config_file_settings_are_resolved_and_attributed() {
    let env = TestEnv::new();
    let dir = env.config_dir().join("howto");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("config.toml"), "model = \"claude-sonnet-5\"\nshell = \"fish\"\n").unwrap();

    let out = run(&mut env.cmd(), &["--config"]);
    let text = stdout(&out);
    assert!(text.contains("claude-sonnet-5"));
    assert!(text.contains("fish"));
    assert!(text.contains("(config file)"), "sources are attributed");
}

// ---- error paths (no network, no key) ----------------------------------------

#[test]
fn query_without_api_key_fails_cleanly() {
    let env = TestEnv::new();
    let out = run(&mut env.cmd(), &["-p", "list", "files"]);
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).is_empty(), "stdout stays clean on errors");
    assert!(stderr(&out).contains("no API key"));
}

#[test]
fn empty_query_prints_usage() {
    let env = TestEnv::new();
    let out = run(&mut env.cmd(), &[]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("usage:"));
}

#[test]
fn typoed_flag_is_rejected_without_an_api_call() {
    let env = TestEnv::new();
    // No key is set, but the guard must fire before key resolution is even relevant.
    let out = run(&mut env.cmd(), &["--lst"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("unknown option '--lst'"));
}

#[test]
fn recall_without_state_is_a_friendly_error() {
    let env = TestEnv::new();
    let out = run(&mut env.cmd(), &["2"]);
    assert_eq!(code(&out), 1);
    assert!(stderr(&out).contains("no previous response"));

    let last = run(&mut env.cmd(), &["--last"]);
    assert_eq!(code(&last), 1);
}

// ---- recall & --last ----------------------------------------------------------

#[test]
fn recall_prints_the_bare_alternative_when_piped() {
    let env = TestEnv::new();
    env.seed_last(SEED);
    let out = run(&mut env.cmd(), &["2"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "kill $(lsof -ti:3000)\n");
    assert!(stderr(&out).is_empty(), "piped mode is silent");
}

#[test]
fn recall_out_of_range_names_the_valid_range() {
    let env = TestEnv::new();
    env.seed_last(SEED);
    for bad in ["0", "5"] {
        let out = run(&mut env.cmd(), &[bad]);
        assert_eq!(code(&out), 1, "howto {bad}");
        assert!(stderr(&out).contains("pick 1 to 2"));
    }
}

#[test]
fn recall_as_json_carries_the_warning() {
    let env = TestEnv::new();
    env.seed_last(SEED);

    let one = run(&mut env.cmd(), &["-j", "1"]);
    let v: serde_json::Value = serde_json::from_str(&stdout(&one)).unwrap();
    assert_eq!(v["command"], "lsof -ti:3000 | xargs kill -9");
    assert!(v["warning"].as_str().unwrap().contains("SIGKILL"));

    let two = run(&mut env.cmd(), &["-j", "2"]);
    let v: serde_json::Value = serde_json::from_str(&stdout(&two)).unwrap();
    assert_eq!(v["command"], "kill $(lsof -ti:3000)");
    assert!(v.get("warning").is_none(), "benign alternative has no warning");
}

#[test]
fn last_piped_prints_only_the_primary_command() {
    let env = TestEnv::new();
    env.seed_last(SEED);
    let out = run(&mut env.cmd(), &["--last"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "lsof -ti:3000 | xargs kill -9\n");
    assert!(stderr(&out).is_empty());
}

#[test]
fn last_json_returns_the_stored_record_verbatim() {
    let env = TestEnv::new();
    env.seed_last(SEED);
    let out = run(&mut env.cmd(), &["--last", "-j"]);
    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v["query"], "kill whatever is using port 3000");
    assert_eq!(v["saved_at"], 1770000000);
    assert_eq!(v["suggestion"]["danger"], "medium");
    assert_eq!(v["suggestion"]["alternatives"][0]["note"], "graceful SIGTERM first");
}

// ---- stream discipline (the wrapper depends on this) ----------------------------

#[test]
fn wrapped_recall_splits_command_and_warning_across_streams() {
    let env = TestEnv::new();
    env.seed_last(SEED);
    let mut cmd = env.cmd();
    cmd.env("__HOWTO_WRAP", "1");
    let out = run(&mut cmd, &["1"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "lsof -ti:3000 | xargs kill -9\n", "stdout is the injectable command only");
    assert!(stderr(&out).contains("⚠"), "warning rides on stderr");
    assert!(stderr(&out).contains("SIGKILL"));
}

#[test]
fn recalling_a_high_danger_command_is_deliberate_and_injects() {
    let env = TestEnv::new();
    env.seed_last(SEED_HIGH);
    let mut cmd = env.cmd();
    cmd.env("__HOWTO_WRAP", "1");
    let out = run(&mut cmd, &["1"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "dd if=/dev/zero of=/dev/disk9\n");
    assert!(stderr(&out).contains("irreversibly destroys"));
}

// ---- live API tests (opt-in: cargo test -- --ignored) ---------------------------
//
// These use the real config dir so the API key resolves however the user set it
// up (env var or config file); state still goes to a temp dir. They skip with a
// note when no key is available.

fn live_cmd(env: &TestEnv) -> Command {
    let mut c = Command::new(BIN);
    c.env("XDG_STATE_HOME", env.state_dir());
    c
}

fn skip_if_no_key(out: &Output) -> bool {
    if code(out) == 1 && stderr(out).contains("no API key") {
        eprintln!("skipping live test: no API key available");
        return true;
    }
    false
}

#[test]
#[ignore = "live API call — run with: cargo test -- --ignored"]
fn live_print_returns_exactly_one_command_and_saves_state() {
    let env = TestEnv::new();
    let out = run(
        &mut live_cmd(&env),
        &["-p", "show", "the", "5", "largest", "files", "in", "this", "directory"],
    );
    if skip_if_no_key(&out) {
        return;
    }
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.trim().is_empty());
    assert_eq!(
        text.trim_end().lines().count(),
        1,
        "stdout must be exactly one command line, got: {text:?}"
    );

    let saved = fs::read_to_string(env.state_dir().join("howto/last.json")).unwrap();
    assert!(saved.contains("\"command\""), "response is saved for recall");

    let recalled = run(&mut live_cmd(&env), &["-p", "1"]);
    assert_eq!(stdout(&recalled), text, "recall [1] reproduces the command without an API call");
}

#[test]
#[ignore = "live API call — run with: cargo test -- --ignored"]
fn live_json_matches_the_documented_contract() {
    let env = TestEnv::new();
    let out = run(
        &mut live_cmd(&env),
        &["-j", "--explain", "compress", "the", "src", "folder", "into", "a", "tar.gz"],
    );
    if skip_if_no_key(&out) {
        return;
    }
    assert_eq!(code(&out), 0, "stderr: {}", stderr(&out));

    let v: serde_json::Value = serde_json::from_str(&stdout(&out)).expect("stdout is valid JSON");
    assert!(!v["command"].as_str().unwrap().is_empty());
    assert!(!v["explanation"].as_str().unwrap().is_empty());
    assert!(
        ["none", "low", "medium", "high"].contains(&v["danger"].as_str().unwrap()),
        "danger is one of the documented levels"
    );
    assert!(v["alternatives"].is_array());
    assert!(
        v["breakdown"].as_array().map(|b| !b.is_empty()).unwrap_or(false),
        "--explain should produce a non-empty breakdown, got: {v}"
    );
}
