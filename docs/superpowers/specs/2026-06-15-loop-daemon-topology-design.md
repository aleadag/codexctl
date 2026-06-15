# Loop Daemon Topology Design

Date: 2026-06-15

## Goal

Make `codexctl loop` useful as a long-running automation feature without making
the dashboard responsible for backend work.

The system should support two independent daemon roles:

- a loop daemon that polls sources, triages items, dedupes work, and submits
  accepted tasks to coord
- a headless coord daemon that executes submitted tasks through the supervisor,
  `codex exec`, verifier gates, retries, resume, and escalation

The dashboard should attach to the durable state produced by those daemons. It
may provide controls, but it must not be required for automation to keep running.

## Current State

The first loop runtime slice already separates the major responsibilities:

- `codexctl loop run <name>` performs a one-shot source poll, item triage, loop
  state update, and coord task submission.
- Loop state lives in `~/.codexctl/loop/loop.db`.
- Coord task state lives in `~/.codexctl/coord/coord.db`.
- Coord owns task states, attempts, verifiers, retries, resume, and escalation.
- `codexctl --headless` is the intended no-TUI runtime for brain, coordination,
  and context rot prevention.

The missing piece is durable scheduling and process topology. A user should not
need to keep the dashboard open for source polling, task execution, or outcome
reconciliation.

## Decision

Use two daemon roles and keep their state ownership separate.

```text
codexctl loop daemon
  -> discovers project-local loop configs
  -> runs each enabled loop on cadence
  -> updates loop.db
  -> submits accepted work to coord.db
  -> reconciles completed loop items from coord/transcripts

codexctl --headless
  -> ticks the coord supervisor
  -> assigns or spawns task attempts
  -> runs verifiers
  -> handles retry, resume, and escalation
  -> emits machine-readable daemon events

codexctl dashboard
  -> reads loop.db and coord.db
  -> shows loop, task, and daemon health
  -> offers operator controls
```

Do not make `loop` a second executor. `loop` decides what work should exist.
`coord` remains the only task executor.

## Why Not Dashboard-Owned Backend Work

Pulling the backend into the dashboard is useful for development, but it is the
wrong default runtime model:

- automation stops when the TUI exits
- terminal rendering, input handling, polling, source IO, and task execution
  become one failure domain
- headless servers and CI hosts need automation without an attached terminal
- system service management becomes unclear because the dashboard is interactive

The dashboard can still offer a convenience mode later, such as "start local
backend daemons for this session," but that should wrap the daemon commands
rather than embed their control loops as the primary implementation.

## Scope

### Loop Daemon

Add a long-running command:

```bash
codexctl loop daemon [--name <loop>] [--once] [--json]
```

Behavior:

- Discover enabled project-local `.codexctl/loops/*.toml` definitions.
- Treat `cadence` as the polling interval for each loop.
- Run only due loops.
- Reuse the same logic as `codexctl loop run <name>`.
- Respect existing pause markers.
- Enforce per-loop `gates.max_items_per_run`.
- Record daemon/run events in `loop_events`.
- Periodically reconcile submitted loop items whose coord tasks are terminal.
- Exit non-zero only for daemon-level failures, not for one bad source item.

V1 should be project-scoped. A process started in one repo manages loops from
that repo only. User-scoped multi-root discovery can be added later.

### Coord Headless Daemon

Keep the execution daemon separate:

```bash
codexctl --headless --json
```

or, if the CLI is made more explicit later:

```bash
codexctl coord daemon --json
```

Behavior:

- Tick the supervisor reconciler.
- Apply supervisor actions through the actuator.
- Spawn headless work with `codex exec` when a task is configured for headless
  execution.
- Run verifier gates.
- Recover from daemon restarts by reading coord state.
- Emit structured JSON events for status, errors, and transitions.

### Dashboard

The dashboard should stay attachable:

- Show loop definitions and recent loop runs.
- Show loop item state, associated coord task ids, and result URLs.
- Show whether expected daemons appear healthy.
- Provide controls for pause/resume loop and drain/undrain coord.

It should not be required to poll sources or execute tasks.

## State Ownership

```text
loop.db
  loop_runs
  loop_sources
  loop_items
  loop_events

coord.db
  tasks
  task_attempts
  task_transitions
  verifier results
```

Cross-links should stay one-way from loop to coord:

- `loop_items.coord_task_id` points at the submitted coord task.
- Coord should not need to know which loop created a task, except through
  existing task metadata/policy fields when useful for display.

## Scheduling

The loop daemon should parse simple cadence values already accepted in loop
configuration, such as `15m`, `2h`, and `1d`.

Each loop needs its own next-due calculation so one slow or failing loop does
not block unrelated loops indefinitely. A simple single-threaded scheduler is
enough for V1:

1. Load loop configs.
2. Select loops that are due.
3. Run each due loop sequentially.
4. Record success or failure.
5. Sleep until the nearest next due time, capped by a short maximum sleep so
   config changes are picked up promptly.

No distributed locking is required in V1. If a lock is needed, use a local
pid/lock file under `~/.codexctl/loop/` to prevent two loop daemons in the same
project from running concurrently.

## Service Model

The recommended production shape is two user services per project host:

```text
codexctl-loop@<project>.service
  ExecStart=codexctl loop daemon --json
  WorkingDirectory=<project>

codexctl-headless.service
  ExecStart=codexctl --headless --json
```

`codexctl loop install-service` can be added later to generate these service
files. The first implementation should focus on the daemon commands, because
systemd generation is packaging work around the same runtime boundary.

## Failure Handling

- Source fetch failure: record a loop event, keep the daemon alive, retry on
  the next cadence.
- Invalid loop config: record/report the validation error and skip that loop.
- Missing triage model or skill: fail validation for that loop and skip it.
- Coord submission failure: keep the loop item in a failed/escalated state with
  the error recorded.
- Headless daemon down: loop can still submit tasks; they remain pending until
  coord is running again.
- Loop daemon down: existing coord tasks continue under the headless daemon.

## Out Of Scope

- A loop-specific executor.
- Dashboard-owned background execution as the primary runtime.
- User-scoped multi-root loop discovery.
- Distributed scheduling or multi-host leader election.
- Automatic PR merge or source mutation after a task completes.
- Service-file generation in the first daemon implementation.

## Acceptance Criteria

- `codexctl loop daemon --once` runs all due enabled loops once and exits.
- `codexctl loop daemon` repeatedly runs due loops by cadence.
- Paused loops are skipped and reported as skipped.
- A submitted item still creates exactly one coord task for a stable source id.
- If `codexctl --headless` is not running, submitted coord tasks remain pending.
- When `codexctl --headless` starts, pending loop-submitted tasks are picked up
  by the coord supervisor.
- The dashboard can display loop state without owning the loop scheduler.
- Unit tests cover cadence parsing, due-loop selection, pause handling, and
  one-shot daemon execution.
- Existing loop tests continue to pass.

