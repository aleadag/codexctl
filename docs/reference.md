# CLI Reference

`codexctl --help` is the canonical option list. This page describes the main workflows.

## Dashboard and output

```bash
codexctl
codexctl --demo
codexctl --list
codexctl --json
codexctl --watch
codexctl --headless --json
```

Use status, focus, project, and search filters to narrow the dashboard. Session controls can focus a terminal, send input, approve a prompt, compact context, launch a session, or terminate one.

## Brain

```bash
codexctl --brain
codexctl --brain --auto-run
codexctl --brain --url <endpoint> --brain-model <model>
codexctl --brain-query --tool Bash --tool-input "cargo test"
codexctl --mode on|off|auto|status
```

Advisory mode leaves execution under operator control. `--auto-run` permits automatic high-confidence actions. The immediate action set is approve, deny, send, terminate, route, and spawn.

## Learning and review

```bash
codexctl --brain-review [list]
codexctl --brain-mark-canonical <decision-id>
codexctl --brain-stats <report>
codexctl --brain-outcomes
codexctl --brain-baseline [--top N]
codexctl --insights [on|off|status]
codexctl --brain-garden [--apply]
codexctl --brain-briefing --project <name>
codexctl --autopsy [--session <id>]
```

Hook-facing outcome flags such as `--record-outcome` and `--reap-outcomes` feed the same local learning store.

## Setup and diagnostics

```bash
codexctl init
codexctl init --plugin-only
codexctl init --check
codexctl init --upgrade
codexctl init --remove
codexctl init --purge
codexctl doctor [--json]
codexctl completions <shell>
codexctl man
```

`init --plugin-only` installs or refreshes the eight managed Codex lifecycle definitions without running the rest of the setup wizard. `init --upgrade` refreshes hooks and the onboarding marker without touching legacy state. `init --remove` removes only managed hooks and keeps lifecycle state; `init --purge --yes` is the explicit destructive path.

Hook installation and trust are separate. Restart Codex and review `/hooks` after installing, upgrading, or rebuilding a declarative configuration.

## Lifecycle status output

`--json` includes non-sensitive lifecycle provenance for each session:

```json
"lifecycle": {
  "available": true,
  "store_condition": "healthy",
  "last_event": "PreToolUse",
  "age_ms": 125,
  "contributing": true,
  "ignored_reason": null
}
```

The dashboard detail panel shows the same event, age, and contribution state. Lifecycle observations affect status only: they do not expose prompts, tool input or output, paths, agent ids, approval evidence, or terminal targets.

## Configuration compatibility

Legacy relay, hive, idle-task, and external-agent sections produce warnings and have no runtime effect. codexctl exposes no durable queue, dependency executor, distributed peer transport, or embedded project tracker.

## External coordination

Beads can track durable tasks, dependencies, claims, blockers, gates, and handoffs outside codexctl. It is an optional companion, not a linked library or background service.
