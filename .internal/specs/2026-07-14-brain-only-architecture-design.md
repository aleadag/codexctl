# Brain-Only Codexctl Architecture

**Date:** 2026-07-14  
**Status:** Approved for planning  
**Tracking:** `codexctl-1h8`

## Summary

Codexctl will return to the local-brain product boundary described by upstream
claudectl. It will observe Codex sessions, evaluate tool calls and session
health with a local LLM, learn from operator corrections, and optionally carry
out immediate brain decisions through `--auto-run`.

Codexctl will stop owning durable project coordination. The coord supervisor,
loop runner, agent bus, relay, hive, and dependency-ordered task runner will be
removed. Projects that need durable task state, dependencies, claims, blockers,
handoffs, or workflow gates can use Beads directly. Codexctl will not add a
Beads adapter or mirror Beads state.

## Goals

- Make the local brain the clear product focus.
- Retain advisory mode and opt-in `--auto-run`.
- Preserve current brain learning, review, metrics, autopsy, briefing, and
  AGENTS.md-gardening capabilities.
- Keep immediate cross-session brain actions: approve, deny, send, terminate,
  route, spawn, and context-saturation restart.
- Remove durable task execution and distributed coordination from codexctl.
- Preserve existing brain data and configuration paths.

## Non-Goals

- Reimplement coord semantics on top of Beads.
- Import coord tasks, attempts, or verifier history into Beads.
- Provide a general-purpose task runner, retry engine, verifier pipeline, or
  worktree orchestrator.
- Preserve bus, relay, hive, supervisor, or loop command compatibility.
- Delete legacy user data during a normal upgrade.

## Architecture

```text
Codex sessions and hooks
          |
          v
session discovery, transcripts, health signals
          |
          v
deterministic rules + local brain + learned preferences
          |
          v
approve | deny | send | terminate | route | spawn
          |
          v
manual confirmation or opt-in --auto-run
          |
          v
local decision log -> retrieval -> preference learning
```

### Retained components

- `codexctl-core`: session discovery, transcript parsing, health checks, rules,
  terminal integration, hooks, and shared runtime types.
- `codexctl-tui`: session monitoring and brain review interfaces.
- `src/brain/**`: inference, context construction, decisions, preference
  learning, retrieval, review, metrics, risk analysis, autopsy, briefing,
  outcomes, and the brain mailbox.
- Binary runtime adapters required to observe sessions and execute immediate
  brain actions.
- Brain-focused configuration, doctor checks, initialization, and docs.

### Removed components

- `src/coord/**`
- `src/loop/**`
- `src/bus/**`
- `src/relay/**`
- `src/hive/**`
- `src/orchestrator.rs`, the durable dependency-ordered task-file runner
- `src/ingest.rs`, which exists only to feed coord hook events
- Supervisor, ingest, loop, bus, relay, and hive CLI surfaces
- Matching TUI panels, runtime views, configuration, docs, feature flags, and
  the optional SQLite dependency used by coord

`src/runtime/orchestrator.rs` is not deleted wholesale. Its brain-mailbox
delivery responsibility remains, while coord interrupt delivery and lease
expiration are removed. The surviving adapter should be named for brain
delivery rather than durable orchestration.

## Runtime Behavior

Codex hooks and the TUI inspect active sessions, recent transcript blocks,
pending tool calls, costs, health signals, and modified files. Deterministic
rules run before local-model inference; deny rules cannot be overridden by the
brain.

The brain evaluates either one session's pending action or the global session
map. Advisory mode queues the suggestion for operator acceptance. With
`--auto-run`, codexctl executes a suggestion only after its confidence, risk,
session-count, and file-conflict checks pass.

Routing may persist a short-lived message in
`~/.codexctl/brain/mailbox/<pid>.jsonl` until the target session can accept
input. This mailbox is session delivery state, not a project task system: it
has no task dependencies, ownership, retries, verifier state, or long-running
workflow lifecycle.

Every suggestion and execution outcome remains in the local brain decision
log. Accepted and rejected decisions continue to feed few-shot retrieval and
preference learning.

## Beads Boundary

Beads is the recommended external source of truth when a project needs durable
coordination:

- issues and operational state for tasks and status
- dependency edges for blockers and ready work
- atomic claims and assignees for ownership
- merge slots for exclusive resources
- notes, comments, and reassignment for handoffs
- gates for human, timer, CI, pull-request, and cross-project waits
- `bd remember` for durable shared memory
- history and event beads for auditability

Beads does not itself observe, spawn, resume, or terminate live Codex
processes. A separate agent or worker can consume Beads ready work when that
automation is desired. Codexctl will not embed that worker.

## Failure and Safety Behavior

- `--auto-run` remains opt-in.
- Deterministic deny rules always override model output.
- Low-confidence, ambiguous, and file-conflicting decisions require manual
  confirmation.
- Session spawning respects the configured `max_sessions` limit.
- Brain inference or terminal-action failures return control to the normal
  manual path and never create an autonomous retrying task.
- The configured brain endpoint remains visible. A non-loopback endpoint warns
  that transcript context may leave the machine.
- Removing relay and hive eliminates codexctl's background peer networking.
- Brain logs and preferences remain under `~/.codexctl/brain`.

## Upgrade and Compatibility

This is an intentional breaking product contraction and must be prominent in
the changelog and release notes.

A normal upgrade leaves existing coord, bus, hive, relay, and loop data
untouched; the new binary stops reading it. The explicit
`codexctl init --purge` path may retain knowledge of legacy directories so a
user can deliberately delete them.

Removed `[relay]` and `[hive]` sections, along with obsolete task-runner
settings, produce a clear unsupported-setting warning instead of being
silently ignored. Brain settings remain compatible, including
`orchestrate`, `orchestrate_interval`, `max_sessions`, and lifecycle
auto-restart. Decisions, preferences, prompt overrides, mailbox data,
`.codexctl.toml`, and `~/.config/codexctl/config.toml` also remain compatible.

Coord state is not migrated into Beads. Runtime attempts and verifier records
do not map reliably to project tasks, and importing them would create
misleading work items.

## Verification

Implementation is complete when:

- Workspace builds no longer expose coord, bus, relay, or hive features.
- Removed commands are absent from CLI help and dispatch.
- Brain prompts contain no coord or hive context.
- Advisory mode and `--auto-run` retain their current behavior.
- Unit tests cover inference parsing, deterministic rules, confidence gates,
  conflict detection, mailbox delivery, preference learning, and immediate
  actions.
- Configuration tests cover retained brain settings and warnings for removed
  sections.
- An upgrade fixture with legacy coord, bus, hive, relay, and loop data proves
  that normal startup leaves those files unchanged.
- Documentation describes codexctl as a local-brain companion rather than a
  durable orchestrator.
- `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings` pass for
  the workspace.

## Implementation Shape

The change should be delivered in reviewable slices:

1. Remove durable task entry points and their runtime wiring.
2. Remove distributed coordination modules and simplify feature/dependency
   definitions.
3. Reduce runtime and TUI contracts to sessions, brain views, brain actions,
   and brain delivery.
4. Remove coord/hive inputs and outputs from the brain while preserving its
   local learning data.
5. Simplify initialization, doctor checks, configuration, docs, and upgrade
   warnings.
6. Run the full verification suite and document the breaking release.

Each slice must keep the workspace compiling and should avoid unrelated brain
refactoring.
