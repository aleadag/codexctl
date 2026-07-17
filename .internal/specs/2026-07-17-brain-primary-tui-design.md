# Coding Brain: Brain-Primary TUI and Dashboard Removal

**Date:** 2026-07-17  
**Status:** Approved; stress test complete  
**Tracking:** `codexctl-0cy`

## Summary

Coding Brain will be a focused local judgment and learning system for coding
agents. Running `coding-brain` will open a dedicated Brain TUI with three tabs:
**Live**, **Review**, and **Scorecard**. The general session dashboard and its
session-management CLI will be removed.

The product boundary is deliberate. Coding Brain observes permission and
lifecycle hooks, applies deterministic safety rules and local-model judgment,
records what happened, and learns from corrections. It does not manage a fleet
of terminal sessions. Operators can use Agent Deck or another session manager
for that job. Agent Deck support is optional and limited to an explicit
"switch to session" navigation action.

This design replaces the broader scheduler and runner direction in
`codexctl-dk3`. Implementation planning must supersede that work so the roadmap
has one product direction.

## Product Name

- The product name is **Coding Brain**.
- The canonical executable is `coding-brain`; the project does not install a
  `cb` executable because that name collides with an existing Unix command,
  and it does not retain a `codexctl` compatibility executable.
- Documentation and user-facing output use Coding Brain and `coding-brain`.
- Public persistent namespaces use `coding-brain`: user configuration lives at
  `$XDG_CONFIG_HOME/coding-brain/config.toml`, state lives under
  `$XDG_STATE_HOME/coding-brain/`, project configuration is
  `.coding-brain.toml`, and project-owned metadata lives under
  `.coding-brain/`. The documented fallbacks are
  `~/.config/coding-brain/config.toml` and `~/.local/state/coding-brain/`.
- The repository and Rust crate names may remain `codexctl` internally for this
  migration. They are not user-facing compatibility promises.

## Product Boundary

### Goals

- Make Brain the default and only interactive TUI.
- Put denied, abstained, failed, and unresolved decisions in front of the
  operator without recreating a session dashboard.
- Keep hook evaluation useful when the TUI is closed.
- Preserve Brain review, outcomes, corrections, metrics, prompts, and learned
  preferences.
- Provide an explicit way to switch from a Brain decision to its source
  session, with optional Agent Deck integration and the existing terminal-focus
  support as fallback.
- Remove session management, runner babysitting, and the obsolete Brain
  mailbox from the product.

### Non-goals

- Replace Agent Deck, tmux, or another session manager.
- Show a compact session list inside Live.
- Spawn, route, message, terminate, or resume agents.
- Add a daemon, background service, package installer, updater, or network
  listener.
- Make Agent Deck a required dependency.
- Migrate legacy mailbox records into Brain learning data.

## Future Extension: Dream

Dream is a future Coding Brain capability, not part of the Brain-primary TUI
migration. It will run a local-model reflection over completed activity,
outcomes, corrections, and selected transcript evidence to learn durable
project context and preferences.

Dream does not require approval for every memory. It may activate a reflection
automatically when evidence, confidence, and scope gates pass. Explicit project
instructions and deterministic safety rules always take precedence, and every
active memory remains traceable and retractable.

Transcripts, source files, command output, fetched content, and model prose are
untrusted evidence. They may create inactive candidates but cannot activate a
memory without corroboration from explicit project instructions, operator
corrections, or repeated decisions with outcomes. Dream emits a strict typed
schema rather than arbitrary prompt fragments, retains evidence IDs and trust
levels, and supersedes or retracts memory when contradictory trusted evidence
arrives. Raw transcript excerpts, fetched prose, and secrets never enter the
ledger, Markdown, or Beads projection.

Dream uses three storage layers:

1. Coding Brain owns the canonical append-only ledger under its XDG state root,
   for example
   `$XDG_STATE_HOME/coding-brain/memory/<project-id>.jsonl`. Records carry
   stable IDs, kind, project and path scope, confidence, evidence references,
   model identity, creation time, and an `assert`, `supersede`, or `retract`
   operation.
2. Coding Brain generates `.coding-brain/MEMORY.md` as the concise current
   project view. It does not commit the file; each project may track or ignore
   it and may point agents to it from `AGENTS.md`.
3. When explicitly enabled in a Beads project, an optional adapter publishes a
   bounded set of active memories through stable `bd remember --key` entries.
   These entries are rebuildable projections, adapter failure never blocks
   Dream, and Coding Brain never runs Beads pull, push, or sync commands.

Dream requires a stable project identity independent of display name or
checkout path. The current TUI work should preserve this extension seam without
adding Dream commands, configuration, storage, or background execution.

### Project identity

`coding-brain init` creates `.coding-brain/project.toml` with a schema version
and random project UUID. Projects track this non-secret file when identity and
future memory should follow clones and worktrees. Activity stores the UUID plus
human-readable path and name evidence.

Without the manifest, live Brain evaluation uses an explicitly temporary
path-derived identity. Dream cannot activate durable project memory or publish
to Beads until the stable UUID exists. Coding Brain never guesses that two
project identities are equivalent; reassignment of legacy data requires a
future explicit operation.

## User Experience

### Default screen

Running `coding-brain` opens the Brain TUI on **Live**:

```text
 Coding Brain | BRAIN ACTIVE | advisory | model
 [ Live ]  Review  Scorecard

 Needs Attention
 > denied     92%  rm -rf build/                 project-a
   abstained  48%  deploy --target staging       project-b
   error       --  inference endpoint timeout    project-c

 Recent
   allowed    96%  cargo test                    project-a
   outcome     --  command exited 0              project-a

 Decision
   source       project-a / session 81f2...
   confidence   0.92 (threshold 0.85)
   rule         destructive-command
   command      rm -rf build/
   reasoning    Target could not be proven to be a disposable build path.
   session      Agent Deck, terminal fallback available

 j/k select  Enter switch session  c correct  Tab next  r refresh  q quit
```

When the endpoint is unavailable, `BRAIN ACTIVE` changes to an offline status;
the rest of the shell remains available in read-only mode.

The layout is attention-first, not session-first:

- **Needs Attention** is a bounded projection of denied, abstained, failed,
  interrupted, and unresolved activity. Repeated activity with the same
  project, rule, and normalized-command fingerprint collapses into one row with
  a count; the underlying records remain append-only.
- **Recent** is a bounded chronological view of approvals, outcomes, and other
  completed activity.
- **Decision** shows normalized evidence for the selected row: source,
  confidence and threshold, matched rule, normalized command, reasoning, and
  the available session navigation provider.

An attention item resolves through an outcome, a `c` review marking Brain
right, wrong, or an exception, or a later successful activity that explicitly
supersedes it. A denied action remains unresolved until reviewed because it
normally has no execution outcome. The projection ranks safety risk, review
value, and recency; overflow remains available in Review and history and is
shown as an unresolved-count indicator.

Live must not grow a session table, session health grid, or hidden dashboard
mode. If an operator needs to browse or manage sessions, they leave Coding Brain
and use their session manager.

### Review and Scorecard

**Review** remains the prioritized teaching queue. It surfaces decisions where
operator feedback is likely to improve future judgment, including low
confidence, disputed, high-risk, and outcome-mismatched decisions.

**Scorecard** remains the quality and safety view. It reports decision accuracy,
abstention behavior, corrections, dangerous false approvals, and other existing
Brain metrics without general session statistics.

### Interaction

- `j` and `k` move the selection.
- `Tab` moves between Live, Review, and Scorecard.
- `Enter` explicitly switches to the source session for the selected activity.
- `c` records whether Brain was right, wrong, or an exception, with an optional
  note.
- `r` refreshes persisted activity and derived views.
- `q` exits.

Approve and deny are the only executable Brain decisions. "Switch to session"
is operator navigation, not a Brain decision, and it never changes a decision
record. Send, terminate, route, and spawn are removed.

## Architecture

The existing three-crate dependency direction remains:

```text
codexctl -> codexctl-tui -> codexctl-core
```

### `coding-brain` binary

The executable is built by the existing `codexctl` package during this
migration. It owns behavior that depends on the concrete Brain implementation:

- rules, inference, prompts, risk policy, and learned preferences
- decision, outcome, correction, and activity persistence
- hook entry points and CLI dispatch
- runtime adapters that connect Brain state to the TUI
- optional Agent Deck command execution

### `codexctl-tui`

The crate remains and becomes the dedicated Brain interface. It owns:

- `BrainApp` state and terminal event handling
- Live, Review, and Scorecard rendering
- selection, refresh, correction, and switch-to-session interactions
- terminal teardown and restoration around an external session attach
- test fixtures and `ratatui::TestBackend` coverage

Dashboard table, detail, help, skills overlay, manual-input, and generic session
management modules are removed. The current Brain screen is promoted into the
normal root application instead of remaining an overlay.

### `codexctl-core`

Core continues to own shared, implementation-neutral contracts:

- normalized Brain activity, decision, outcome, and correction DTOs
- runtime traits consumed by `codexctl-tui`
- Codex session discovery as internal evidence infrastructure
- stable and temporary `ProjectId` contracts
- an opaque `SessionTarget` containing only the stable session identity,
  project identity, working directory, and provider hints needed for explicit
  navigation
- transcript and health evidence used by Brain evaluation
- terminal-focus fallback backends

No TUI or public runtime contract exposes a general session collection.
Headless evaluation may inspect full sessions internally, but it does not
restore session-list or watch output.

Today, `src/brain_screen.rs` stays in the binary because it imports binary-only
metrics and risk types. The migration moves scorecard and risk projections
behind core runtime contracts so the TUI does not import binary modules or
duplicate Brain policy.

## Hook-First Runtime

Coding Brain does not need a resident process. Permission and lifecycle hooks invoke
the Brain pipeline independently, persist activity, and return control to
Codex. Opening the TUI reads that persisted state; closing it does not stop
evaluation.

`--headless` is the only continuous evaluator. It retains structured JSON
output for machine consumers. The default interactive process is a cockpit over
persisted activity, not another evaluator loop.

If the configured Brain endpoint is unavailable, the TUI still opens in
read-only mode. Live shows endpoint health and setup guidance alongside
historical activity, while Review and Scorecard remain usable. Hooks safely
abstain. Coding Brain does not install or start a model service automatically.

## Activity and Learning Data

### Lifecycle

Every evaluation attempt follows an append-only lifecycle:

```text
observed -> evaluating -> allowed | denied | abstained | error
                                 -> outcome -> correction
```

An activity can have one terminal evaluation state. The first terminal record
wins; later terminal records for the same activity are skipped with a visible
diagnostic, so they cannot silently rewrite history.

An `evaluating` record that remains unfinished beyond the configured internal
timeout is projected as `interrupted` when activity is read.

### Stores

- `activity.jsonl` records every hook and headless attempt, including disabled,
  low-confidence, malformed, unsupported, failed, and interrupted attempts. Its
  retained history is bounded. Records are immutable, but bounded compaction
  may evict the oldest complete activity lifecycles while preserving the newest
  valid ones.
- `decisions.jsonl` remains the resolved learning set. It contains decisions
  suitable for retrieval, outcomes, review, metrics, and preference learning;
  it is not expanded into an operational event log.
- Outcomes and corrections reference stable activity and decision IDs. They do
  not copy the normalized command into each record.

Live reads normalized activity DTOs and joins decisions, outcomes, and
corrections by ID. Projections may be rebuilt from the append-only records;
corrections never edit an earlier row in place.

All activity appenders and compactors use the same cross-process `fs2` lock
file. Hooks hold the lock only while appending one complete JSON line.
Compaction runs from TUI or headless maintenance, never on the hook hot path;
it acquires the exclusive lock, retains complete recent lifecycles, flushes a
temporary file, and atomically replaces the log. Maintenance skips compaction
when it cannot acquire the lock promptly. Hook lock or write failure follows
the persistence-failure rules below.

### Evaluation order

For each hook request:

1. Validate and bound the supported input.
2. Persist `observed`, then `evaluating`.
3. Apply deterministic deny rules before model inference.
4. Run inference only when the request is supported and no deny rule has
   matched.
5. Persist the final activity state and any resolved learning record before
   returning allow or deny.
6. On parsing, inference, or timeout failure, append `error` when possible and
   abstain so Codex uses its native prompt.
7. If persistence fails after a deterministic deny rule matches, return deny
   and emit a hook diagnostic when possible. A deterministic safety deny fails
   closed even without its audit record. If persistence fails for a model-derived
   allow or deny, abstain; Coding Brain never emits an automatic allow without
   its audit record.

### Learning maintenance

Preference distillation uses a persisted watermark rather than a process-local
counter. After recording a decision, a short-lived hook may spawn a detached
one-shot internal distiller; this is maintenance work, not a daemon. The worker
acquires a cross-process lock, exits if another worker is active, and processes
only when enough decisions exist after the watermark.

Preferences and the new watermark are written atomically. A crash leaves the
old watermark so a later worker retries. TUI and headless startup invoke the
same catch-up path. Distillation failure never delays or changes a hook
decision; Scorecard and doctor expose stale-learning health.

## Optional Agent Deck Navigation

Agent Deck is discovered only when the operator presses `Enter`. Coding Brain does
not require it at startup and does not monitor it in the background.

The adapter uses Agent Deck's public CLI:

- Query sessions with `agent-deck list --json`.
- Resolve the selected Brain activity to exactly one Agent Deck session using
  stable session evidence such as ID, title, or path.
- Invoke `agent-deck session attach <id-or-title>` with an argument vector, not
  a shell command string.

Coding Brain never guesses when the result is missing or ambiguous, and it never
uses Agent Deck's internal tmux sockets or session names. If Agent Deck is not
installed or cannot resolve one session, Coding Brain tries the existing
terminal-focus backend. If neither provider succeeds, it restores the TUI and
shows a nonfatal error.

Before attaching, the TUI installs a `TerminalSuspendGuard`, leaves raw mode and
the alternate screen, and launches Agent Deck with inherited standard streams.
While the child is active, terminal input and Ctrl-C belong to the child. The
guard restores terminal mode, the alternate screen, active tab, and selection
after normal exit, nonzero exit, spawn failure, unwind, or a handled termination
signal.

Coding Brain does not inspect `$TMUX`, switch tmux clients directly, or add
nested-session behavior. Agent Deck's public CLI remains authoritative. A
failed attach may try terminal focus once before reporting a nonfatal error.
An endpoint failure only changes Brain evaluation to abstain: it never
synthesizes a terminal `Enter` or an attach. A user can still press `Enter` on
persisted activity to request navigation explicitly.

## CLI and Configuration

### Retained

- default Brain TUI
- Brain evaluation, prompts, metrics, review, outcomes, baseline, insights,
  garden, briefing, autopsy, mode, and canonical marking
- `--headless`, including JSON machine output
- init, doctor, configuration validation and templates, hooks, completions,
  manual pages, and diagnostics
- internal permission, lifecycle, and outcome hook entry points
- endpoint and model overrides, plus TUI theme settings

The model endpoint may be loopback, LAN, or remote. Only an explicit CLI flag
or user-level configuration may select it; project `.coding-brain.toml` cannot
override the endpoint. TUI and doctor visibly warn for non-loopback endpoints
that normalized project context may leave the machine. Selecting the endpoint
establishes model trust, so automatic decisions remain available; deterministic
local deny rules still run before inference.

### Removed

- session list and JSON list, watch, summary, filtering, search, new, resume,
  history, and generic statistics
- dashboard refresh interval, dashboard debug, and dashboard demos not required
  by headless operation
- budget-kill behavior, notifications, and webhooks
- dashboard and session recording, session cleanup, skills overlay, table,
  detail, help, and manual input
- send, terminate, route, and spawn commands, configuration, runtime traits,
  and implementations
- Brain mailbox commands, configuration, runtime contracts, persistence, tests,
  and documentation

Removed command-line flags disappear from help and fail parsing. Removed
configuration keys produce explicit unsupported-setting warnings instead of
being silently accepted.

## Persistent Paths and Pre-release Reset

This is an immediate breaking contraction, not a staged deprecation. Release
notes must direct users who need session management to Agent Deck or another
manager.

Coding Brain reads and writes only its new public namespaces:

- user configuration: `$XDG_CONFIG_HOME/coding-brain/config.toml`, falling back
  to `~/.config/coding-brain/config.toml`;
- user state: `$XDG_STATE_HOME/coding-brain/`, falling back to
  `~/.local/state/coding-brain/`;
- project configuration: `.coding-brain.toml`;
- project identity and generated memory: `.coding-brain/`.

There is no executable, hook, configuration, or data migration command. The new
binary stops reading and writing `.codexctl.toml`, `~/.config/codexctl`, and
`~/.codexctl`. It leaves those paths untouched; the sole pre-release tester may
reset or copy wanted data manually. `coding-brain init` writes only current
`coding-brain` hook commands and current namespaces.

The Brain mailbox has no continuing role. The new binary stops reading and
writing it, does not import mailbox messages into `activity.jsonl` or
`decisions.jsonl`, and leaves existing mailbox files untouched. The explicit
`coding-brain init --purge` path may delete documented files under the known old
namespaces after the operator requests destructive cleanup.

## Failure and Security Rules

- Deterministic deny rules run first and model output cannot override them.
- Only supported tool-call types are evaluated; malformed or unsupported input
  abstains.
- Automatic allow or deny requires the configured confidence and risk gates.
- The final activity and resolved learning record are persisted before an
  allow or deny response is emitted.
- Parsing, endpoint, inference, and timeout errors abstain. Persistence errors
  abstain for model-derived decisions, while deterministic safety denies still
  return deny.
- Activity stores normalized context, not raw prompts, full model responses, or
  complete hook payloads.
- Project configuration cannot redirect model traffic. Non-loopback endpoints
  are allowed only when selected by CLI or user-level configuration and remain
  visibly identified as external.
- Commands and notes are length-bounded and secret-shaped values are redacted.
- State files use restrictive permissions and the existing safe append or
  locking pattern.
- Malformed JSONL rows are skipped with a visible diagnostic; valid rows remain
  readable.
- Corrections are append-only.
- Agent Deck receives a fixed executable plus an argument vector. It is called
  only after explicit operator input and cannot mutate Brain decisions.
- Coding Brain adds no daemon, installer, updater, or listener.

## Verification

### Activity and persistence

- Cover every valid state transition, duplicate terminal states, ordering,
  bounded history, stale `evaluating` projection, malformed rows, restart
  recovery, truncation, and redaction.
- Run concurrent appenders against repeated compaction and prove that no
  successfully appended record disappears.
- Preserve the per-run nonce in isolated tests because sandbox PID reuse can
  otherwise expose state from an earlier test process.

### Hook behavior

- Prove deterministic denies run before inference.
- Prove model-derived allow and deny records are persisted before hook output.
- Prove parsing, endpoint, inference, and timeout failures abstain.
- Prove deterministic-deny persistence failure returns deny, while
  model-decision persistence failure abstains and automatic allow never occurs
  without an audit record.
- Run hooks as separate processes with no TUI and prove one distiller advances
  the persisted watermark. Cover lock contention, worker crash and retry,
  atomic preference replacement, and large decision-history fixtures.

### TUI

- Use `ratatui::TestBackend` snapshots or buffer assertions for the default Live
  tab, attention and recent lists, detail pane, corrections, empty state, and
  offline state.
- Cover duplicate collapse, outcome/review/supersession resolution, ranked
  overflow, and the unresolved-count indicator without deleting source events.
- Prove `Enter` emits only a navigation request and never a Brain action.
- Prove external attach restores raw mode, alternate-screen state, selected tab,
  and row selection.

### Agent Deck

- Test JSON fixtures for exact, missing, and ambiguous session matches.
- Prove Agent Deck is optional and queried only after explicit navigation.
- Prove arguments are passed without shell interpretation.
- Prove terminal-focus fallback and the nonfatal failure path.
- Use a fake Agent Deck executable to cover successful attach, blocking then
  exit, nonzero exit, spawn failure, and signal handling while verifying
  terminal restoration.

### Removal and compatibility

- Removed CLI surfaces are absent from help and fail parsing.
- Removed configuration keys produce explicit warnings.
- Normal startup does not read, write, copy, or delete old `.codexctl`
  namespaces.
- Explicit purge removes only the documented files under known old namespaces.
- No removed action, mailbox, dashboard, or session-management runtime API
  remains public.
- Project identity tests cover two same-named repositories, clones and
  worktrees sharing a tracked UUID, missing-manifest temporary identity, and
  refusal to activate durable memory without a stable UUID.

### Workspace gates

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

## Implementation Shape

Deliver the change as an unreleased stack of compiling, reviewable jj
changesets:

1. Add the normalized activity model, append-only persistence, and stable
   project identity with tests. These state changes are additive.
2. Move Brain projections and runtime contracts into `codexctl-core`, then
   build the new `BrainApp` in `codexctl-tui` while the old dashboard still
   compiles internally.
3. Switch the default TUI and installed hook definitions to BrainApp; implement
   Live while preserving Review, Scorecard, corrections, and offline behavior.
4. Add explicit switch-to-session navigation with optional Agent Deck and the
   terminal-focus fallback.
5. Remove the dashboard, session-management actions and CLI, mailbox code, and
   obsolete configuration. Add unsupported-setting warnings and purge coverage.
6. Rename the executable to `coding-brain` last, update documentation and
   breaking release notes, then run all workspace gates.

Each slice should remove any imports, tests, or configuration made obsolete by
that slice, while leaving unrelated Brain behavior unchanged. No intermediate
slice is released, no feature flag or dual binary is added, and old state stays
untouched so rollback remains a code rollback.

## Stress Test Results: Coding Brain Product Design

### Resolved Decisions

- The rename is an immediate pre-release break: no `codexctl` shim, `cb` alias,
  or migration command.
- Session discovery remains internal evidence and cannot expose a general
  session collection through the TUI, headless output, or public runtime traits.
- Deterministic safety denies fail closed when audit persistence fails;
  model-derived decisions abstain instead of acting without an audit record.
- Activity append and compaction share one cross-process lock, and compaction
  never runs on the hook hot path.
- Stable project UUIDs gate durable memory; missing manifests use temporary
  identities for live evaluation only.
- Live attention is a bounded projection with duplicate collapse, explicit
  resolution, ranked overflow, and immutable source events.
- Agent Deck attachment suspends and restores the terminal through a guard and
  never reaches into tmux internals.
- Dream treats repository and transcript content as untrusted evidence and
  requires trusted corroboration before automatic activation.
- Implementation is an unreleased stack of compiling jj changesets with
  additive state first, removals later, and the executable rename last.
- Preference distillation uses a persisted watermark and locked one-shot worker
  so learning continues across short-lived hook processes without a daemon.
- Any model endpoint is allowed, but only a CLI flag or user-level configuration
  can select it; project configuration cannot redirect traffic, and
  non-loopback use remains visibly identified.
- All public persistent namespaces move coherently to `coding-brain` XDG,
  project-config, and project-metadata paths without compatibility reads or a
  migration command.

### Changes Made

- Added explicit rename, hook-failure, project-identity, activity-locking,
  attention-resolution, terminal-suspension, Dream trust, sequencing, and
  learning-maintenance requirements.
- Restricted endpoint selection to operator-controlled configuration and made
  the `coding-brain` namespace consistent across all persistent paths.
- Expanded verification for multi-process persistence, compaction, project
  identity, attention overflow, attach lifecycle, and distillation recovery.

### Deferred / Parking Lot

- Dream commands, reflection prompts, memory retrieval, and Beads publication
  remain a future feature.
- Reassigning legacy state between project UUIDs requires a future explicit
  operation.

### Confidence Assessment

- Overall: High.
- Areas of concern: no blocking design concerns remain. The implementation plan
  must set measurable hook-latency and state-compaction budgets.
