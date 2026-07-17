# Configuration

codexctl loads global configuration first and project configuration second. CLI flags override both.

- Global: `~/.config/codexctl/config.toml`
- Project: `.codexctl.toml`

Print the template with `codexctl --config-template`, inspect resolved values with `codexctl --config`, and validate a file with `codexctl --config-validate`.

## Home Manager

With codexctl declared as a flake input, import its Home Manager module and configure it with your other Home Manager modules:

```nix
{
  imports = [ inputs.codexctl.homeManagerModules.default ];

  programs.codex.enable = true;
  programs.codexctl = {
    enable = true;
    codexHooks.enable = true;
    settings.brain = {
      enabled = true;
      endpoint = "http://localhost:11434/api/generate";
      model = "gemma4:e4b";
      auto = false;
      timeout_ms = 25000;
      terminal_auto_approve_fallback = false;
    };
  };
}
```

Apply the configuration with your Home Manager configuration name in place of `<profile>`:

```bash
home-manager switch --flake .#<profile>
```

The module installs its selected package, writes the settings as TOML, and merges the eight managed lifecycle definitions into `programs.codex.hooks`. Each handler calls the selected codexctl package by its immutable Nix store path rather than relying on `PATH`. Hooks configured by other Home Manager modules remain independent and are preserved.

`programs.codexctl.settings` is visible in the Nix store. Do not put secrets, tokens, credentials, or token-bearing URLs in it.

Changing the codexctl package changes the trusted hook definition. After an upgrade, rebuild Home Manager, restart Codex, and review `/hooks` before trusting the new handlers.

## Codex lifecycle hooks

For a non-Nix installation, install or refresh only the managed hooks with:

```bash
codexctl init --plugin-only
```

codexctl merges these definitions into `~/.codex/hooks.json` and preserves unrelated hooks:

| Codex event | Matcher | Handler | Timeout |
| --- | --- | --- | ---: |
| `SessionStart` | `startup\|resume\|clear\|compact` | `--lifecycle-hook` | 2s |
| `UserPromptSubmit` | none | `--lifecycle-hook` | 2s |
| `PreToolUse` | `*` | `--lifecycle-hook` | 2s |
| `PermissionRequest` | `*` | `--permission-hook` | 30s |
| `PostToolUse` | `*` | `--lifecycle-hook` | 2s |
| `SubagentStart` | `*` | `--lifecycle-hook` | 2s |
| `SubagentStop` | `*` | `--lifecycle-hook` | 2s |
| `Stop` | none | `--lifecycle-hook` | 2s |

The lifecycle handlers provide immediate status evidence to the dashboard. They do not copy prompts, commands, tool input, or tool output into lifecycle state, and they cannot approve a tool or send terminal input. `PermissionRequest` contributes status for every tool, but brain allow/deny decisions remain limited to Bash requests.

Lifecycle state is a bounded, reconstructible snapshot at `~/.codexctl/hooks/lifecycle.json`, protected by `~/.codexctl/hooks/lifecycle.lock`. Config remains at `~/.config/codexctl/config.toml`. Removing hooks leaves the snapshot in place; expired evidence is ignored.

Installing a definition does not establish that Codex trusts it. Restart Codex and use `/hooks` to inspect and trust the exact commands after installation, an upgrade, or a Home Manager rebuild.

## Brain

```toml
[brain]
enabled = true
endpoint = "http://localhost:11434/api/generate"
model = "gemma4:e4b"
auto = false
timeout_ms = 5000
max_context_tokens = 4000
few_shot_count = 5
max_sessions = 10
orchestrate = false
orchestrate_interval = 30
test_runners = ["cargo test", "npm test", "pytest", "go test", "bun test"]
```

`auto = false` keeps suggestions advisory. The CLI `--auto-run` enables automatic execution for that invocation. `orchestrate` allows periodic cross-session evaluation for immediate route, spawn, terminate, or deny decisions; it is not a durable task runner.

Loopback endpoints are recommended. When an enabled brain uses another host, codexctl warns that transcript context may leave this machine.

## Lifecycle

```toml
[lifecycle]
auto_restart = false
restart_threshold_pct = 90.0
restart_only_when_idle = true
```

Lifecycle restart is local session maintenance. It does not schedule project work.

## Rules and file conflicts

```toml
[orchestrate]
file_conflicts = true
auto_deny_file_conflicts = false

[rules.approve_reads]
match_tool = ["Read", "Glob", "Grep"]
action = "approve"

[rules.deny_destructive]
match_tool = ["Bash"]
match_command = ["rm -rf", "push --force"]
action = "deny"
```

Rules can use `approve`, `deny`, `send`, or `terminate`. Brain suggestions can additionally route context or spawn a live session.

## Budgets, health, and hooks

Top-level budget, notification, filtering, model-price overrides, health thresholds, and `[hooks.*]` sections remain supported. Run `codexctl --config-template` for the canonical key list.

## Removed configuration

The following legacy settings are ignored and reported as warnings:

- `[relay]`
- `[hive]`
- `[idle]`
- `[agents.*]`
- `lifecycle.retention_days`

These warnings do not delete data. Normal startup and `codexctl init --upgrade` preserve legacy files under `~/.codexctl`; only `codexctl init --purge` removes them.
