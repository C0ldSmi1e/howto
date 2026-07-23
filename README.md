# howto

Ask your terminal in plain words — get a runnable command, pre-typed at your prompt.
It knows your OS, shell, and installed tools, and it never runs anything on its own.

```
$ howto kill whatever is using port 3000
lsof -ti:3000 | xargs kill -9
  Find the process ID listening on port 3000 and force-kill it.
  ⚠ SIGKILL gives the process no chance to clean up.
  [2] Use SIGTERM to let the process shut down cleanly.
kill $(lsof -ti:3000)
$ lsof -ti:3000 | xargs kill -9▌   ← Enter to run, edit it, or Ctrl-C to discard
```

## Install

```sh
curl -LsSf https://github.com/C0ldSmi1e/howto/releases/latest/download/howto-installer.sh | sh
```

Prebuilt for macOS and Linux. With Rust installed: `cargo install --path .` from a checkout.

## Setup

```sh
export ANTHROPIC_API_KEY=sk-ant-...   # get one at platform.claude.com
```

```sh
# ~/.zshrc — puts commands at your prompt (bash: --init bash)
eval "$(howto --init zsh)"
```

Without the wrapper, howto prints the command instead of typing it for you.

## Usage

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

stdout carries only the command (or JSON with `-j`); everything else goes to stderr.
Exit codes: `0` ok · `1` error · `2` no sensible command. Never blocks on input.

```sh
howto -p free up disk space
howto -j convert a.png to webp
```

## Safety

- Nothing runs until you press Enter at your own prompt.
- Risky commands show a `⚠ reason`. A local blocklist catches `rm -rf /`-class
  commands regardless of what the model says.
- High-danger commands are never pre-typed — load one deliberately with `howto 1`.

## Config

`~/.config/howto/config.toml`:

```toml
# api_key = ""                 # prefer the ANTHROPIC_API_KEY env var
# model = "claude-haiku-4-5"
# shell = ""                   # zsh, bash, fish (default: $SHELL)
```

Design details: [SPEC.md](SPEC.md) · MIT license
