# Troubleshooting

Start with:

```bash
codexctl doctor
```

It checks the binary on `PATH`, Codex hooks, brain endpoint, session discovery, and terminal integration. Advisories do not make the command fail.

Lifecycle reporting is split into three checks:

- `Codex hooks` checks whether all eight managed definitions are present, current, enabled, and executable.
- `Codex hook trust` is always advisory when definitions are enabled because codexctl cannot observe Codex's trust decision. Restart Codex and inspect `/hooks`.
- `lifecycle state` checks whether `~/.codexctl/hooks/lifecycle.json` is missing, readable, corrupt, or from a newer schema.

Passing the definition check does not mean the trust check is satisfied.

## Lifecycle status is unavailable or stale

Run `codexctl doctor`, then use the matching recovery:

| Doctor result | Recovery |
| --- | --- |
| Managed definitions missing | Run `codexctl init --plugin-only`, restart Codex, then review `/hooks`. |
| Definition stale | Refresh it with `codexctl init --plugin-only`, restart Codex, and review the changed command. |
| Definition disabled | Enable it in Codex and review it through `/hooks`. |
| Definitions duplicated | Keep the managed set in either global or project scope, not both. |
| Executable unavailable | Reinstall codexctl or rebuild Home Manager so the configured executable exists. |
| Trust unverified | Inspect `/hooks`; codexctl cannot confirm this decision itself. |
| State missing | Run a Codex turn after the hooks are enabled and trusted. |
| State unavailable | Check ownership and permissions for `~/.codexctl/hooks/`. A transient lock or I/O error retains only still-fresh evidence. |
| State corrupt | Let the next lifecycle event quarantine and rebuild the snapshot, or remove only the corrupt snapshot. |
| Newer schema | Upgrade codexctl. An older binary will not write the newer snapshot. |

If the dashboard reports an identity mismatch, let normal transcript discovery resolve the session. codexctl deliberately rejects cwd-only matching, null or missing transcript paths, stale `SessionStart` hints, and ambiguous same-cwd processes.

## No sessions appear

Confirm a Codex session is running and that rollout files exist under `~/.codex/sessions/`. Run `codexctl --list` to separate discovery problems from terminal rendering problems.

## The brain is unavailable

Check the configured endpoint directly and verify the model name. For Ollama:

```bash
curl http://localhost:11434/api/tags
ollama list
```

Brain support is optional; the dashboard and deterministic rules still work without it.

## Non-loopback privacy advisory

This warning means the configured brain host is not `localhost`, `127.0.0.1`, or `::1`. Transcript context may leave the machine. Use a loopback endpoint or confirm the remote provider's data handling before enabling the brain.

## Legacy configuration warnings

`[relay]`, `[hive]`, `[idle]`, `[agents.*]`, and `lifecycle.retention_days` are no longer supported. Remove those entries when convenient. The warning is informational and does not delete legacy data.

## Upgrade, rollback, or removal

`codexctl init --upgrade` refreshes hooks and preserves `~/.codexctl`. `codexctl init --remove` removes managed hooks but keeps data. `codexctl init --purge` deletes brain data and legacy codexctl state after confirmation.

For an imperative downgrade, remove the newer definitions before replacing the binary:

```bash
codexctl init --remove     # run with the newer binary
# downgrade codexctl
codexctl init              # reinstall the older managed hooks if wanted
```

For Home Manager, set `programs.codexctl.codexHooks.enable = false` or revert the configuration that emitted the lifecycle definitions, then rebuild and switch. Confirm the definitions are gone before downgrading the selected `programs.codexctl.package`; rebuild again after changing the package. Do not use imperative `init --remove` as the primary rollback for declaratively managed hooks.

If the binary was downgraded first, restore the newer binary and run `codexctl init --remove`. The manual fallback is to remove only handlers whose bare or absolute executable resolves to codexctl and whose exact argument is `--lifecycle-hook` or `--permission-hook`. Preserve lookalike commands and neighboring user hooks.

## Terminal input or focus fails

Run `codexctl --doctor` for the legacy terminal-specific report and compare your terminal with the [support matrix](terminal-support.md). tmux and native terminal APIs may need to be enabled in the terminal itself.
