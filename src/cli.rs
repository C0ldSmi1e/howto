use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "howto",
    version,
    about = "Ask in plain words, get a shell command typed at your prompt",
    after_help = AFTER_HELP
)]
pub struct Cli {
    /// Print the bare command to stdout and exit (no extras)
    #[arg(short, long)]
    pub print: bool,

    /// Print the full result as JSON (for scripts and agents)
    #[arg(short, long)]
    pub json: bool,

    /// Include a part-by-part breakdown of the command
    #[arg(long)]
    pub explain: bool,

    /// Show the last response again, every command numbered
    #[arg(long)]
    pub last: bool,

    /// Print the shell integration wrapper for eval (zsh or bash)
    #[arg(long, value_name = "SHELL")]
    pub init: Option<String>,

    /// Show the config file path and resolved settings
    #[arg(long)]
    pub config: bool,

    /// Debug detail on stderr: detected context, model, timing
    #[arg(short, long)]
    pub verbose: bool,

    /// What you want to do, in plain words (no quotes needed).
    /// A single number recalls that command from the last response.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "QUERY")]
    pub query: Vec<String>,
}

const AFTER_HELP: &str = "\
Examples:
  howto kill whatever is using port 3000    suggest a command (pre-typed at your prompt if the wrapper is installed)
  howto -p free up disk space               bare command on stdout only (for scripts)
  howto -j convert a.png to webp            JSON: {command, explanation, danger, alternatives, ...}
  howto --explain tar this folder           include a part-by-part breakdown
  howto 2                                   use command [2] from the last response
  howto --last                              show the last response again

Setup:
  export ANTHROPIC_API_KEY=...              or set api_key via `howto --config`
  eval \"$(howto --init zsh)\"                in ~/.zshrc — commands land pre-typed at your prompt

Contract (for scripts and agents):
  stdout carries only a command (or JSON with -j); everything else goes to stderr.
  Exit codes: 0 = suggestion produced, 1 = error, 2 = the model could not help.";
