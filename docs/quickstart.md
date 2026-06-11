# Quick Start

Get codexctl running in under two minutes.

## 1. Install

```bash
brew install aleadag/tap/codexctl     # Homebrew — ships with bus/coord/relay/hive built in
# or
cargo install codexctl                                          # Cargo — default features only (hive)
cargo install codexctl --features bus,coord,relay,hive          # Cargo with all features
```

Verify it works:

```bash
codexctl --version
```

## 2. Onboard with `codexctl init`

```bash
codexctl init
```

The wizard walks five phases — weekly budget cap, local-LLM brain auto-detection (probes ollama / llama.cpp / LM Studio / vLLM), Codex hook install, agent-bus role binding, and curated skill suggestions. Each phase is skippable (`s` at the prompt) and the result is recorded at `~/.codexctl/onboarding.json` so later runs of `codexctl init --check` can show drift.

For dotfile automation:

```bash
codexctl init --non-interactive --budget 25 --skip-brain --skip-bus --skip-skills
```

If you only want the hook install (the previous `--init` flag), that's the **Plugin** phase — accept it and skip the others.

Your existing Codex hook config is preserved; the hook install only adds codexctl entries.

**What gets added:** the Plugin phase writes hooks in two places — into `~/.codex/hooks.json` (the dashboard-observability hooks) and into the embedded plugin at `~/.codex/plugins/codexctl/hooks/hooks.json` (the bus + brain plugin hooks). Both sets coexist; Codex merges them.

**`~/.codex/hooks.json` (dashboard observability):**

| Hook | Matcher | Command | What it does |
|------|---------|---------|--------------|
| `PreToolUse` | `Bash` | `codexctl --json 2>/dev/null \|\| true` | Lets codexctl see Bash commands before they run |
| `PostToolUse` | `*` | `codexctl --json 2>/dev/null \|\| true` | Notifies codexctl after every tool completion |
| `Stop` | (all) | `codexctl --json 2>/dev/null \|\| true` | Notifies codexctl when a turn ends |

These are fire-and-forget snapshot reads. `|| true` keeps Codex unblockable if codexctl isn't installed or fails.

**`~/.codex/plugins/codexctl/hooks/hooks.json` (bus + brain plugin):**

| Hook | Matcher | Script | What it does |
|------|---------|--------|--------------|
| `PreToolUse` | `Bash\|Write\|Edit\|NotebookEdit` | `brain-gate.sh` | Queries the local LLM for approve/deny on potentially destructive tool calls |
| `PostToolUse` | `Bash\|Write\|Edit\|NotebookEdit` | `outcome-record.sh` | Records the outcome so the brain learns from your corrections |
| `SessionStart` | (all) | `session-briefing.sh` | Surfaces queued mail and recent context at session start |
| `Stop` | (all) | `inbox-drain.sh` | Drains the agent's bus mailbox; can return `decision:"block"` with `additionalContext` to deliver mail in the same turn (Trigger A in [docs/AGENT_BUS.md](AGENT_BUS.md#6-notification--delivery-handshake)) |

Both sets are removed cleanly by `codexctl init --remove`.

## 3. Verify the install

```bash
codexctl doctor
```

This runs a top-down checklist: PATH, hooks, plugin files, brain endpoint, bus feature, bus DB, session discovery, terminal integration. Green means you're ready. If anything fails, the doctor names the exact command to fix it.

## 4. Start the dashboard

Open one or more Codex sessions in separate terminals, then:

```bash
codexctl
```

You'll see every session in a live table with status, cost, context usage, burn rate, and more. (Forgot step 2 + 3? On first run you'll see a banner pointing you back to `codexctl init`.)

## 5. Try demo mode (no Codex needed)

```bash
codexctl --demo
```

Runs with fake sessions so you can explore the dashboard, keybindings, and features without any live sessions. Press `R` on any session to record a highlight reel — demo mode drip-feeds a scripted coding session (reading files, writing code, fixing errors, running tests) so you can see the session recorder in action.

## Key actions from the dashboard

| Key | Action |
|-----|--------|
| `j`/`k` | Navigate sessions |
| `Enter` | Expand session detail |
| `Tab` | Jump to session's terminal |
| `y` | Approve a blocked prompt |
| `i` | Send input to a session |
| `n` | Launch a new session |
| `?` | Show all keybindings |

## Optional: submit a task to the supervisor

The supervisor turns the durable coord ledger into a task runner: submit work, declare verifiers, let the reconciler hand it to a role's mailbox (or spawn a fresh session). It survives daemon restarts.

```bash
# Inline submission — useful for one-shot scripts
codexctl supervisor submit \
  --name "rename-utils" \
  --cwd "$PWD" \
  --prompt "Rename utils.rs → helpers.rs and update every import" \
  --role backend

# Inspect what's running
codexctl supervisor status            # compact table
codexctl supervisor logs <task_id>    # transitions + verifier history
```

Batch from a `tasks.toml` file (RFC §4 shape) with `codexctl supervisor run tasks.toml --dry-run` to preview, then without `--dry-run` to commit. See the [README's Supervisor section](../README.md#supervisor) for the verifier syntax (`run` / `brain` / `agent`) and the full design overview.

`codexctl supervisor drain` halts new assignments without killing running tasks; the `supervisor drain` row in `codexctl doctor` surfaces the state.

## Optional: project-scoped hooks

If you only want codexctl hooks in specific projects (not globally), the `--init` legacy flag still works for hook-only installs:

```bash
codexctl --init -s project
```

This writes to `.codex/hooks.json` (gitignored) instead of the global file. The `-s project` flag matches Codex's own `--scope` convention. `--init` is otherwise deprecated — prefer `codexctl init` for new setups.

## Optional: add the local LLM brain

The brain auto-approves safe operations and blocks dangerous ones using a local model:

```bash
ollama pull gemma4:e4b && ollama serve       # One-time setup
codexctl --brain                            # Start with brain enabled
```

### Toggle the brain mid-session

```bash
codexctl --mode off                         # Pause brain (manual approvals)
codexctl --mode on                          # Resume brain (default)
codexctl --mode auto                        # Brain handles everything
codexctl --mode status                      # Show current mode
```

If you use the Codex plugin, type `/brain off` or `/brain auto` directly in your session.

### Auto-insights

Enable the brain to automatically detect friction patterns and suggest workflow improvements:

```bash
codexctl --brain --insights on            # Enable auto-generation
codexctl --brain --insights               # View current insights
```

## Optional: install the Codex plugin

The `codex-plugin/` directory in the codexctl repo is a Codex plugin that integrates the brain directly into your sessions, no TUI required:

- `/sessions` — see all active sessions
- `/spend` — cost breakdown
- `/brain on|off|auto` — toggle brain mid-session
- `/auto-insights` — view or configure auto-generated workflow insights
- `/inbox` — drain pending agent-bus messages addressed to this session's role
- `/role <name>` — set this session's agent-bus role, e.g. `/role frontend` or `/role tester` (auto-detects Codex's pid)
- **Automatic brain gate** — the plugin hook queries the brain before every Bash/Write/Edit call

The plugin and `--init` hooks are complementary. Use `--init` for dashboard observability, the plugin for inline brain decisions.

## Upgrading

After `brew upgrade codexctl` (or `cargo install codexctl --force --locked`), the new binary is on disk but the hook entries, plugin files, and DB schema were written by the old binary. Refresh them with:

```bash
codexctl init --upgrade
```

The command walks four steps and reports each: (1) re-write Codex hook entries, (2) re-write embedded plugin files from the new binary, (3) run any pending bus / coord DB migrations, (4) bump the onboarding marker's recorded version. It's safe to run any time — files that haven't changed are reported "unchanged."

`codexctl doctor` has a `plugin version` row that flags this scenario: it compares the binary's version against the on-disk `.codex-plugin/plugin.json` and surfaces an advisory with the upgrade command when they differ.

## Uninstall

Roll back the onboarding wizard's installed artifacts:

```bash
codexctl init --remove                      # Soft uninstall: hooks + onboarding marker
codexctl init --purge --yes                 # Hard uninstall: --remove + nuke ~/.codexctl/ + config
```

`--remove` is the safe form — strips Codex hooks and the onboarding marker, but preserves user data (bus DB roles, brain decision logs, hive knowledge, relay identity, your budget config line). Use this when you want to stop the integration without losing what codexctl has learned.

`--purge` is the hard reset — `--remove` plus `~/.codexctl/` (all subdirs) plus `~/.config/codexctl/config.toml`. Use this when reinstalling fresh, recovering from corrupted state, or fully uninstalling. Pair with `--yes` to skip the confirmation prompt; without it you'll see a list of paths and have to confirm.

Or remove just the hooks (legacy flag, still supported):

```bash
codexctl --uninstall                        # Remove from user settings
codexctl --uninstall -s project             # Remove from project settings
```

Both `--remove` and `--uninstall` surgically remove only codexctl entries. All other settings and hooks are preserved.

To uninstall the binary:

```bash
brew uninstall codexctl                     # Homebrew
# or
cargo uninstall codexctl                    # Cargo
```

## Next steps

- [Reference](reference.md) -- dashboard features, keybindings, all CLI flags
- [Configuration](configuration.md) -- TOML config, hooks, rules, model pricing
- [Relay & Hive Mind](relay.md) -- hive knowledge is built-in; add `--features relay` for cross-machine networking
- [Terminal Support](terminal-support.md) -- compatibility and setup notes
- [Troubleshooting](troubleshooting.md) -- common issues and FAQ
