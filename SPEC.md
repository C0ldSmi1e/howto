# howto — design spec

Ask your terminal how to do things. `howto kill whatever is using port 3000` answers with a
runnable command — pre-typed at your prompt when the shell wrapper is installed, printed
plainly when piped, structured JSON for agents. Nothing ever runs on its own.

```
$ howto kill whatever is using port 3000
  force-kills the process listening on port 3000
$ lsof -ti:3000 | xargs kill -9▌        ← pre-typed at the prompt; Enter runs, Ctrl-C discards
```

## Core principles

1. **The binary never executes anything and never blocks on input.** It is a pure
   "question in, command out" program. The shell does all interaction: editing via your own
   line editor, running via Enter at your own prompt, canceling via Ctrl-C.
2. **Stream discipline.** stdout carries *only* a command (or JSON with `-j`). Explanations,
   warnings, alternatives, hints — all stderr. This makes `$(howto ...)`, pipes, and the
   shell wrapper work by construction.
3. **Context-aware.** The prompt includes this machine's OS, shell, installed binaries, and
   cwd project markers, so `kill port 3000` answers with `lsof` on macOS and `fuser` where
   that's what exists.
4. **Copy-hygiene.** A line containing a command contains nothing but the command, at
   column 0, never hard-wrapped or truncated. Metadata lives on adjacent dimmed lines.
5. **Agent-friendly by construction.** No interactivity means nothing to hang on; `-p`/`-j`
   give one-shot output; exit codes are an API.

## Modes (picked automatically, overridable)

| Context | Behavior |
|---|---|
| Wrapped (`eval "$(howto --init zsh)"` installed) | full answer on stderr (command echoed first, explanation beneath) → command also lands pre-typed at the prompt via `print -z`, so scrollback stays complete after the injected line is edited or run |
| Unwrapped, stdout is a TTY | everything rendered numbered; bare command on stdout (bold); one-time hint to install the wrapper |
| Piped / `-p` | bare command on stdout, nothing else |
| `-j` | full structured JSON on stdout |

The wrapper sets `__HOWTO_WRAP=1` and captures stdout via `$(...)`; flags and empty input
pass through to the raw binary (`case "$1" in ""|-*)`), so `--last`, `--help` etc. behave
identically wrapped or not.

## CLI surface (v1 — final)

| Surface | Role |
|---|---|
| `howto <free text>` | the product; no quotes needed, hyphens inside the query pass through |
| `howto <digit>` | recall command [N] from the last response (no API call) |
| `--last` | re-show the last response, every command numbered (no API call) |
| `-p, --print` | bare command on stdout, no frills |
| `-j, --json` | structured output: `{command, explanation, danger, danger_reason, alternatives, breakdown?}` |
| `--explain` | ask for a part-by-part breakdown too |
| `--init <shell>` | emit the shell wrapper (zsh, bash; fish planned) |
| `--config` | show config path + resolved settings (creates a commented template on first run) |
| `-v, --verbose` | debug on stderr: detected context, model, latency |
| `-h` / `-V` | help (written for agents too) / version |

**No subcommands** — the query is free text, so any word could start a query. Everything
non-query is a flag. Flags come before the query; the first non-flag token starts the query
and everything after passes through verbatim.

## Numbered recall

A number labels anything **not already sitting at your prompt**: alternatives, danger-blocked
commands, everything in `--last` and unwrapped mode. `howto N` re-emits that command (injected
when wrapped). This removes copy-paste entirely; the state behind it is one file,
`~/.local/state/howto/last.json` (honors `$XDG_STATE_HOME`).

## Danger policy

- The model classifies `danger`: `none | low | medium | high` with a one-line reason.
- A local regex blocklist (`rm -rf /`~-class, `dd of=/dev/`, `mkfs`, fork bombs, `curl|sh`,
  raw-device writes) escalates to `high` regardless of what the model said. Escalate-only.
- `medium`+ renders a colored `⚠ reason` line.
- **`high` is never injected.** Wrapped mode prints it numbered instead; typing `howto 1` is
  the deliberate confirmation that loads it. `-p`/`-j` still emit it (explicit request);
  the warning goes to stderr / the `danger` field.

## Agent contract

- stdout = exactly one command (`-p`) or one JSON object (`-j`). Everything else stderr.
- Exit codes: `0` suggestion produced · `1` error (network, config, usage) · `2` model
  could not help (`-j` still emits the JSON with `cannot_help` populated).
- Never blocks: there is no interactive code path at all.
- `--help` documents `-p`/`-j` usage with examples — it doubles as agent docs.

## Claude API usage

- Endpoint: `POST https://api.anthropic.com/v1/messages`; headers `x-api-key`,
  `anthropic-version: 2023-06-01`. Blocking HTTP via `ureq` (no async runtime).
- Model: `claude-haiku-4-5` by default (fast + cheap; a one-line answer doesn't need more).
  Override precedence: `HOWTO_MODEL` env > `model` in config > default.
- **Forced tool use for structured output**: one tool `suggest_command` with
  `tool_choice: {type: "tool", name: "suggest_command"}` — the response is tool-input JSON,
  never prose. Only `command`/`explanation`/`danger` are required; inapplicable fields
  (`danger_reason`, `cannot_help`, `breakdown`) are omitted, not empty strings — requiring
  empty-string placeholders made the model leak tool-syntax garbage into them (found live).
- One call returns command + explanation + up to 2 alternatives (+ breakdown with
  `--explain`, + `cannot_help` when refusing). `max_tokens: 1024`.
- API key: `ANTHROPIC_API_KEY` env > `api_key` in config. Missing → friendly error, exit 1.

## Config & state

- Config: `~/.config/howto/config.toml` (honors `$XDG_CONFIG_HOME`), auto-created with
  commented defaults on first run — the file documents itself. Keys: `api_key`, `model`,
  `shell` (override detection).
- Precedence, strictly: flags > env vars > config file > defaults.
- State: `~/.local/state/howto/last.json` (last response for recall/`--last`) plus a
  `hint-shown` marker for the one-time wrapper hint.

## Module map

```
src/
  main.rs      dispatch: --init / --config / --last / digit recall / fresh query; exit codes
  cli.rs       clap definition (trailing_var_arg free-text query) + agent-oriented --help
  config.rs    XDG paths, template creation, env>file>default resolution
  context.rs   OS/arch/shell detection, PATH binary probe, cwd markers → prompt block
  api.rs       Messages API call: system prompt, strict tool schema, error mapping
  response.rs  Suggestion/Alternative/Breakdown/Danger types (serde, shared everywhere)
  danger.rs    local escalation regexes + tests
  render.rs    mode resolution, ANSI styling, wrapped/tty/menu/recall renderers
  state.rs     last.json save/load, hint marker
  init.rs      zsh/bash wrapper source
```

## Deferred (by design, not omission)

- **fish wrapper** — `commandline` semantics differ; zsh/bash first.
- **Query cache** — cut for v1; `last.json` covers recall. Revisit if latency/cost bite.
- **`--copy` (OSC 52), `--set key=value`, `-y/--run`** — cut in design review.
- **Picker UI** — deliberately never: editing the injected line beats menus.
- Distribution: `cargo-dist` → GitHub releases + Homebrew tap; check crates.io name.
