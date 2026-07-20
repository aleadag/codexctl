# Troubleshooting

Start with:

```bash
coding-brain doctor
```

Doctor separates hard failures from advisories. Missing executables or unusable managed definitions fail the check; an unreachable optional local model, no active session, unverified Codex trust, or a remote endpoint is advisory.

## Hooks are missing or stale

```bash
coding-brain init --plugin-only
coding-brain doctor
```

Restart Codex and inspect `/hooks`. Coding Brain cannot observe whether Codex trusts a command, so trust remains advisory even when every definition is current.

Init removes only exact managed `coding-brain` entries and exact legacy managed hook commands. Lookalike and unrelated commands remain in place. The hook file is written to a complete sibling file, flushed, and atomically replaced; a failed pre-replace write leaves the original intact.

## Project identity is missing or malformed

Identity resolution first uses the project-root `.coding-brain/project.toml`, then a canonical network `origin`, and finally a path-derived temporary identity. A normal Git clone with a usable network origin therefore has stable identity without `coding-brain init`. Local paths and `file:` origins are not network origins, so they use the temporary fallback unless a manifest overrides them.

Use `coding-brain init` to create an explicit override when the origin is unusable or when you want to pin identity independently of the remote. Fix malformed TOML in the project-root manifest rather than editing its UUID. If a fork should intentionally learn as a separate project, remove its project-root `.coding-brain/project.toml` and rerun init.

## No sessions appear in Live

Confirm a Codex session is running and that rollout files exist under `~/.codex/sessions/`. Hook events may appear before transcript discovery has enough evidence to attach a rollout; once it does, the richer transcript identity wins.

Run doctor from the same terminal environment that owns the Codex session. For terminal-specific setup, see the [navigation matrix](terminal-support.md#navigation-matrix).

## Brain endpoint warnings

The default endpoint is loopback. A remote HTTPS endpoint produces an advisory that transcript context may leave the machine. Remote plaintext HTTP adds a stronger warning because context and credentials may be exposed in transit.

Project `.coding-brain.toml` cannot change the endpoint. Set it in `$XDG_CONFIG_HOME/coding-brain/config.toml` or pass `--url` explicitly.

## State is unavailable or corrupt

Coding Brain state is under `$XDG_STATE_HOME/coding-brain/`, normally `~/.local/state/coding-brain/`. Check ownership and permissions for that directory. A newer-schema advisory means the state was written by a newer build; upgrade before writing it again.

Activity and preference files use bounded, repair-aware writes. If doctor reports corrupt lifecycle state, let the next hook event quarantine and rebuild the snapshot, or remove only that snapshot after inspecting it.

## Agent Deck attach fails

Agent Deck is optional. Confirm its command is on `PATH` and that it can reach the tmux session itself. Cancelling or failing an attach should restore Coding Brain; use the terminal-native switch path when Agent Deck does not own the selected session.

## Rollback and purge

Normal startup and doctor do not modify old data. Before purge, reinstall the old build and rerun its init command if you need to roll back.

`coding-brain init --remove` removes managed hooks and the onboarding marker while preserving data. `coding-brain init --purge` previews the documented current and legacy global config/state targets, rechecks each target after confirmation, and deletes them. Purge is irreversible. It preserves project `.coding-brain.toml`, `.coding-brain/project.toml`, unrelated hooks, and sibling XDG files.

For declarative Home Manager hooks, disable `programs.coding-brain.codexHooks.enable` or revert the module configuration and rebuild. Do not use imperative removal as the primary rollback for declaratively managed definitions.
