# howto

Ask your terminal in plain words — get a runnable command, pre-typed at your prompt,
explained, and never executed for you. It knows your OS, shell, and installed tools.

**[Website](https://c0ldsmi1e.github.io/howto/)** · [Design spec](SPEC.md) · [Releases](https://github.com/C0ldSmi1e/howto/releases/latest)

```
$ howto kill whatever is using port 3000
lsof -ti:3000 | xargs kill -9
  Find the process listening on port 3000 and force-kill it.
  ⚠ SIGKILL gives the process no chance to clean up.
  [2] Use SIGTERM to let the process shut down cleanly.
kill $(lsof -ti:3000)
$ lsof -ti:3000 | xargs kill -9▌   ← Enter runs it, edit it, or Ctrl-C to discard
```

## How it works

- **Nothing ever runs on its own.** howto only suggests. With the shell wrapper
  installed, the answer lands pre-typed at your own prompt — Enter runs it, your
  line editor edits it, Ctrl-C throws it away.
- **Answers fit this machine.** The prompt includes your OS, shell, installed
  binaries, and project markers, so "kill port 3000" uses `lsof` on macOS and
  `fuser` where that's what exists.
- **Well-behaved in pipes.** stdout carries only the command; explanations,
  warnings, and alternatives go to stderr. `$(howto ...)` just works.

## Install

```sh
curl -LsSf https://github.com/C0ldSmi1e/howto/releases/latest/download/howto-installer.sh | sh
```

Prebuilt for macOS and Linux. With Rust installed: `cargo install --path .` from a checkout.

Then set your API key (get one at [platform.claude.com](https://platform.claude.com)):

```sh
export ANTHROPIC_API_KEY=sk-ant-...
```

And add the wrapper — the part that puts commands at your prompt. Without it,
howto prints the command instead of typing it for you:

```sh
# ~/.zshrc   (bash: --init bash)
eval "$(howto --init zsh)"
```

## Usage

No quotes, no subcommands — everything after `howto` is your question.

```sh
howto compress this folder into a tar.gz
howto 2                                   # use option [2] from the last answer
howto --last                              # show the last answer again
howto --explain find files changed today  # part-by-part breakdown
```

| Flag | |
|---|---|
| `-p, --print` | bare command on stdout only |
| `-j, --json` | structured JSON result |
| `--explain` | part-by-part breakdown |
| `--last` | re-show the last answer (no API call) |
| `--init <shell>` | print the shell wrapper (zsh, bash) |
| `--config` | config path and resolved settings |
| `-v, --verbose` | context, model, latency on stderr |

## Scripts & agents

howto never blocks on input, so there is nothing to hang on. stdout is exactly
one command (`-p`) or one JSON object (`-j`); exit codes are an API:
`0` suggestion produced · `1` error · `2` no sensible command.

```sh
howto -p free up disk space
howto -j convert a.png to webp   # {command, explanation, danger, alternatives, ...}
```

## Safety

- Nothing runs until you press Enter at your own prompt. There is no `-y`, on purpose.
- Risky commands show a `⚠ reason`. A local blocklist catches `rm -rf /`-class
  commands, `dd` to raw devices, fork bombs, and `curl | sh` — regardless of
  what the model says.
- High-danger commands are never pre-typed — they appear numbered, and
  `howto 1` is the deliberate act that loads one.

## Config

`~/.config/howto/config.toml`, created with commented defaults on first run:

```toml
# api_key = ""                 # prefer the ANTHROPIC_API_KEY env var
# model = "claude-haiku-4-5"
# shell = ""                   # zsh, bash (default: detected from $SHELL)
```

Precedence: flags > environment variables > config file > defaults.

## Building from source

```sh
cargo build --release
cargo test
```

## License

MIT
