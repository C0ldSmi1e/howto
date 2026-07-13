# howto

Ask your terminal in plain words — get a runnable command, pre-typed at your prompt.

```
$ howto kill whatever is using port 3000
lsof -ti:3000 | xargs kill -9
  Find the process ID listening on port 3000 and force-kill it.
  ⚠ SIGKILL gives the process no chance to clean up.
  [2] Use SIGTERM to let the process shut down cleanly.
kill $(lsof -ti:3000)
$ lsof -ti:3000 | xargs kill -9▌   ← pre-typed: Enter runs it, edit it, or Ctrl-C to discard
```

howto knows your machine — OS, shell, which tools are actually installed, what project
you're standing in — so the answer fits *your* system. It never runs anything on its own:
the command lands at your prompt and you decide.

## Install

```sh
curl -LsSf https://github.com/C0ldSmi1e/howto/releases/latest/download/howto-installer.sh | sh
```

(Prebuilt for macOS and Linux, both arches. Yes — howto itself flags `curl | sh` as
high-danger; read the script first, that's the point. With Rust installed you can
`cargo install --path .` from a checkout instead.)

You'll need an [Anthropic API key](https://platform.claude.com/):

```sh
export ANTHROPIC_API_KEY=sk-ant-...           # or: api_key in `howto --config`
```

Add the shell integration (this is what puts commands at your prompt):

```sh
# ~/.zshrc                                    # ~/.bashrc
eval "$(howto --init zsh)"                    eval "$(howto --init bash)"
```

Without it, howto still works — it prints the command instead of typing it for you.
fish support is planned.

## Use

```sh
howto compress this folder into a tar.gz      # no quotes needed
howto 2                                       # use option [2] from the last answer
howto --last                                  # show the last answer again
howto --explain find files changed today      # include a part-by-part breakdown
```

| Flag | |
|---|---|
| `-p, --print` | bare command on stdout, nothing else |
| `-j, --json` | full structured result |
| `--explain` | part-by-part breakdown |
| `--last` | re-show the last answer, numbered (no API call) |
| `--init <shell>` | print the shell wrapper (`zsh`, `bash`) |
| `--config` | config path + resolved settings |
| `-v, --verbose` | detected context, model, latency on stderr |

## Scripts & agents

stdout carries only a command (or JSON with `-j`); everything else goes to stderr —
so `$(howto ...)` and pipes always capture exactly the command.

```sh
howto -p free up disk space                   # → one bare command line
howto -j convert a.png to webp                # → {"command": ..., "explanation": ...,
                                              #    "danger": ..., "alternatives": [...]}
```

Exit codes: `0` suggestion produced · `1` error · `2` no sensible command exists.
There is no interactive code path, so nothing ever blocks waiting for input.

## Safety

- Nothing executes without you pressing Enter at your own prompt.
- Every answer carries a danger rating; risky commands render a `⚠ reason` line.
- A local blocklist (`rm -rf /`-class, `dd of=/dev/…`, `mkfs`, `curl | sh`, fork bombs)
  escalates to **high** regardless of what the model said.
- High-danger commands are **never pre-typed** — they're shown numbered, and you load
  one deliberately with `howto 1`.

## Configuration

`~/.config/howto/config.toml` (created with commented defaults on first run):

```toml
# api_key = ""                 # prefer the ANTHROPIC_API_KEY env var
# model = "claude-haiku-4-5"   # env override: HOWTO_MODEL
# shell = ""                   # override shell detection: zsh, bash, fish
```

Precedence: environment variables > config file > defaults.
State (the last answer, for `howto <n>` recall) lives in `~/.local/state/howto/`.
Both honor `$XDG_CONFIG_HOME` / `$XDG_STATE_HOME`.

## How it works

One blocking call to the Claude API (`claude-haiku-4-5` by default — fast and cheap;
a query costs a fraction of a cent and ~1–2 s). The machine context goes into the
prompt; a forced tool call returns structured JSON: command, explanation, danger
rating, and up to two genuinely different alternatives. Design details in
[SPEC.md](SPEC.md).

## License

MIT
