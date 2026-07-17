# Coding Brain Product Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> beads-superpowers:subagent-driven-development (recommended) or
> beads-superpowers:executing-plans to implement this plan task-by-task. Each
> Task is already a child bead of `codexctl-662`. Steps within tasks use
> checkbox (`- [ ]`) syntax for human readability.

**Goal:** Ship `coding-brain` as a hook-first local judgment and learning
product whose only interactive TUI is Live, Review, and Scorecard, while
removing dashboard and session-management behavior.

**Architecture:** Add a normalized append-only activity store and stable project
identity first. Expose Brain projections through `codexctl-core`, render them in
a new `BrainApp` inside `codexctl-tui`, and keep Codex transcript discovery
internal for evaluation and explicit navigation. Remove the old dashboard and
management contracts only after BrainApp and headless evaluation are live, then
rename the runtime namespace. Update packaging and current documentation in the
final unreleased changeset after the new CLI contract is executable.

**Tech Stack:** Rust 2024 with Rust 1.88, Clap, Serde/JSONL, `fs2`, Ratatui,
Crossterm, existing Codex transcript and terminal integrations, Agent Deck's
public CLI, Nix/Home Manager, Jujutsu, and Beads.

**Spec:** `.internal/specs/2026-07-17-brain-primary-tui-design.md`

**ADR:** `docs/decisions/ADR-0002-coding-brain-product-boundary.md`

**Tracking:** Beads epic `codexctl-662`. This epic supersedes the scheduler and
runner direction in `codexctl-dk3`; Task 2 supersedes the separate audit-log
bead `codexctl-8yc`, and Task 8 supersedes the old compatibility migration bead
`codexctl-2yk`.

**Execution order:** Tasks are a strict chain, `1 -> 2 -> 3 -> 4 -> 5 -> 6 ->
7 -> 8 -> 9`. They intentionally do not run in parallel because adjacent tasks
share decision, runtime, and module-registration files.

## Global Constraints

- The product name is **Coding Brain** and the only installed executable is
  `coding-brain`; do not add `cb` or a `codexctl` compatibility executable.
- The repository, root Cargo package, and Rust crate names may remain
  `codexctl`, `codexctl-core`, and `codexctl-tui` internally.
- Preserve the dependency direction `codexctl -> codexctl-tui ->
  codexctl-core`; core must not import binary-only Brain modules.
- Brain's executable decisions are allow and deny. Parsing, endpoint,
  inference, timeout, unsupported-input, and low-confidence cases abstain.
- Deterministic safety denies run before inference and fail closed even when
  audit persistence fails. Model-derived allow or deny requires a persisted
  final audit record or it abstains.
- `decisions.jsonl` records model proposals and learning evidence;
  `activity.jsonl` is the authoritative decision-commit and lifecycle audit. A
  model proposal must persist first, then its terminal activity referencing the
  same `decision_id`, before the hook emits a response. Failure of either append
  abstains.
- `Allowed` and `Denied` mean the hook decision was durably committed, not that
  Codex received it or ran the tool. A best-effort `Delivered` or
  `DeliveryFailed` event follows the stdout attempt; a missing delivery event
  projects as `DeliveryUnknown`. Only lifecycle/outcome evidence proves tool
  execution.
- Codex native exec-policy remains the configurable shell policy. New
  deterministic safety denies are code-owned, deny-only invariants and must not
  resurrect legacy configurable `AutoRule` approval or management actions.
- Permission authorization remains Bash-only under ADR-0001. Other lifecycle
  tools may produce non-actionable activity evidence, but broadening executable
  decisions beyond Bash remains separate future work.
- Standard hook output remains one valid Codex response envelope or empty.
  Diagnostics use standard error, and bounded hook input remains 64 KiB.
- The permission-hook model timeout remains capped at 25 seconds. Activity
  lock acquisition on the hook path is capped at 100 ms; uncontended activity
  append overhead must remain below 20 ms at p95 in the release scale test.
- Retain at most 10,000 complete activity lifecycles. Consider compaction at
  32 MiB; a release-mode fixture with 100,000 events must compact in under five
  seconds without losing a successfully appended record.
- Activity append repairs a crash-truncated tail while holding the store lock:
  finish a valid unterminated JSON value, or truncate an invalid fragment to
  the last newline and append only a bounded discarded-byte diagnostic.
- Live shows at most 100 Needs Attention rows and 100 Recent rows. Overflow
  remains reachable in Review/history and is represented by an unresolved
  count. The internal TUI refresh interval is one second and is not configurable.
- Activity stores normalized bounded context, not raw prompts, full hook
  payloads, full model responses, fetched prose, or secrets. Commands and notes
  are redacted and length-bounded before persistence.
- Public configuration is
  `$XDG_CONFIG_HOME/coding-brain/config.toml`, falling back to
  `~/.config/coding-brain/config.toml`. Public state is
  `$XDG_STATE_HOME/coding-brain/`, falling back to
  `~/.local/state/coding-brain/`.
- Project configuration is `.coding-brain.toml`; project identity and generated
  memory live under `.coding-brain/`. Project configuration cannot select or
  redirect the model endpoint.
- A valid tracked project UUID is authoritative across clones, worktrees, and
  forks. Never infer identity from names, paths, or remotes. A user who wants a
  separate identity removes the manifest and reruns `coding-brain init`.
- The runtime must not read `.codexctl.toml`, `~/.config/codexctl`, or
  `~/.codexctl` after the final rename. There is no automatic or command-driven
  migration. Normal startup leaves old data untouched.
- `coding-brain init --purge` is the only destructive compatibility path. It
  previews and, after confirmation, removes only documented current and legacy
  global targets; it never deletes `.coding-brain.toml` or `.coding-brain/` in a
  project.
- Purge accepts only absolute non-root bases and fixed lexical children. It
  rejects escaping targets, previews file types, revalidates them before
  deletion, and unlinks symlinks without following them.
- Agent Deck is optional, queried only after explicit `Enter`, invoked through
  `agent-deck list --json` and `agent-deck session attach`, and never accessed
  through tmux internals or shell command strings.
- Agent Deck schema additions are ignored, missing required fields are
  nonfatal, and a user-cancelled attach restores Brain without attempting the
  terminal-focus fallback.
- Endpoint choice remains unrestricted when selected by CLI or user config.
  Redact and bound model-bound context, pass curl request bodies over standard
  input, disable redirects, cap dynamic prompt context at 48 KiB and responses
  at 1 MiB, and show stronger warnings for plaintext non-loopback HTTP. These
  warnings do not disable automatic action.
- Distillation publishes immutable preference generations and atomically swaps
  a current-generation watermark only after every file is flushed. Readers
  never use or write an unpublished generation.
- Coding Brain adds no daemon, network listener, service manager, model
  installer, updater, or automatic Beads synchronization.
- Dream commands, reflection prompts, retrieval, ledger writes, Markdown
  generation, and Beads publication are not part of this implementation. Only
  the stable project identity and typed extension seam are in scope.
- Keep each implementation changeset buildable and described as
  `<emoji> <type>: <imperative summary> (<bead-id>)`. Do not publish an
  intermediate changeset.
- The supported cutover is install, `coding-brain init`, `coding-brain doctor`,
  then restart Codex. Normal startup only diagnoses stale hooks. Preserve old
  data until separately confirmed purge so reinstalling the old build and
  rerunning its init remains the rollback path.

## File Structure

The final ownership boundaries are:

- `crates/codexctl-core/src/paths.rs`: deterministic XDG and project-path
  resolution with injectable inputs for tests.
- `crates/codexctl-core/src/project.rs`: stable/temporary `ProjectId`, manifest
  validation, and project evidence.
- `crates/codexctl-core/src/brain_activity.rs`: normalized activity, attention,
  review, scorecard, correction, endpoint-health, and navigation DTOs.
- `crates/codexctl-core/src/runtime.rs`: Brain-only read/write/navigation
  traits; no public session roster or session-management action.
- `src/brain/activity.rs`: locked `activity.jsonl` append, read, projection, and
  bounded compaction.
- `src/brain/safety.rs`: code-owned deterministic deny-only rules used before
  inference.
- `src/brain/distill.rs`: persisted watermark, one-shot worker, and atomic
  preference maintenance.
- `src/runtime/brain.rs`: production Brain projection and correction adapter.
- `src/runtime/navigation.rs`: lazy Agent Deck resolution and terminal-focus
  fallback over opaque `SessionTarget` values.
- `crates/codexctl-tui/src/brain_app.rs`: Live/Review/Scorecard application
  state and event handling.
- `crates/codexctl-tui/src/ui/brain/{mod,live,review,scorecard}.rs`: dedicated
  Brain rendering.
- `crates/codexctl-tui/src/terminal_suspend.rs`: idempotent terminal teardown,
  external attach, and restoration.
- `src/{main,commands,config,doctor}.rs` and `src/init/`: retained CLI,
  configuration, diagnostics, hooks, onboarding, and explicit purge.

After callers move, remove these dashboard-era files:

- `crates/codexctl-tui/src/app.rs`
- `crates/codexctl-tui/src/demo.rs`
- `crates/codexctl-tui/src/recorder.rs`
- `crates/codexctl-tui/src/session_recorder.rs`
- `crates/codexctl-tui/src/ui/{detail,help,skills,status_bar,table}.rs`
- `src/brain_screen.rs`
- `src/brain/mailbox.rs`
- `src/runtime/{actions,brain_driver,brain_review,delivery,sessions}.rs`

Keep `codexctl-core` transcript discovery, session evidence, and terminal-focus
backends as internal implementation support. Remove only their public roster,
launch, input, terminate, and approval-management exposure.

---

### Task 1: Add Coding Brain Paths, Project Identity, and Activity Store (`codexctl-662.1`)

**Files:**

- Create: `crates/codexctl-core/src/paths.rs`
- Create: `crates/codexctl-core/src/project.rs`
- Create: `crates/codexctl-core/src/brain_activity.rs`
- Create: `src/brain/activity.rs`
- Modify: `crates/codexctl-core/src/lib.rs`
- Modify: `crates/codexctl-core/Cargo.toml`
- Modify: `src/brain/mod.rs`
- Modify: `Cargo.toml`
- Test: inline `#[cfg(test)]` modules in the four created Rust files
- Test: `tests/activity_scale.rs`

**Interfaces:**

- Produces: `CodingBrainPaths::resolve(env: &PathEnvironment) ->
  Result<CodingBrainPaths, PathError>` with `config_file`, `state_root`,
  `project_config(cwd)`, and `project_dir(cwd)`.
- Produces: `ProjectId::{Stable(String), Temporary(String)}` and
  `ProjectIdentity::load(cwd, paths) -> Result<ProjectIdentity, ProjectError>`.
- Produces: `ProjectManifest::create(cwd, paths) -> Result<ProjectIdentity,
  ProjectError>` writing schema version `1` and a random UUID to
  `.coding-brain/project.toml` atomically.
- Produces: `ActivityEvent`, `ActivityState`, `ActivityOutcome`,
  `CorrectionDisposition`, `SessionTarget`, `AttentionItem`, and
  `ActivitySnapshot` in core.
- Produces: `ActivityStore::{append, read, compact_if_needed, snapshot}` in the
  binary crate.

**Acceptance Criteria:**

- New code resolves only the documented Coding Brain XDG and project paths.
- Stable UUID manifests and temporary identities behave correctly across
  clones, worktrees, same-named repositories, and missing manifests.
- Concurrent append and compaction lose no successful record; malformed rows
  remain diagnosable; stale evaluation and first-terminal-state rules hold.
- Committed decisions distinguish delivered, delivery-failed, and
  delivery-unknown projections; none are treated as proof of tool execution.
- Activity retention, attention limits, redaction, and scale budgets from the
  Global Constraints are enforced by tests.

- [ ] **Step 1: Claim the task and start a described changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.1 --claim
jj new -m "🧱 feat: add Coding Brain activity foundation (codexctl-662.1)"
```

Expected: the bead is `in_progress` and `@` is an empty described changeset.

- [ ] **Step 2: Write failing path and identity tests**

Define the public contracts in `paths.rs` and `project.rs` and add tests with
explicit environment inputs rather than mutating process-global `HOME`:

```rust
#[test]
fn resolves_xdg_paths_and_documented_fallbacks() {
    let explicit = PathEnvironment::new(Some("/cfg"), Some("/state"), Some("/home/alex"));
    let paths = CodingBrainPaths::resolve(&explicit).unwrap();
    assert_eq!(paths.config_file(), Path::new("/cfg/coding-brain/config.toml"));
    assert_eq!(paths.state_root(), Path::new("/state/coding-brain"));

    let fallback = CodingBrainPaths::resolve(&PathEnvironment::new(
        None,
        None,
        Some("/home/alex"),
    ))
    .unwrap();
    assert_eq!(
        fallback.config_file(),
        Path::new("/home/alex/.config/coding-brain/config.toml")
    );
    assert_eq!(
        fallback.state_root(),
        Path::new("/home/alex/.local/state/coding-brain")
    );
}

#[test]
fn missing_manifest_is_temporary_and_cannot_enable_durable_memory() {
    let dir = tempfile::tempdir().unwrap();
    let identity = ProjectIdentity::load(dir.path(), &fixture_paths(dir.path())).unwrap();
    assert!(matches!(identity.id(), ProjectId::Temporary(_)));
    assert!(!identity.is_durable());
}

#[test]
fn tracked_manifest_keeps_identity_across_checkout_paths() {
    let first = tempfile::tempdir().unwrap();
    let created = ProjectManifest::create(first.path(), &fixture_paths(first.path())).unwrap();
    let second = tempfile::tempdir().unwrap();
    copy_manifest(first.path(), second.path());
    let loaded = ProjectIdentity::load(second.path(), &fixture_paths(second.path())).unwrap();
    assert_eq!(created.id(), loaded.id());
}

#[test]
fn copied_manifest_is_authoritative_until_user_resets_it() {
    let original = fixture_project_with_manifest();
    let fork = copy_project_fixture(&original);
    assert_eq!(identity(&original).id(), identity(&fork).id());

    fs::remove_file(fork.path().join(".coding-brain/project.toml")).unwrap();
    assert!(matches!(identity(&fork).id(), ProjectId::Temporary(_)));
}
```

Run:

```bash
cargo test -p codexctl-core paths::tests
cargo test -p codexctl-core project::tests
```

Expected: FAIL because the path and identity modules do not exist.

- [ ] **Step 3: Implement paths and stable project identity**

Use these core shapes and reject unsupported schema versions or malformed UUIDs:

```rust
pub const PROJECT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ProjectId {
    Stable(String),
    Temporary(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub project_id: String,
}

#[derive(Debug, Clone)]
pub struct CodingBrainPaths {
    config_file: PathBuf,
    state_root: PathBuf,
}
```

Use `uuid::Uuid::new_v4()` for creation and `uuid::Uuid::parse_str()` for
validation. Serialize the two-field manifest with `toml`, write a sibling
temporary file with mode `0600` on Unix, flush it, and atomically rename it.
Temporary identity hashes the canonical checkout path only for the current
machine; it is visibly temporary and is never written as a stable UUID.

- [ ] **Step 4: Write failing activity lifecycle and projection tests**

Add core DTO tests and binary store tests covering every transition:

```rust
#[test]
fn first_terminal_state_wins_and_late_terminal_is_diagnostic() {
    let store = fixture_store();
    store.append(event("a1", ActivityState::Observed)).unwrap();
    store.append(event("a1", ActivityState::Evaluating)).unwrap();
    store.append(event("a1", ActivityState::Denied)).unwrap();
    store.append(event("a1", ActivityState::Allowed)).unwrap();

    let log = store.read().unwrap();
    assert_eq!(log.activity("a1").unwrap().terminal_state(), ActivityState::Denied);
    assert_eq!(log.diagnostics().duplicate_terminal_states, 1);
}

#[test]
fn stale_evaluating_projects_as_interrupted_without_rewriting_source() {
    let store = fixture_store_with_clock(1_000);
    store.append(event_at("a1", ActivityState::Observed, 100)).unwrap();
    store.append(event_at("a1", ActivityState::Evaluating, 101)).unwrap();
    let snapshot = store.snapshot(SnapshotLimits::default()).unwrap();
    assert_eq!(snapshot.attention[0].state, ActivityState::Interrupted);
    assert_eq!(store.read().unwrap().events().len(), 2);
}

#[test]
fn committed_decision_without_delivery_evidence_projects_unknown() {
    let store = fixture_store_with_clock(1_000);
    store.append(event_at("a1", ActivityState::Allowed, 100)).unwrap();
    let snapshot = store.snapshot(SnapshotLimits::default()).unwrap();
    assert_eq!(snapshot.attention[0].delivery, DeliveryState::Unknown);
    assert!(!snapshot.attention[0].tool_execution_confirmed);
}

#[test]
fn repeated_attention_collapses_but_source_events_remain() {
    let store = fixture_store();
    append_denial(&store, "a1", "project", "destructive", "rm -rf build");
    append_denial(&store, "a2", "project", "destructive", "rm -rf build");
    let snapshot = store.snapshot(SnapshotLimits::default()).unwrap();
    assert_eq!(snapshot.attention.len(), 1);
    assert_eq!(snapshot.attention[0].occurrences, 2);
    assert_eq!(store.read().unwrap().complete_lifecycles(), 2);
}
```

Also cover malformed rows, bounded strings, secret-shaped redaction, explicit
outcome/review/supersession resolution, denied-item review requirements,
ranking, 100-row limits, unresolved overflow count, valid JSON without a final
newline, an invalid crash-truncated tail followed by another append, delivery
failure, delivery unknown, and later outcome confirmation.

Run:

```bash
cargo test -p codexctl-core brain_activity::tests
cargo test --lib brain::activity::tests
```

Expected: FAIL because the DTOs and store do not exist.

- [ ] **Step 5: Implement locked append, read, projection, and compaction**

Use an immutable event shape with stable IDs and bounded normalized fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub schema_version: u32,
    pub activity_id: String,
    pub recorded_at_ms: u64,
    pub project: ProjectEvidence,
    pub session: Option<SessionTarget>,
    pub state: ActivityState,
    pub tool: Option<String>,
    pub normalized_command: Option<String>,
    pub fingerprint: Option<String>,
    pub rule_id: Option<String>,
    pub confidence: Option<f64>,
    pub threshold: Option<f64>,
    pub reasoning: Option<String>,
    pub decision_id: Option<String>,
    pub outcome: Option<ActivityOutcome>,
    pub correction: Option<CorrectionDisposition>,
    pub supersedes: Option<String>,
}

pub struct ActivityStore {
    path: PathBuf,
    lock_path: PathBuf,
    limits: ActivityLimits,
}
```

Serialize one complete JSON line before locking. Retry `try_lock_exclusive()`
only until the 100 ms hook budget expires. Under the lock, inspect bytes after
the final newline. Complete a valid JSON value by appending its newline; for an
invalid fragment, truncate to the final complete newline and append a bounded
`TruncatedTail { discarded_bytes }` diagnostic without copying the raw bytes.
Then append the new line with one `write_all`, flush, and unlock.

Create state directories as `0700` and activity and lock files as `0600` on
Unix. Readers continue after malformed complete lines and report bounded line
offsets. Compaction uses the same exclusive lock, skips immediately when busy,
retains the newest 10,000 complete lifecycles plus a bounded corruption
summary, writes and flushes a temporary file, then atomically replaces the
source. It never runs from the hook append function. Two same-named repositories
without manifests must get different temporary identities, and Coding Brain
never guesses that two stable UUIDs are equivalent.

- [ ] **Step 6: Add concurrency and release scale tests**

`tests/activity_scale.rs` must spawn multiple writer threads while repeatedly
calling compaction, collect every successfully appended ID, and assert each ID
survives unless it belongs to a lifecycle intentionally evicted by retention.
The release-only scale test writes 100,000 events, triggers compaction, asserts
the 10,000-lifecycle bound, and records filesystem location, append p95, and
compaction elapsed time. A separate normal-profile 100,000-event test asserts
only correctness and retention, never wall-clock performance.
Add a helper-process case killed during a deliberately split append; the next
writer must repair the tail and its valid event must remain readable.

Run:

```bash
cargo test --test activity_scale concurrent_append_and_compaction_preserve_successes
cargo test --test activity_scale hundred_thousand_events_preserve_retention
cargo test --release --test activity_scale release_activity_budgets -- --ignored --test-threads=1
```

Expected: both tests pass; append p95 is below 20 ms and compaction completes in
under five seconds on the local temporary filesystem.

- [ ] **Step 7: Verify and finish the changeset**

```bash
cargo fmt --all --check
cargo test -p codexctl-core
cargo test --lib brain::activity::tests
cargo clippy -p codexctl-core --all-targets -- -D warnings
jj --no-pager st
```

Expected: all focused gates pass and only Task 1 files are changed. Close
`codexctl-662.1` with the commands and results in the reason.

---

### Task 2: Record the Hook-First Activity Lifecycle (`codexctl-662.2`)

**Files:**

- Create: `src/brain/safety.rs`
- Modify: `src/brain/mod.rs`
- Modify: `src/brain/query.rs`
- Modify: `src/brain/client.rs`
- Modify: `src/brain/permission_hook.rs`
- Modify: `src/brain/decisions.rs`
- Modify: `src/brain/outcomes.rs`
- Modify: `src/lifecycle_hook.rs`
- Modify: `src/commands.rs`
- Test: inline tests in the files above
- Test: `tests/hook_activity.rs`

**Interfaces:**

- Consumes: `ActivityStore`, `ActivityEvent`, `ProjectIdentity`, and the existing
  bounded Codex hook payload parsers.
- Produces: `safety::evaluate(&BrainDecisionRequest) -> Option<SafetyDeny>`;
  `SafetyDeny` contains a stable `rule_id` and bounded operator-facing reason
  and can never represent allow.
- Produces: `permission_hook::evaluate_request(...) -> HookEvaluation`, where
  `HookEvaluation::{Allow, Deny, Abstain}` carries the terminal activity state
  already prepared for persistence.
- Produces: `decisions::append_hook_proposal(...) -> Result<DecisionId,
  DecisionError>`; proposal records never claim that Codex received or executed
  the action.
- Produces: one activity ID shared by observed, evaluating, terminal, outcome,
  and correction records.

**Acceptance Criteria:**

- Every supported hook attempt records observed, evaluating, and exactly one
  terminal activity state when persistence is available.
- Every committed response records delivered or delivery-failed best-effort;
  absence of either is delivery-unknown and never claims tool execution.
- Deterministic denies run before inference and still deny after audit failure;
  model decisions abstain unless both their proposal and authoritative terminal
  activity persist.
- Unsupported, malformed, timeout, endpoint, and inference failures abstain and
  remain visible in activity when a normalized record can be written.
- Hook stdout/stderr, 64 KiB input, and 25-second model-timeout contracts remain
  intact.
- Model payloads reuse the activity redactor, never appear in curl argv, obey
  prompt/response limits, and do not follow redirects.

- [ ] **Step 1: Claim the task and start a described changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.2 --claim
jj new -m "🧠 feat: record hook-first Brain activity (codexctl-662.2)"
```

- [ ] **Step 2: Add deterministic deny and evaluation-order red tests**

Add tests that use injected inference and persistence:

```rust
#[test]
fn irreversible_root_delete_denies_without_inference() {
    let request = request("rm -rf /");
    let deny = safety::evaluate(&request).unwrap();
    assert_eq!(deny.rule_id, "irreversible-root-delete");
}

#[test]
fn deterministic_deny_precedes_inference() {
    let calls = AtomicUsize::new(0);
    let result = evaluate_fixture(request("rm -rf /"), |_, _| {
        calls.fetch_add(1, Ordering::SeqCst);
        panic!("deterministic deny must not invoke the model")
    });
    assert!(matches!(result, HookEvaluation::Deny { deterministic: true, .. }));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn ordinary_command_reaches_inference() {
    assert!(safety::evaluate(&request("cargo test")).is_none());
}
```

The deny-only invariant must cover literal root/home deletion and an unresolved
empty or root-valued expansion used as a recursive deletion target. It must not
approve commands or import legacy configurable `AutoRule` values.

Run:

```bash
cargo test --lib brain::safety::tests
cargo test --lib brain::permission_hook::tests
```

Expected: FAIL until the safety module and ordered evaluator exist.

- [ ] **Step 3: Harden model transport and integrate permission activity**

Before inference, run command/diff context through the Task 1 secret redactor
and cap dynamic context at 48 KiB. Keep the fixed decision instruction outside
that budget. Replace curl `-d <body>` with a piped standard-input body and cap
response collection at 1 MiB:

```rust
let mut child = Command::new("curl")
    .args([
        "--silent",
        "--show-error",
        "--max-redirs",
        "0",
        "--max-filesize",
        "1048576",
        "--data-binary",
        "@-",
        endpoint,
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;
child.stdin.take().unwrap().write_all(body.as_bytes())?;
```

Assert the fake curl process receives no prompt fragment in argv, an oversized
response abstains, redirects are not followed, and explicit non-loopback
endpoints still reach inference.

Build one activity ID after strict payload parsing and use this ordering:

```rust
let observed_error = activity
    .append(observed(&request, &identity, &activity_id))
    .err();
let evaluating_error = activity
    .append(evaluating(&request, &identity, &activity_id))
    .err();

if let Some(deny) = safety::evaluate(&brain_request) {
    let decision_id = decisions.append_deterministic(&deny).ok();
    let persisted = activity.append(denied(
        &request,
        &identity,
        &activity_id,
        decision_id.as_ref(),
        &deny,
    ));
    return HookEvaluation::deterministic_deny(
        deny,
        observed_error.or(evaluating_error).or(persisted.err()),
    );
}

if let Some(error) = observed_error.or(evaluating_error) {
    return HookEvaluation::abstain_persistence(error);
}

let decision = query::evaluate(&brain_request, brain_config, gate_mode);
let decision_id = match decisions.append_hook_proposal(&request, &decision) {
    Ok(decision_id) => decision_id,
    Err(error) => return HookEvaluation::abstain_persistence(error),
};
let terminal = terminal_event(
    &request,
    &identity,
    &activity_id,
    &decision_id,
    &decision,
);
match (decision.executable(), activity.append(terminal)) {
    (Some(executable), Ok(())) => HookEvaluation::committed(executable),
    (Some(_), Err(error)) => HookEvaluation::abstain_persistence(error),
    (None, result) => HookEvaluation::abstain(decision, result.err()),
}
```

Serialize the hook response before final persistence as today. A deterministic
deny writes the Codex deny envelope even if final activity or decision audit
append fails, with a bounded stderr diagnostic. A model allow/deny writes no
envelope after either persistence failure. A proposal left without a terminal
activity is explicitly non-executed and excluded from learning projections.
Initial activity failure therefore still permits deny-only safety evaluation
but stops model inference.

The outer hook runner writes a committed response only after the ordering above
and then appends delivery evidence without changing the already safe response:

```rust
match stdout.write_all(evaluation.serialized_response()) {
    Ok(()) => {
        let _ = activity.append(evaluation.delivered_event());
    }
    Err(error) => {
        let _ = activity.append(evaluation.delivery_failed_event(&error));
        write_diagnostic(stderr, error);
    }
}
```

`Delivered` means the complete envelope was written to the hook pipe, not that
Codex parsed it or executed the tool. If the process dies after the write and
before the append, projection derives `DeliveryUnknown`; a later lifecycle or
outcome event is the only execution confirmation.

- [ ] **Step 4: Add full hook failure-matrix tests**

`tests/hook_activity.rs` must run the adapter with fake stores and assert:

| Case | Proposal | Terminal | Stdout | Delivery projection | Meaning |
| --- | --- | --- | --- | --- | --- |
| deterministic deny | best effort | succeeds | deny succeeds | delivered | committed deny written |
| deterministic deny, audit down | fails | fails | deny succeeds | diagnostic only | fail-closed deny written |
| model allow/deny | succeeds | succeeds | succeeds | delivered | committed response written |
| model allow/deny | fails | not attempted | empty | none | abstained |
| model allow/deny | succeeds | fails | empty | proposal only | abstained |
| committed model action | succeeds | succeeds | fails | delivery failed | no execution claim |
| killed after stdout | succeeds | succeeds | succeeds | delivery unknown | receipt/execution unknown |
| low confidence | succeeds | abstained | empty | not applicable | abstained |
| malformed/unsupported | not applicable | best effort | empty | not applicable | error/abstained |
| endpoint/timeout | succeeds | best effort | empty | not applicable | error/abstained |

Also assert that duplicate terminal appends cannot rewrite the first terminal
state, unpaired proposals are excluded from learning, and stdout contains either
exactly one JSON envelope or no bytes. A helper-process kill after stdout proves
the projection becomes delivery-unknown; a later outcome confirms execution
without rewriting the original terminal state.

- [ ] **Step 5: Join lifecycle and outcome records by stable IDs**

Extend normalized lifecycle writes to record non-actionable hook activity and
attach `session_id`, `turn_id`, and `tool_use_id` only as bounded identity
fields. Change `record_outcome` and the outcome reaper to append an
`ActivityState::Outcome` event referencing the originating `activity_id` and
`decision_id`; do not copy the command. When attribution is unavailable, append
an explicit orphan outcome diagnostic rather than guessing by project name.

```rust
activity.append(ActivityEvent::outcome(
    attribution.activity_id,
    attribution.decision_id,
    normalized_outcome,
))?;
```

- [ ] **Step 6: Verify isolated hook processes**

```bash
cargo test --lib brain::safety::tests
cargo test --lib brain::permission_hook::tests
cargo test --lib lifecycle_hook::tests
cargo test --test hook_activity
cargo clippy --all-targets -- -D warnings
jj --no-pager st
```

Expected: the matrix passes, inference is never called for deterministic
denies, and no model decision escapes without its audit record. Close
`codexctl-662.2` with the focused evidence.

---

### Task 3: Make Preference Distillation Restart-Safe (`codexctl-662.3`)

**Files:**

- Create: `src/brain/distill.rs`
- Modify: `src/brain/mod.rs`
- Modify: `src/brain/decisions.rs`
- Modify: `src/brain/preferences.rs`
- Modify: `src/brain/pref_store.rs`
- Modify: `src/main.rs`
- Modify: `src/commands.rs`
- Test: inline tests in `src/brain/distill.rs`
- Test: `tests/distill_process.rs`

**Interfaces:**

- Consumes: the Task 1 Coding Brain state root and existing decision/preference
  readers.
- Produces: `DistillWatermark { schema_version: 1,
  through_decision_id: Option<String>, generation_id: Option<String> }`
  persisted at `brain/distill-watermark.json`; the generation ID points to an
  immutable complete tree under `brain/preferences-generations/`.
- Produces: `run_once(paths: &CodingBrainPaths) -> Result<DistillOutcome,
  DistillError>` guarded by `brain/distill.lock`.
- Produces: hidden `--distill-once` dispatch and
  `spawn_one_shot_if_due(paths) -> io::Result<()>`.

**Acceptance Criteria:**

- At most one distiller processes a watermark range at a time.
- Crashes leave the old watermark and a later process retries without changing
  a hook decision.
- Readers observe either the complete previous preference generation or the
  complete new one, never a mixture, and never write preferences on demand.
- TUI and headless startup catch up missed work; hook latency is not coupled to
  distillation.
- Multi-process, crash-retry, atomic-write, and 100,000-decision history tests
  pass within the five-second release budget.

- [ ] **Step 1: Claim the task and start a described changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.3 --claim
jj new -m "🛠️ refactor: make Brain distillation restart-safe (codexctl-662.3)"
```

- [ ] **Step 2: Replace process-local assumptions with red tests**

Delete the intended behavior of `DECISION_COUNT` and `DISTILLING` from tests and
add store-driven cases:

```rust
#[test]
fn crash_before_atomic_commit_keeps_old_watermark() {
    let fixture = DistillFixture::with_decisions(25);
    fixture.write_watermark(Some("dec_10"));
    let result = run_once_with(&fixture.paths, |_batch| Err("injected crash".into()));
    assert!(result.is_err());
    assert_eq!(fixture.read_watermark().through_decision_id.as_deref(), Some("dec_10"));
}

#[test]
fn second_worker_exits_when_lock_is_held() {
    let fixture = DistillFixture::with_decisions(25);
    let held = fixture.hold_lock();
    assert_eq!(run_once(&fixture.paths).unwrap(), DistillOutcome::AlreadyRunning);
    drop(held);
}

#[test]
fn successful_run_advances_to_last_processed_decision() {
    let fixture = DistillFixture::with_decisions(25);
    assert!(matches!(run_once(&fixture.paths).unwrap(), DistillOutcome::Updated { .. }));
    assert_eq!(
        fixture.read_watermark().through_decision_id.as_deref(),
        Some("dec_25")
    );
}

#[test]
fn crash_before_pointer_swap_keeps_previous_generation_visible() {
    let fixture = DistillFixture::with_published_generation("gen_1");
    fixture.fail_after_generation_file(2);
    assert!(run_once(&fixture.paths).is_err());
    assert_eq!(fixture.reader().generation_id(), Some("gen_1"));
    assert_eq!(fixture.read_watermark().generation_id.as_deref(), Some("gen_1"));
}
```

Run:

```bash
cargo test --lib brain::distill::tests
```

Expected: FAIL because the durable worker does not exist.

- [ ] **Step 3: Implement the locked one-shot worker**

Acquire `distill.lock` with `try_lock_exclusive`; return `AlreadyRunning` rather
than blocking. Read decisions after the watermark, excluding proposal records
without authoritative terminal activity, return `NotDue` below the existing
ten-decision interval, and build both global and project preferences in memory.

Write the complete result to a new immutable
`brain/preferences-generations/<generation_id>/` directory. Flush every file
and the generation directory, then atomically replace the watermark/current
pointer last. Readers load only the named generation; a missing, corrupt, or
mismatched generation makes learned preferences unavailable and the hook
abstain rather than scanning another directory. Remove on-demand preference
writeback from read paths. Under the distillation lock, later maintenance may
delete abandoned generations while retaining the current and previous
published generations. On any pre-pointer error, leave the old watermark so the
batch retries.

Use this dispatch shape:

```rust
if cli.distill_once {
    brain::distill::run_once(&paths).map_err(io::Error::other)?;
    return Ok(());
}
```

The hook-side trigger uses `std::env::current_exe()`, `--distill-once`, null
standard streams, and `spawn()` without `wait()`. Failure to spawn is a health
diagnostic only and never changes the hook response.

- [ ] **Step 4: Add process and scale coverage**

`tests/distill_process.rs` must launch multiple current-test helper processes
against one temporary state root, prove exactly one watermark advancement, kill
a worker after each generation-file boundary and before the pointer swap, prove
readers retain the old generation, and prove a later process retries. After a
successful swap, prove readers see the complete new generation and maintenance
keeps only current/previous plus any actively written generation. A normal
fixture verifies a 100,000-decision generation and watermark without a time
limit. The ignored release fixture records its filesystem and elapsed time and
must finish in under five seconds. Use an injected clock for due-interval and
100 ms lock-cutoff tests rather than wall-clock sleeps.

Run:

```bash
cargo test --test distill_process
cargo test --test distill_process hundred_thousand_decisions_publish_one_generation
cargo test --release --test distill_process release_distill_budget -- --ignored --test-threads=1
```

- [ ] **Step 5: Wire catch-up without a daemon**

Call `spawn_one_shot_if_due` only after a decision append returns. Call
`run_once` synchronously during TUI and headless startup before their first
projection refresh. Expose `last_success`, `pending_decisions`, and last error
through the later scorecard/doctor adapter, but do not create a timer, service,
or resident worker.

- [ ] **Step 6: Verify and finish the changeset**

```bash
cargo fmt --all --check
cargo test --lib brain::distill::tests
cargo test --test distill_process
cargo clippy --all-targets -- -D warnings
jj --no-pager st
```

Expected: all focused tests pass and no process-local counter controls
distillation. Close `codexctl-662.3` with the evidence.

---

### Task 4: Build the Brain Runtime Contract and Primary TUI (`codexctl-662.4`)

**Files:**

- Modify: `crates/codexctl-core/src/brain_activity.rs`
- Modify: `crates/codexctl-core/src/runtime.rs`
- Create: `crates/codexctl-tui/src/brain_app.rs`
- Create: `crates/codexctl-tui/src/ui/brain/mod.rs`
- Create: `crates/codexctl-tui/src/ui/brain/live.rs`
- Create: `crates/codexctl-tui/src/ui/brain/review.rs`
- Create: `crates/codexctl-tui/src/ui/brain/scorecard.rs`
- Modify: `crates/codexctl-tui/src/ui/mod.rs`
- Modify: `crates/codexctl-tui/src/lib.rs`
- Modify: `src/runtime/mod.rs`
- Modify: `src/runtime/brain.rs`
- Modify: `src/runtime/brain_review.rs`
- Modify: `src/runtime/actions.rs`
- Modify: `src/brain/metrics.rs`
- Modify: `src/brain/risk.rs`
- Test: inline tests in the new TUI modules and runtime adapters

**Interfaces:**

- Produces alongside the still-compiling dashboard `BrainRuntime`, containing
  `Arc<dyn BrainSource>` and `Arc<dyn BrainActions>`.
- Produces: `BrainSource::{snapshot, review_queue, scorecard, gate_mode,
  endpoint_health}` returning core-owned DTOs only.
- Produces: `BrainActions::{record_correction, mark_canonical, set_gate_mode}`.
- Produces: `BrainApp::new(runtime, theme)`, `BrainApp::refresh()`,
  `BrainApp::handle_key(KeyEvent) -> Option<BrainEffect>`, and
  `BrainEffect::SwitchToSession(SessionTarget)`.
- Produces: `BrainTab::{Live, Review, Scorecard}` with Live as default.

**Acceptance Criteria:**

- Live is the default attention-first tab with bounded Needs Attention, Recent,
  detail, offline state, duplicate collapse, explicit resolution, and overflow
  count.
- Delivery-failed and delivery-unknown decisions are visible in Needs Attention
  and never rendered as confirmed tool execution.
- Review and Scorecard preserve teaching, canonical marking, accuracy,
  abstention, dangerous false-approval, and correction workflows.
- The TUI depends only on `codexctl-core` contracts and no new public runtime
  trait exposes a session collection.
- Ratatui buffer tests cover all tabs, navigation effects, corrections, empty
  state, and offline state.

- [ ] **Step 1: Claim the task and start a described changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.4 --claim
jj new -m "🖥️ feat: build the primary Brain TUI (codexctl-662.4)"
```

- [ ] **Step 2: Add the Brain-only runtime contract and mock tests**

Define a parallel aggregate so the old dashboard compiles until Task 7:

```rust
pub trait BrainSource: Send + Sync {
    fn snapshot(&self, limits: SnapshotLimits) -> Result<ActivitySnapshot, String>;
    fn review_queue(&self) -> Result<Vec<ReviewItemSummary>, String>;
    fn scorecard(&self) -> Result<ScorecardSummary, String>;
    fn gate_mode(&self) -> BrainGateMode;
    fn endpoint_health(&self) -> EndpointHealth;
}

pub trait BrainActions: Send + Sync {
    fn record_correction(&self, correction: CorrectionInput) -> Result<(), String>;
    fn mark_canonical(&self, decision_id: &str, note: Option<String>) -> Result<(), String>;
    fn set_gate_mode(&self, mode: BrainGateMode) -> Result<(), String>;
}

#[derive(Clone)]
pub struct BrainRuntime {
    pub source: Arc<dyn BrainSource>,
    pub actions: Arc<dyn BrainActions>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrainEffect {
    SwitchToSession(SessionTarget),
}
```

Extend `MockRuntime` or add `MockBrainRuntime` with an action log. Tests must
prove correction and canonical calls retain exact IDs and notes and that the
Brain aggregate has no session-list, inject-text, terminate, or mailbox field.

- [ ] **Step 3: Project existing binary metrics into core DTOs**

Move no binary algorithm upward. Instead, add core-owned `ScorecardSummary`,
`RiskTierSummary`, `LatencySummary`, `CacheSummary`, and
`CounterfactualSummary`, then convert the existing `brain::metrics` and
`brain::risk` output in `LiveBrainSource`. `codexctl-tui` must not import
`crate::brain::*` or duplicate risk policy.

Implement corrections as append-only Task 1 activity events:

```rust
pub struct CorrectionInput {
    pub activity_id: String,
    pub disposition: CorrectionDisposition,
    pub note: Option<String>,
}

pub enum CorrectionDisposition {
    BrainRight,
    BrainWrong,
    Exception,
}
```

Bound and redact the note before append. A correction resolves its attention
item but never mutates the source activity row.

- [ ] **Step 4: Write BrainApp state and key-handling red tests**

Add tests before rendering:

```rust
#[test]
fn defaults_to_live_and_cycles_all_tabs() {
    let mut app = fixture_app();
    assert_eq!(app.tab(), BrainTab::Live);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.tab(), BrainTab::Review);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.tab(), BrainTab::Scorecard);
}

#[test]
fn enter_emits_navigation_without_mutating_decision() {
    let mut app = fixture_app_with_attention();
    let effect = app.handle_key(key(KeyCode::Enter));
    assert!(matches!(effect, Some(BrainEffect::SwitchToSession(_))));
    assert!(app.runtime().actions_log().is_empty());
}

#[test]
fn correction_records_right_wrong_or_exception() {
    let mut app = fixture_app_with_attention();
    app.begin_correction();
    app.choose_correction(CorrectionDisposition::BrainWrong, Some("safe fixture".into()));
    assert_eq!(app.runtime().corrections().len(), 1);
}
```

Run:

```bash
cargo test -p codexctl-tui brain_app::tests
```

Expected: FAIL because `BrainApp` does not exist.

- [ ] **Step 5: Implement BrainApp and the three render modules**

`BrainApp` owns only tab, selection, input, status, refresh deadline, and the
latest core DTOs. It refreshes every second or on `r`. `j`/`k` move selection,
`Tab` cycles tabs, `Enter` returns navigation, `c` opens the right/wrong/exception
correction flow with an optional bounded note, and `q` requests exit. Review
keeps canonical mark/note/skip behavior; Scorecard is read-only.

Render the approved hierarchy:

```text
 Coding Brain | BRAIN ACTIVE | advisory | model
 [ Live ]  Review  Scorecard

 Needs Attention
 Recent
 Decision
```

Offline endpoint health changes the status label and adds setup guidance but
does not hide persisted Live, Review, or Scorecard data.
Delivery-failed and delivery-unknown rows use distinct status text; only joined
lifecycle/outcome evidence may render an executed/completed label.

- [ ] **Step 6: Add Ratatui buffer assertions**

Use `ratatui::TestBackend` to assert the Live default, attention count,
collapsed occurrence count, overflow indicator, detail evidence, empty state,
offline state, Review queue, Scorecard guardrails, active-tab styling, and
correction prompt. Include committed/delivered, delivery-failed,
delivery-unknown, and outcome-confirmed fixtures. Assert rendered buffers
contain no session table header, health grid, send/terminate/route/spawn hint,
or hidden dashboard copy.

- [ ] **Step 7: Verify standalone layering and finish the changeset**

```bash
cargo fmt --all --check
cargo test -p codexctl-core runtime::tests
cargo test -p codexctl-tui
cargo clippy -p codexctl-tui --all-targets -- -D warnings
cargo build -p codexctl-tui
jj --no-pager st
```

Expected: `codexctl-tui` builds and tests without the binary crate while the old
dashboard still compiles internally. Close `codexctl-662.4` with the evidence.

---

### Task 5: Add Explicit Optional Agent Deck Navigation (`codexctl-662.5`)

**Files:**

- Modify: `crates/codexctl-core/src/brain_activity.rs`
- Modify: `crates/codexctl-core/src/runtime.rs`
- Create: `src/runtime/navigation.rs`
- Modify: `src/runtime/mod.rs`
- Create: `crates/codexctl-tui/src/terminal_suspend.rs`
- Modify: `crates/codexctl-tui/src/brain_app.rs`
- Modify: `crates/codexctl-tui/src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `crates/codexctl-tui/Cargo.toml`
- Test: inline tests in `src/runtime/navigation.rs` and
  `crates/codexctl-tui/src/terminal_suspend.rs`
- Test: `tests/agent_deck_navigation.rs`

**Interfaces:**

- Extends: `BrainRuntime` with `Arc<dyn SessionNavigation>`.
- Produces: `SessionNavigation::resolve(&SessionTarget) ->
  Result<NavigationPlan, NavigationError>` and
  `SessionNavigation::focus_fallback(&SessionTarget) -> Result<(), String>`.
- Produces: `NavigationPlan::External(ExternalCommand)` where
  `ExternalCommand { program: PathBuf, args: Vec<OsString> }` has no shell
  string representation.
- Produces: `TerminalSuspendGuard::{suspend, run_external, restore}` with
  idempotent restoration from `Drop`.

**Acceptance Criteria:**

- Agent Deck is optional, never queried at startup, and exact, missing, and
  ambiguous matches are tested. Unknown JSON fields are ignored; missing
  required identity fields make navigation unavailable.
- Attach uses `agent-deck session attach` with an argument vector, no shell
  interpretation, and no tmux internals.
- Terminal state, active tab, and selection restore after success, nonzero
  exit, spawn failure, unwind, and handled termination. User cancellation never
  invokes fallback.
- Failure is nonfatal and at most one terminal-focus fallback is attempted.

- [ ] **Step 1: Claim the task and start a described changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.5 --claim
jj new -m "🧭 feat: add optional Agent Deck navigation (codexctl-662.5)"
```

- [ ] **Step 2: Add navigation-resolution red tests**

Use a fake executable whose path is injected into `LiveSessionNavigation`:

```rust
#[test]
fn exact_session_id_builds_attach_argv() {
    let navigator = fixture_navigator(agent_deck_json(&[
        session("deck-1", "project-a", "/work/project-a"),
    ]));
    let plan = navigator.resolve(&target("deck-1", "/work/project-a")).unwrap();
    assert_eq!(
        plan,
        NavigationPlan::External(ExternalCommand::new(
            "agent-deck",
            ["session", "attach", "deck-1"],
        ))
    );
}

#[test]
fn ambiguous_path_is_an_error_not_a_guess() {
    let navigator = fixture_navigator(agent_deck_json(&[
        session("deck-1", "one", "/work/project-a"),
        session("deck-2", "two", "/work/project-a"),
    ]));
    assert!(matches!(
        navigator.resolve(&target("unknown", "/work/project-a")),
        Err(NavigationError::Ambiguous { .. })
    ));
}

#[test]
fn resolver_is_lazy() {
    let navigator = counting_navigator();
    let _runtime = BrainRuntime::with_navigation(navigator.clone());
    assert_eq!(navigator.invocations(), 0);
}
```

Cover exact stable ID first, then exact normalized cwd plus title/provider hints;
never pick the first fuzzy match. Cap `agent-deck list --json` output at 1 MiB
and its query at two seconds. Missing executable, nonzero exit, malformed JSON,
missing required fields, no match, and ambiguous match are typed nonfatal
errors. Deserialize only required fields and ignore additive schema fields.

Run:

```bash
cargo test --lib runtime::navigation::tests
```

Expected: FAIL until the adapter exists.

- [ ] **Step 3: Implement public-CLI-only Agent Deck resolution**

Invoke exactly:

```rust
Command::new(&self.agent_deck)
    .args(["list", "--json"])
```

After one exact match, return only this external command:

```rust
ExternalCommand::new(
    self.agent_deck.clone(),
    [OsString::from("session"), OsString::from("attach"), matched_id.into()],
)
```

Do not inspect `$TMUX`, run `tmux`, switch clients, synthesize `Enter`, or add
nested-session rules. `focus_fallback` may resolve the opaque target to an
internal live `CodexSession` and call the existing
`terminals::switch_to_terminal` once.

- [ ] **Step 4: Add terminal lifecycle red tests**

Separate terminal operations behind a test double and cover each exit path:

```rust
#[test]
fn normal_child_exit_restores_terminal_once() {
    let terminal = FakeTerminal::default();
    let child = FakeChild::exit_code(0);
    run_suspended(&terminal, child).unwrap();
    assert_eq!(terminal.calls(), ["leave_alt", "raw_off", "raw_on", "enter_alt", "redraw"]);
}

#[test]
fn panic_path_restores_through_drop() {
    let terminal = FakeTerminal::default();
    let result = std::panic::catch_unwind(|| {
        let _guard = TerminalSuspendGuard::suspend(&terminal).unwrap();
        panic!("fixture");
    });
    assert!(result.is_err());
    assert!(terminal.is_restored());
}
```

Add nonzero exit, spawn failure, idempotent explicit restore plus `Drop`, and a
handled termination flag. Install the parent Ctrl-C handler once and have it set
an atomic cancellation flag while an external child owns the terminal. The
executed child retains the default interrupt disposition, inherits
stdin/stdout/stderr, and exits on Ctrl-C; the parent waits, restores the terminal,
clears the attachment state, and reports `NavigationOutcome::Cancelled`.

- [ ] **Step 5: Wire `Enter` through suspend, attach, and one fallback**

The outer TUI loop consumes `BrainEffect::SwitchToSession`. Resolve lazily; if
an external command is returned, save current tab and selection, suspend raw
mode and the alternate screen, spawn/wait with inherited standard streams, and
restore before refresh. On resolve or attach failure, call `focus_fallback`
once. On `NavigationOutcome::Cancelled`, restore and refresh Brain without
fallback. Report a bounded status error if both ordinary navigation paths fail.
Never change an activity, decision, or correction record.

```rust
match effect {
    BrainEffect::SwitchToSession(target) => {
        let result = navigation.resolve(&target).and_then(|command| {
            TerminalSuspendGuard::run(&terminal, command)
        });
        match result {
            Ok(NavigationOutcome::Attached | NavigationOutcome::Cancelled) => {}
            Err(_) => navigation.focus_fallback(&target)?,
        }
        app.refresh(runtime.snapshot()?);
    }
}
```

- [ ] **Step 6: Verify with a fake Agent Deck executable**

`tests/agent_deck_navigation.rs` creates a fixture executable that returns JSON,
captures exact argv, blocks until released, exits nonzero, or fails to spawn.
Assert additive schema fields are ignored, missing identity fields fail
nonfatally, optional absence does not affect startup, cancellation attempts no
fallback, every other failing path attempts at most one fallback, and every path
restores terminal state, tab, selection, and cursor visibility.

Run:

```bash
cargo test --lib runtime::navigation::tests
cargo test -p codexctl-tui terminal_suspend::tests
cargo test --test agent_deck_navigation
cargo clippy --workspace --all-targets -- -D warnings
jj --no-pager st
```

Expected: all attach and restoration cases pass. Close `codexctl-662.5` with
the evidence.

---

### Task 6: Make Brain the Default TUI and Preserve Headless Evaluation (`codexctl-662.6`)

**Files:**

- Modify: `src/main.rs`
- Modify: `src/commands.rs`
- Modify: `src/runtime/mod.rs`
- Modify: `src/runtime/brain.rs`
- Modify: `crates/codexctl-tui/src/brain_app.rs`
- Modify: `crates/codexctl-tui/src/lib.rs`
- Test: inline tests in `src/main.rs` and `src/commands.rs`
- Test: `tests/brain_tui_smoke.rs`
- Test: `tests/headless_activity.rs`

**Interfaces:**

- Consumes: `runtime::build_brain_runtime(paths, config) -> BrainRuntime` from
  Tasks 4 and 5.
- Produces: `run_brain_tui(terminal, BrainApp, Duration) -> io::Result<()>` as
  the sole default interactive path.
- Produces: `run_headless(config, paths, json) -> io::Result<()>` emitting
  normalized activity updates rather than a session roster.
- Preserves: hidden permission/lifecycle/distill entry points and approved
  Brain analysis/setup CLI dispatch.

**Acceptance Criteria:**

- Running the current binary opens `BrainApp` on Live with no hidden dashboard
  mode.
- Closing the TUI does not stop hook evaluation; offline endpoint state leaves
  review and scorecard usable.
- `--headless` retains JSON machine output, activity recording, outcome
  handling, and learning catch-up without exposing session-list output.
- Internal Codex transcript discovery remains available only to evaluation and
  navigation adapters.

- [ ] **Step 1: Claim the task and start a described changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.6 --claim
jj new -m "🔀 refactor: make Brain the default runtime (codexctl-662.6)"
```

- [ ] **Step 2: Add default-dispatch and offline red tests**

Factor CLI dispatch so tests can observe the selected mode without entering a
real terminal:

```rust
#[test]
fn no_mode_selects_brain_tui() {
    let cli = Cli::try_parse_from(["codexctl"]).unwrap();
    assert_eq!(select_mode(&cli), RunMode::BrainTui);
}

#[test]
fn headless_is_the_only_continuous_non_tui_mode() {
    let cli = Cli::try_parse_from(["codexctl", "--headless", "--json"]).unwrap();
    assert_eq!(select_mode(&cli), RunMode::Headless { json: true });
}

#[test]
fn unreachable_endpoint_still_builds_read_only_app() {
    let app = build_fixture_app(EndpointHealth::Offline("connection refused".into()));
    assert!(app.is_read_only());
    assert_eq!(app.tab(), BrainTab::Live);
    assert!(!app.review().is_empty());
}
```

Run:

```bash
cargo test --bin codexctl default_brain_cli_tests
cargo test -p codexctl-tui brain_app::tests
```

Expected: FAIL while the dashboard is still the default.

- [ ] **Step 3: Replace default TUI wiring without deleting old modules**

Build paths and config once, run distillation catch-up, construct the live
Brain runtime, and start `BrainApp`. The loop draws the new root UI, polls
Crossterm events, refreshes at the fixed one-second internal interval, executes
navigation effects outside the app, and restores terminal state on every exit.
Do not pass a session list, filters, budget, rules, mailbox, or dashboard flags
to the new app.

```rust
loop {
    terminal.draw(|frame| app.render(frame))?;
    if let Some(effect) = app.handle_event(events.next_or_refresh()?)? {
        run_brain_effect(effect, &mut app, &runtime, &terminal)?;
    }
    if app.should_quit() {
        break;
    }
}
```

- [ ] **Step 4: Rework headless output around activity**

Headless may scan Codex transcripts internally, reconcile lifecycle evidence,
and evaluate pending supported requests, but its public JSON is an envelope of
normalized activity changes:

```json
{"type":"activity","activity_id":"act_...","state":"denied","project_id":{"kind":"stable","value":"..."}}
```

It must not emit a session array, general roster, watch stream, terminal target,
or command not already normalized and redacted for activity. Startup calls the
same distillation catch-up and maintenance compaction used by the TUI;
compaction skips when its lock is busy.

- [ ] **Step 5: Add process smoke tests**

`tests/brain_tui_smoke.rs` uses a test backend/runtime to assert default Live,
offline review access, refresh, and clean `q` exit. `tests/headless_activity.rs`
runs one bounded iteration with fixture transcript/activity stores and asserts
valid JSON activity, no `sessions` key, no daemon child, and no dependence on an
open TUI.

- [ ] **Step 6: Verify the convergence point**

```bash
cargo fmt --all --check
cargo test --bin codexctl default_brain_cli_tests
cargo test --test brain_tui_smoke
cargo test --test headless_activity
cargo test -p codexctl-tui
cargo clippy --workspace --all-targets -- -D warnings
jj --no-pager st
```

Expected: BrainApp is the current binary's default, headless emits only activity,
and the old dashboard still compiles but is unreachable. Close
`codexctl-662.6` with the evidence.

---

### Task 7: Remove Dashboard, Mailbox, and Session Management (`codexctl-662.7`)

**Files:**

- Delete: `crates/codexctl-tui/src/app.rs`
- Delete: `crates/codexctl-tui/src/demo.rs`
- Delete: `crates/codexctl-tui/src/recorder.rs`
- Delete: `crates/codexctl-tui/src/session_recorder.rs`
- Delete: `crates/codexctl-tui/src/ui/detail.rs`
- Delete: `crates/codexctl-tui/src/ui/help.rs`
- Delete: `crates/codexctl-tui/src/ui/skills.rs`
- Delete: `crates/codexctl-tui/src/ui/status_bar.rs`
- Delete: `crates/codexctl-tui/src/ui/table.rs`
- Delete: `scripts/record-demos.sh`
- Delete: `src/brain_screen.rs`
- Delete: `src/brain/mailbox.rs`
- Delete: `src/brain/engine.rs`
- Delete: `src/runtime/actions.rs`
- Delete: `src/runtime/brain_driver.rs`
- Delete: `src/runtime/brain_review.rs`
- Delete: `src/runtime/delivery.rs`
- Delete: `src/runtime/sessions.rs`
- Modify: `crates/codexctl-tui/src/{lib,ui/mod}.rs`
- Modify: `crates/codexctl-tui/Cargo.toml`
- Modify: `crates/codexctl-core/src/{lib,runtime,rules,terminals/mod}.rs`
- Modify: `src/{main,commands,config}.rs`
- Modify: `src/brain/{client,decisions,mod}.rs`
- Modify: `src/runtime/mod.rs`
- Modify: `src/init/{mod,phases,state}.rs`
- Test: CLI/config tests inline in `src/main.rs` and `src/config.rs`
- Test: `tests/removed_surfaces.rs`

**Interfaces:**

- Retains: Brain evaluation, prompts, metrics, review, outcomes, baseline,
  insights, garden, briefing, autopsy, mode, canonical marking, headless,
  internal hooks, init, doctor, config, hook inspection, completions, man, and
  diagnostics.
- Removes: public `SessionSource`, `Actions`, `BrainDriver`, `BrainDelivery`,
  legacy `Runtime`, and `DecisionScope::Orchestration`.
- Shrinks: suggestion/rule actions to approve and deny; terminal switching
  remains reachable only through the opaque navigation adapter.
- Shrinks: configuration to Brain, TUI theme, outcome test runners, and retained
  hook/config diagnostics; project endpoint values are warnings and ignored.

**Acceptance Criteria:**

- Removed flags are absent from help and fail Clap parsing.
- Removed config keys and sections produce explicit unsupported-setting
  warnings.
- No public runtime API exposes send, terminate, route, spawn, mailbox,
  dashboard, or a general session list.
- Retained Brain commands and internal transcript-dependent features pass
  characterization tests.

- [ ] **Step 1: Claim the task and start a described changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.7 --claim
jj new -m "🧹 refactor: remove dashboard session management (codexctl-662.7)"
```

- [ ] **Step 2: Add the retained/removed CLI contract tests first**

Use complete argument tables in `src/main.rs`:

```rust
const REMOVED_ARGS: &[&str] = &[
    "--interval", "--debug", "--demo", "--list", "--watch", "--summary",
    "--since", "--filter-status", "--focus", "--search", "--new", "--cwd",
    "--prompt", "--resume", "--budget", "--kill-on-budget", "--notify",
    "--webhook", "--webhook-on", "--terminal-auto-approve-fallback",
    "--record", "--duration", "--clean", "--older-than", "--finished",
    "--history", "--stats", "--scope", "--init", "--uninstall", "--doctor",
];

const RETAINED_ARGS: &[&str] = &[
    "--headless", "--json", "--theme", "--brain", "--auto-run", "--url", "--brain-model",
    "--brain-eval", "--brain-prompts", "--brain-stats", "--brain-review",
    "--brain-mark-canonical", "--brain-query", "--tool", "--tool-input",
    "--project", "--mode", "--record-outcome", "--exit-code", "--duration-ms",
    "--stderr-tail", "--session-id", "--tool-use-id", "--reap-outcomes",
    "--brain-outcomes", "--brain-baseline", "--top", "--insights",
    "--brain-garden", "--apply", "--brain-briefing", "--autopsy", "--session",
    "--config", "--config-template", "--config-validate", "--config-init",
    "--hooks", "--log",
];

#[test]
fn removed_args_fail_and_retained_args_are_in_long_help() {
    let help = Cli::command().render_long_help().to_string();
    for arg in REMOVED_ARGS {
        assert!(Cli::try_parse_from(["codexctl", arg]).is_err(), "{arg}");
        assert!(!help.contains(arg), "{arg}");
    }
    for arg in RETAINED_ARGS {
        assert!(help.contains(arg), "{arg}");
    }
}
```

Subcommands `init`, `doctor`, `completions`, and `man` remain. Legacy flag forms
of init/doctor/uninstall are removed immediately rather than deprecated.

- [ ] **Step 3: Add explicit unsupported-config tests**

Warnings must name every removed surface instead of treating it as unknown:

```rust
#[test]
fn dashboard_and_management_config_are_explicitly_unsupported() {
    let warnings = validate_fixture(
        r#"
interval = 2000
notify = true
budget = 10
[webhook]
url = "https://example.invalid"
[brain]
max_sessions = 4
orchestrate = true
terminal_auto_approve_fallback = true
[lifecycle]
auto_restart = true
"#,
    );
    assert!(warnings.iter().all(|warning| warning.message.contains("no longer supported")));
}

#[test]
fn project_endpoint_is_ignored_with_a_source_warning() {
    let loaded = load_layers(user_config("endpoint = \"http://127.0.0.1:1\""),
                             project_config("endpoint = \"https://remote.invalid\""));
    assert_eq!(loaded.config.brain.unwrap().endpoint, "http://127.0.0.1:1");
    assert!(loaded.warnings.iter().any(|w| w.message.contains("project configuration cannot select brain.endpoint")));
}
```

Remove budget, notification, webhook, filters, model-price/health dashboard,
orchestration, lifecycle restart, max-session, terminal fallback, and custom
dashboard-event hook fields from parsed config. Keep a source-aware
`apply(raw, ConfigSource)` so endpoint is accepted only from user config; the
CLI `--url` override remains highest priority.

- [ ] **Step 4: Remove old TUI and runtime contracts**

Delete the unreachable dashboard modules and their exports. Remove launch,
terminate, inject, approve, send, route, spawn, mailbox delivery, generic
session roster, orchestration decision scope, session recordings, dashboard
demos, cleanup, skills overlay, and dashboard help. Keep only terminal focus
needed by `runtime/navigation.rs`; make the concrete `CodexSession` resolution
private to that adapter.

Shrink the suggestion action parser to:

```rust
pub enum BrainAction {
    Approve,
    Deny,
}
```

Any other model action is malformed and abstains. Do not keep compatibility
variants that make removed actions reachable.

- [ ] **Step 5: Remove obsolete command and onboarding branches**

Delete list/JSON roster, watch, summary, history/statistics, launch/resume,
cleanup, dashboard recorder, budget, notification, webhook, terminal fallback,
and mailbox code from `main.rs` and `commands.rs`. Remove the onboarding budget
phase and its non-interactive flags. Retain Brain endpoint setup, managed Codex
hooks, and skill suggestions because they do not manage sessions.

Transcript discovery remains private support for autopsy, Brain evidence,
outcome attribution, headless evaluation, and navigation. It must not gain a
replacement list/watch command.

- [ ] **Step 6: Prove no public management surface remains**

`tests/removed_surfaces.rs` should combine Clap help assertions with compile-time
API usage of the final `BrainRuntime`. Add source guards:

```bash
rg -n "BrainDelivery|BrainDriver|SessionSource|terminate_session|inject_text|deliver_mailbox" src crates
rg -n "RuleAction::(Send|Terminate|Route|Spawn)|DecisionScope::Orchestration" src crates
```

Expected: no runtime occurrences remain. Historical changelog/spec text is not
part of this source guard.

- [ ] **Step 7: Verify retained Brain behavior and finish the changeset**

```bash
cargo fmt --all --check
cargo test --bin codexctl brain_only_cli_tests
cargo test --lib brain::query::tests
cargo test --lib brain::permission_hook::tests
cargo test --test removed_surfaces
cargo test -p codexctl-tui
cargo clippy --workspace --all-targets -- -D warnings
jj --no-pager st
```

Expected: removed surfaces fail parsing, retained Brain flows pass, and the
workspace contains no dashboard runtime. Close `codexctl-662.7` with the
evidence.

---

### Task 8: Rename Runtime Namespaces to Coding Brain (`codexctl-662.8`)

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/main.rs`
- Modify: `src/config.rs`
- Modify: `src/doctor.rs`
- Modify: `src/lifecycle_hook.rs`
- Modify: Brain runtime modules under `src/brain/`, including `activity.rs`,
  `autopsy.rs`, `briefing.rs`, `decisions.rs`, `detectors.rs`, `evals.rs`,
  `garden.rs`, `mod.rs`, `pref_store.rs`, `preferences.rs`, `prompts.rs`, and
  `review.rs`
- Modify: `src/init/{hooks,marker,mod,phases,state}.rs`
- Modify: `crates/codexctl-core/src/{config,lifecycle/store,runtime,terminals/mod}.rs`
- Modify: `crates/codexctl-tui/src/{brain_app,lib}.rs`
- Test: naming/path/purge tests inline in `src/main.rs`, `src/config.rs`,
  `src/init/mod.rs`, and `src/init/hooks.rs`
- Test: `tests/public_namespace.rs`

**Interfaces:**

- Produces: `[[bin]] name = "coding-brain"` while retaining Cargo package and
  Rust library/crate names `codexctl`.
- Produces: Clap command name `coding-brain`, generated hook commands using the
  running immutable executable, and completion/man names for `coding-brain`.
- Produces: explicit purge target resolution for current Coding Brain global
  config/state plus documented old codexctl global config/state.

**Acceptance Criteria:**

- Only `coding-brain` is installed and generated hooks invoke it.
- Runtime never reads `.codexctl.toml`, `~/.config/codexctl`, or `~/.codexctl`;
  normal startup leaves them untouched.
- Explicit confirmed `init --purge` removes only documented current and legacy
  global targets and never project files.
- `coding-brain init` atomically replaces exact managed legacy hook entries;
  normal startup and doctor only diagnose stale entries.
- Non-loopback and plaintext-HTTP endpoint warnings are visible without
  overriding the user's CLI/user-config choice.
- CLI, config, hook, doctor, namespace, and purge tests pass.

- [ ] **Step 1: Claim the task and start the final described changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.8 --claim
jj new -m "🏷️ feat: rename Coding Brain runtime namespaces (codexctl-662.8)"
```

This task runs after the removed surfaces can no longer leak old product copy.
Distribution and documentation follow in Task 9. Do not release or publish any
earlier changeset.

- [ ] **Step 2: Add public-name and no-legacy-read red tests**

```rust
#[test]
fn clap_and_cargo_expose_only_coding_brain() {
    assert_eq!(Cli::command().get_name(), "coding-brain");
    assert_eq!(env!("CARGO_BIN_NAME"), "coding-brain");
}

#[test]
fn old_config_and_state_are_ignored_and_untouched() {
    let fixture = NamespaceFixture::new();
    fixture.write_old_config("[brain]\nenabled = true\n");
    fixture.write_old_state("brain/decisions.jsonl", "legacy\n");
    let before = fixture.legacy_digest();
    let loaded = Config::load_from(&fixture.environment(), fixture.cwd()).unwrap();
    assert!(loaded.brain.is_none());
    assert_eq!(fixture.legacy_digest(), before);
}

#[test]
fn project_config_cannot_redirect_endpoint() {
    let fixture = NamespaceFixture::new();
    fixture.write_user_config("[brain]\nendpoint = \"http://127.0.0.1:11434\"\n");
    fixture.write_project_config("[brain]\nendpoint = \"https://remote.invalid\"\n");
    let loaded = Config::load_from(&fixture.environment(), fixture.cwd()).unwrap();
    assert_eq!(loaded.brain.unwrap().endpoint, "http://127.0.0.1:11434");
}
```

`tests/public_namespace.rs` must run the built binary with isolated XDG config
and state roots, create sentinel old files, exercise help/config/doctor and one
hook fallthrough, then prove no old file was read, changed, or removed.
Add fixtures for stale exact managed hooks, unrelated hook entries, malformed
project manifests, remote HTTPS, plaintext remote HTTP, and a simulated crash
before hook-file replacement.

- [ ] **Step 3: Rename the binary, paths, hooks, and operator copy**

Change only the root `[[bin]]` name; keep package, lib, and internal crate names.
Update Clap metadata, diagnostics, init copy, hook definitions/discovery,
completion/man generation, first-run marker, prompts, evals, decisions,
preferences, canonical records, outcomes, lifecycle snapshot, and logs to use
Task 1 `CodingBrainPaths`. Rename the public first-run override to
`CODING_BRAIN_SKIP_FIRST_RUN`; removed dashboard-only environment variables get
no replacements. Test-only `CODEXCTL_*` variables inside internal crate fixtures
may retain their names because they are not public product contracts.

Managed-hook cleanup recognizes exact old `codexctl --permission-hook` and
`codexctl --lifecycle-hook` entries only when the operator runs init/remove or
purge. `coding-brain init` writes a complete sibling hook file, flushes it, and
atomically replaces the original while preserving every unrelated entry. A
failed pre-rename write leaves the original hook file intact. Normal startup
and doctor may diagnose exact stale managed hooks but never modify them;
ordinary config/state loading never scans old namespaces. `coding-brain init`
writes only new hook commands.

Non-loopback endpoint warnings remain visible in TUI and doctor, while explicit
CLI or user config selection keeps automatic decisions available. Project
config endpoint attempts warn and are ignored. Plaintext non-loopback HTTP uses
a stronger warning than remote HTTPS. Doctor reports missing/malformed project
identity and documents that removing `.coding-brain/project.toml` before
rerunning init deliberately creates a new identity; it never compares Git paths
or remotes.

- [ ] **Step 4: Implement exact purge semantics**

Resolve and print these global targets before confirmation:

1. current `$XDG_STATE_HOME/coding-brain/` or fallback;
2. current `$XDG_CONFIG_HOME/coding-brain/config.toml` or fallback;
3. old `~/.codexctl/`;
4. old `~/.config/codexctl/config.toml`;
5. exact managed `coding-brain` and old `codexctl` hook entries plus the current
   onboarding marker.

Accept only absolute, non-root HOME/XDG bases. Join only the fixed children
above and lexically reject empty, root, relative, `.` or `..`-escaping targets.
Preview each target's exact path and file type. After explicit confirmation or
`--yes`, resolve no environment variables again: re-check the previewed target
with `symlink_metadata`, refuse it if its identity/type changed, unlink a
symlink itself without following it, and otherwise delete only that exact
target. Use injected path fixtures in tests; never derive a recursive target
from an unset environment variable. Assert `.coding-brain.toml`,
`.coding-brain/project.toml`, unrelated hook entries, and sibling XDG files
survive. Add root, relative-base, symlink, changed-after-preview, and interrupted
hook-rewrite cases.

- [ ] **Step 5: Run focused runtime namespace verification**

```bash
cargo fmt --all --check
cargo test --test public_namespace
cargo test --bin coding-brain
cargo test --lib config::tests
cargo test --lib init::tests
cargo test --lib init::hooks::tests
cargo test --lib doctor::tests
cargo test --test public_namespace stale_hooks_are_diagnostic_until_init
cargo test --test public_namespace purge_rejects_unsafe_or_changed_targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build --bin coding-brain
cargo metadata --no-deps --format-version 1 | jq -e \
  '[.packages[] | select(.name == "codexctl") | .targets[] | select(.kind | index("bin")) | .name] == ["coding-brain"]'
jj --no-pager st
```

Expected: the runtime exposes only `coding-brain`, uses only Coding Brain paths,
leaves legacy files untouched during ordinary operation, and removes only the
enumerated targets after confirmed purge. Close `codexctl-662.8` with the
evidence.

---

### Task 9: Update Coding Brain Distribution and Documentation (`codexctl-662.9`)

**Files:**

- Modify: `nix/home-manager.nix`
- Modify: `nix/tests/home-manager-module.nix`
- Modify: `flake.nix`
- Modify: `install.sh`
- Modify: `.github/workflows/release.yml`
- Modify: `scripts/render-homebrew-formula.sh`
- Modify: `scripts/render-aur-bin-files.sh`
- Delete: `packaging/homebrew-core/codexctl.rb`
- Create: `packaging/homebrew-core/coding-brain.rb`
- Delete: `packaging/aur/codexctl-bin/PKGBUILD`
- Create: `packaging/aur/coding-brain-bin/PKGBUILD`
- Modify: packaging READMEs under `packaging/`
- Modify: `README.md`
- Modify: `CHANGELOG.md`
- Modify: `mkdocs.yml`
- Modify: `.github/RELEASE_TEMPLATE.md`
- Modify: `LAUNCH_POSTS.md`
- Modify: `docs/{index,quickstart,configuration,reference,terminal-support,troubleshooting,contributing,llms.txt}`
- Modify: `docs/decisions/ADR-0001-lifecycle-hooks-as-status-evidence.md`
- Modify: `justfile`

**Interfaces:**

- Produces: Home Manager option `programs.coding-brain`, XDG config target
  `coding-brain/config.toml`, package `mainProgram = "coding-brain"`, and no
  `programs.codexctl` alias.
- Produces: release archives named
  `coding-brain-<tag>-<target>.tar.gz` containing only `coding-brain`.
- Produces: Homebrew formula `coding-brain`, AUR package `coding-brain-bin`, and
  installer/release copy that invokes `coding-brain`.
- Documents: Brain-only product boundary, optional Agent Deck navigation,
  hook-first runtime, current XDG/project paths, explicit install/init/doctor
  cutover, project-identity reset, rollback, and manual pre-release purge.

**Acceptance Criteria:**

- Home Manager, Nix, installer, packaging, and release artifacts expose
  `coding-brain` with no `codexctl` compatibility alias.
- Current README/docs teach the Brain product boundary, optional Agent Deck,
  hook-first runtime, new paths, and manual pre-release reset.
- Historical ADR and changelog context remains accurate while current
  instructions contain no stale dashboard or old-path guidance.
- Cargo workspace, standalone crates, Nix, packaging renders, release archive
  smoke tests, and documentation checks pass.

- [ ] **Step 1: Claim the task and start the distribution changeset**

```bash
bd -C /home/alexander/.beads-planning update codexctl-662.9 --claim
jj new -m "📦 build: update Coding Brain distribution (codexctl-662.9)"
```

Task 8 must already be closed, so every package and document can inspect the
actual final CLI rather than predict it.

- [ ] **Step 2: Add failing distribution namespace checks**

Update `nix/tests/home-manager-module.nix` to instantiate only the new option and
assert the new generated path:

```nix
testPackage = pkgs.writeShellScriptBin "coding-brain" "exit 0";

programs.coding-brain = {
  enable = true;
  package = testPackage;
};

assert lib.hasAttrByPath [
  "xdg"
  "configFile"
  "coding-brain/config.toml"
] configured.config;
assert lib.getExe testPackage == "${testPackage}/bin/coding-brain";
```

Add renderer and release smoke assertions that initially fail while old names
remain:

```bash
rg -n "coding-brain" packaging scripts install.sh .github/workflows/release.yml
if rg -n "programs\.codexctl|bin/codexctl|codexctl-bin|codexctl-.*\.tar\.gz" \
  nix packaging scripts install.sh .github/workflows/release.yml; then
  exit 1
fi
```

Scope that guard to current distribution files; historical changelog and ADR
references are intentionally excluded.

- [ ] **Step 3: Rename Nix, packaging, installer, and release artifacts**

Change the Home Manager option to `programs.coding-brain`, write
`coding-brain/config.toml`, and keep the flake input and repository URL
unchanged. Set Nix `pname` and `mainProgram` to `coding-brain`. Update module
assertions and trust notices without adding an option alias.

Release builds package the `coding-brain` executable in
`coding-brain-<tag>-<target>.tar.gz`. Update install script, Homebrew renderer,
`packaging/homebrew-core/coding-brain.rb`, AUR `coding-brain-bin`, checksum
paths, and smoke tests to install and invoke `coding-brain`. The crates.io
package remains `codexctl`, so installation instructions must distinguish the
package name from the installed executable:

```bash
cargo install codexctl
coding-brain init
coding-brain doctor
coding-brain
```

- [ ] **Step 4: Rewrite current documentation and record the break**

Lead README and docs with Coding Brain's judgment and learning boundary, the
Live/Review/Scorecard views, optional Agent Deck navigation, hook-first
operation, current paths, and the no-migration reset. Remove current
dashboard/session-management, mailbox, recording, cleanup, budget,
notification, and legacy-path instructions.

Document the exact cutover sequence:

```bash
cargo install codexctl
coding-brain init
coding-brain doctor
# Restart Codex after doctor reports the new managed hooks.
```

Explain that normal startup only diagnoses stale hooks, old state remains
available for rollback, and confirmed purge is irreversible. Before purge, the
rollback path is reinstalling the old build and rerunning its init. For a fork
that should learn independently, remove `.coding-brain/project.toml` and rerun
`coding-brain init`; never suggest editing the UUID by hand.

Keep historical changelog entries intact. Add a top breaking release entry that
names the executable/path change and manual reset. Annotate ADR-0001's
legacy-path sentence as superseded by ADR-0002 without changing its remaining
accepted decision. Keep the repository URL and Rust crate references as
`codexctl` only where they describe those internal identities.

Run targeted prose guards after the rewrite:

```bash
current_docs=(
  README.md mkdocs.yml docs/index.md docs/quickstart.md docs/configuration.md
  docs/reference.md docs/terminal-support.md docs/troubleshooting.md
  docs/contributing.md docs/llms.txt
)
rg -n "Coding Brain|Live|Review|Scorecard|Agent Deck|coding-brain/config.toml" \
  "${current_docs[@]}"
if rg -n "codexctl dashboard|codexctl list|codexctl watch|\.codexctl\.toml|\.config/codexctl" \
  "${current_docs[@]}"; then
  exit 1
fi
```

- [ ] **Step 5: Run distribution and documentation verification**

Use a temporary render directory created by the shell rather than a persistent
workspace path:

```bash
render_dir="$(mktemp -d)"
./scripts/render-homebrew-formula.sh 0.0.0 test test test test \
  > "$render_dir/coding-brain.rb"
./scripts/render-aur-bin-files.sh 0.0.0 test test "$render_dir"
nix build
nix build .#checks.x86_64-linux.home-manager-module
nix flake check
just check
rg -n "coding-brain" "$render_dir" packaging install.sh .github/workflows/release.yml
```

Expected: generated package definitions name and invoke only `coding-brain`, the
Home Manager test realizes the new config path, and documentation guards find no
current legacy instructions. Remove only `"$render_dir"` after inspecting its
contents.

- [ ] **Step 6: Run final workspace and release gates**

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo build --release
cargo build -p codexctl-core
cargo test -p codexctl-core
cargo build -p codexctl-tui
cargo test -p codexctl-tui
cargo test --test activity_scale hundred_thousand_events_preserve_retention
cargo test --test distill_process hundred_thousand_decisions_publish_one_generation
cargo test --release --test activity_scale release_activity_budgets -- --ignored --test-threads=1
cargo test --release --test distill_process release_distill_budget -- --ignored --test-threads=1
nix flake check
target/release/coding-brain --help
cargo metadata --no-deps --format-version 1 | jq -e \
  '[.packages[] | select(.name == "codexctl") | .targets[] | select(.kind | index("bin")) | .name] == ["coding-brain"]'
smoke_dir="$(mktemp -d)"
cp target/release/coding-brain "$smoke_dir/coding-brain"
tar -C "$smoke_dir" -czf "$smoke_dir/coding-brain-v0.0.0-smoke.tar.gz" coding-brain
test "$(tar -tzf "$smoke_dir/coding-brain-v0.0.0-smoke.tar.gz")" = coding-brain
jj --no-pager st
```

Expected: every command succeeds. `target/release/coding-brain --help` names
Coding Brain, Cargo metadata exposes no `codexctl` binary target, and a clean
release archive contains no old executable. Close
`codexctl-662.9`, then close `codexctl-662` only after every child task is closed
and the full evidence is recorded. Do not push or publish without separate user
authority. Remove only the two temporary directories created by Steps 5 and 6
after their contents have been inspected.

## Stress Test Results: Coding Brain Implementation Plan

### Resolved Decisions

- `activity.jsonl` is the authoritative decision-commit/lifecycle audit;
  `decisions.jsonl` stores proposals and learning evidence. Model actions
  require both writes, and separate delivery/outcome evidence prevents a
  committed decision from being mislabeled as executed.
- Execute all nine tasks serially because adjacent tasks share persistence,
  runtime, and module-registration files.
- Treat a valid tracked project UUID as authoritative across clones, worktrees,
  and forks; identity reset is explicit and never inferred.
- Repair a crash-truncated JSONL tail under the activity lock before accepting
  another append.
- Publish preferences as immutable complete generations selected by one atomic
  watermark/current pointer.
- Keep Agent Deck schema-tolerant and optional; user cancellation restores Brain
  without fallback.
- Separate deterministic correctness/retention coverage from explicit
  single-threaded release timing gates.
- Validate purge targets lexically and by file identity, and never follow a
  symlink.
- Preserve unrestricted user endpoint choice while redacting and bounding
  payloads, moving request bodies off argv, disabling redirects, and warning
  visibly.
- Keep the no-alias/no-migration cutover, with atomic managed-hook replacement
  and legacy data retained until separately confirmed purge.

### Changes Made

- Serialized the Beads task graph from Task 1 through Task 9.
- Added dual-store ordering and failure-boundary tests.
- Added JSONL tail repair and killed-writer coverage.
- Replaced multi-file in-place preference publication with generation-based
  atomic publication.
- Added Agent Deck cancellation and schema-evolution coverage.
- Added normal-profile scale correctness tests and final ignored release-budget
  gates.
- Hardened purge, model transport, endpoint warnings, cutover, rollback, and
  project-identity documentation requirements.
- Added explicit delivered, delivery-failed, and delivery-unknown semantics plus
  later lifecycle/outcome confirmation.

### Deferred / Parking Lot

- Dream memory/reflection remains a future feature built on stable project
  identity and the typed extension seam.
- Permission decisions beyond Bash remain separate work under `codexctl-85x`.
- Agent Deck remains an optional external integration without tmux internals or
  a compatibility commitment to undocumented output.

### Confidence Assessment

- Overall: High; all 11 adversarial branches are resolved, pending
  implementation evidence.
- Areas of concern: hook response delivery cannot be transactional with two
  append-only stores and a stdout pipe, so projections and operator copy must
  preserve the committed/delivered/executed distinction.
