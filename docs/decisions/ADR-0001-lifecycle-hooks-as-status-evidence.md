# ADR-0001: Treat Codex Lifecycle Hooks as Status Evidence

- Status: Accepted
- Date: 2026-07-17
- Bead: `codexctl-rqm`

## Context

Transcript discovery gives codexctl durable telemetry, but transcript writes can
lag the Codex session state that an operator needs to see. Codex lifecycle hooks
provide earlier signals for prompts, tool execution, permission requests,
subagents, and stops. Those payloads can also contain commands, prompts, tool
inputs, and other data that must not become a second authorization or telemetry
channel.

The existing permission hook has a narrower responsibility: the brain may make
confident allow or deny decisions for Bash commands. Expanding lifecycle status
coverage must not silently expand that authority.

## Decision

Codexctl will consume lifecycle hooks as a bounded, status-only overlay:

- A core lifecycle projection stores validated identity, event kind, ordering,
  receipt time, and bounded diagnostic state. It never stores prompt, command,
  tool input, tool output, or raw rejected values.
- Hook state is derivative. Writers use a short advisory lock and atomic file
  replacement; consumers fall back to transcript and process evidence when the
  state is missing, invalid, newer than the supported schema, or expired.
- Process death and explicit approval or `request_user_input` evidence outrank
  hook status. Only strictly newer, non-future transcript timestamps may
  invalidate fresh hook evidence.
- `PermissionRequest` observes every tool for status. Brain inference and
  allow/deny responses remain Bash-only; non-Bash requests record
  `NeedsInput` and emit no decision.
- Lifecycle evidence cannot populate pending-tool identity, approval evidence,
  terminal targets, rule inputs, or brain authorization inputs.
- Managed hook definitions and Codex trust are separate diagnostics. Codexctl
  can verify its JSON definitions, but the operator must review trust in Codex
  with `/hooks`.
- Existing installs retain the compatibility state root under `~/.codexctl`.
  Moving state to `XDG_STATE_HOME` requires a separate migration.

The detailed event model, leases, storage bounds, rollout behavior, and test
matrix live in the [approved design](../../.internal/specs/2026-07-17-codex-lifecycle-hook-status-design.md)
and [implementation plan](../../.internal/plans/2026-07-17-codex-lifecycle-hook-status.md).

## Rationale

Hooks improve status freshness without replacing the transcript, which remains
the durable source for telemetry and semantic correction. Keeping lifecycle
data non-actionable limits the blast radius of malformed, stale, or spoofed
hook input. Preserving the Bash-only authorization boundary also lets status
coverage expand independently from the higher-risk question of which tools the
brain may approve.

A bounded snapshot is simpler than a new daemon or event database. It supports
short-lived hook processes, cross-process updates, and one dashboard read per
refresh while remaining disposable after corruption or data loss.

## Consequences

- Session status can update before the corresponding transcript line is
  visible, with provenance exposed in JSON and the TUI.
- The implementation must maintain a validated state machine, cross-process
  locking, leases, transcript reconciliation, and exact hook installation and
  removal tests.
- Hook input or persistence failures fail open for Codex operation and never
  create an authorization response. Status may temporarily fall back to the
  existing transcript, CPU, and process heuristics.
- Lifecycle writes deliberately omit `fsync`; a crash may lose recent status
  evidence without losing authoritative session data.
- Broader non-Bash authorization is deferred to `codexctl-85x`. XDG state
  migration is deferred to `codexctl-2yk`.
