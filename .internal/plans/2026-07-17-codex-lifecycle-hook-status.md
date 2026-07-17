# Codex Lifecycle Hook Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use beads-superpowers:subagent-driven-development (recommended) or beads-superpowers:executing-plans to implement this plan task-by-task. Each Task becomes a bead (`bd create -t task --parent codexctl-rqm`). Steps within tasks use checkbox (`- [ ]`) syntax for human readability.

**Goal:** Consume documented Codex lifecycle hooks as an immediate, bounded status overlay while preserving transcript telemetry, process liveness, and every existing approval safety boundary.

**Architecture:** `codexctl-core` gains a focused lifecycle subsystem with validated event types, a pure ordered projection, a locked atomic snapshot store, and reconciliation helpers. The binary supplies two stdin adapters: a generic lifecycle handler and the existing permission handler, which observes every permission request but continues to authorize Bash only. The TUI reads one snapshot per refresh, binds only strongly matched live sessions, and lets explicit input, newer transcript semantics, and process death override hook evidence.

**Tech Stack:** Rust 2024 (MSRV 1.88), Serde JSON, `fs2` advisory locks, `tempfile` atomic persistence, Cargo tests, Ratatui, Nix/Home Manager evaluation, Markdown documentation, Jujutsu, Beads.

## Global Constraints

- Bead: `codexctl-rqm`; approved design: `.internal/specs/2026-07-17-codex-lifecycle-hook-status-design.md`.
- Keep production state under `~/.codexctl/hooks/lifecycle.json` and `lifecycle.lock`; `codexctl-2yk` owns the later XDG migration.
- Production callers use one core-owned compatibility root resolver; tests inject `LifecycleStore::at(root)` or a temporary `HOME`.
- Hook input is capped at 64 KiB; session, turn, and agent ids at 512 UTF-8 bytes; cwd and transcript paths at 4 KiB.
- Snapshot limits are 128 sessions, 32 recent closed/superseded turns per session, 64 active subagents per session, 1 MiB serialized size, and 24-hour retention.
- Hooks state directories are `0700` and files are `0600` on Unix. No prompt, command, tool input, tool output, or raw rejected value is persisted or echoed.
- Writers wait at most 100 ms for the advisory lock. Generic lifecycle handlers have a 2-second Codex timeout; permission handling retains its 30-second timeout and 25-second inference clamp.
- Atomic replacement protects reader consistency, but lifecycle writes do not `fsync`; the store is derivative and consumers fall back safely after data loss.
- Newer schemas are read-only. Corrupt current-schema snapshots are quarantined as `lifecycle.json.corrupt-<timestamp>`, with at most three retained.
- Only `UserPromptSubmit` may supersede a different open turn. Other cross-turn events are ignored as ambiguous and diagnosed.
- Status precedence is: dead process; terminal-confirmed approval or transcript-explicit `request_user_input`; fresh applicable hook evidence; transcript task state; high CPU; legacy fallback.
- Lifecycle evidence must never populate pending tool identity, approval evidence, terminal targets, rule inputs, brain authorization inputs, or cause an external action.
- `PermissionRequest` matches every tool for status. Brain allow/deny remains Bash-only; non-Bash requests record `NeedsInput` and emit no decision. Broader authorization is `codexctl-85x`.
- Automated tests and smoke checks use temporary homes and must not modify live `~/.codex/hooks.json`, `~/.codexctl`, or a Home Manager generation.
- Start every task in a new jj changeset with the exact emoji conventional description shown below. Do not push.
- If automation advances the working copy to an undescribed child while a task is still in progress, apply that task's exact description before making further edits. Final verification counts and audits non-empty implementation changesets rather than assuming one changeset per task.

## Dependency Order

```text
Task 1 ─→ Task 2 ─┬─→ Task 3 ─→ Task 4 ─────────┐
                 └─→ Task 5 ─→ Task 6 ────────┼─→ Task 7
```

When execution creates the child Beads, add dependencies in dependent-first
order: `2 depends on 1`, `3 depends on 2`, `5 depends on 2`, `4 depends on 3`,
`6 depends on 5`, and `7 depends on 4 and 6`. In CLI form, each edge is
`bd dep add <dependent-task-id> <dependency-task-id>`.

Before any implementation edit, record the final reviewed-plan change id in
the parent Bead as `implementation-base`. Create all seven child task Beads,
claim them, add the dependency edges above, and run `bd lint` over the parent
and children. Use the recorded change id as the stable scope baseline through
handoff; never reconstruct it from an ancestor count.

---

### Task 1: Define Validated Lifecycle Events and Ordered Projection

**Files:**
- Create: `crates/codexctl-core/src/lifecycle/mod.rs`
- Create: `crates/codexctl-core/src/lifecycle/input.rs`
- Create: `crates/codexctl-core/src/lifecycle/projection.rs`
- Modify: `crates/codexctl-core/src/lib.rs:13-31`
- Create: `tests/fixtures/hooks/session-start.json`
- Create: `tests/fixtures/hooks/user-prompt-submit.json`
- Create: `tests/fixtures/hooks/pre-tool-use.json`
- Create: `tests/fixtures/hooks/post-tool-use.json`
- Create: `tests/fixtures/hooks/subagent-start.json`
- Create: `tests/fixtures/hooks/subagent-stop.json`
- Create: `tests/fixtures/hooks/stop.json`
- Create: `tests/fixtures/hooks/permission-request.json`

**Interfaces:**
- Produces: `LifecycleEvent::parse(&[u8]) -> Result<Self, LifecycleInputError>` for generic installed events.
- Produces: `LifecycleIdentity::try_new(...) -> Result<Self, LifecycleInputError>` as the only adapter-facing identity constructor; its fields remain private and bounded getters expose persisted values.
- Produces: `LifecycleEvent::permission(LifecycleIdentity, PermissionDisposition) -> Result<Self, LifecycleInputError>` for the permission adapter; this constructor requires a turn-bearing validated identity and never accepts raw tool input or a tool name.
- Produces: `LifecycleSnapshot::apply(event, received_at_ms) -> ApplyOutcome`, the only state-transition entry point used by the store.
- Produces: `ProjectedStatus::{Processing, NeedsInput, Idle}`, `LifecycleEventName`, `PermissionDisposition::{Decided, NeedsInput}`, `SessionLifecycleState`, and schema constants consumed by later tasks.

**Acceptance Criteria:**
- Sanitized official payload fixtures parse without retaining prompt, tool input, tool response, command, or model values.
- Unknown JSON fields are accepted, while wrong events, missing identity, oversized ids/paths, and missing event-specific fields are rejected.
- Event projection implements every approved event/status mapping, the 32-turn guard, the 64-subagent cap, duplicate idempotence, and strict cross-turn rules.
- `SessionStart` changes identity metadata only; `SubagentStop` never closes the parent turn; `Stop` closes only the matching current turn.
- The pure transition suite requires no filesystem, environment mutation, or wall clock.

- [ ] **Step 1: Start the task changeset**

```bash
jj new -m "✨ feat: model Codex lifecycle hook events (codexctl-rqm)"
jj --no-pager st
```

Expected: an empty working-copy changeset with the exact description.

- [ ] **Step 2: Add sanitized official payload fixtures**

Create the eight files with these exact payload shapes; values omitted from persistence are deliberately present in the fixtures:

```json
// session-start.json
{"session_id":"session-1","transcript_path":"/tmp/rollout-1.jsonl","cwd":"/work/codexctl","hook_event_name":"SessionStart","model":"gpt-test","permission_mode":"default","source":"startup"}
// user-prompt-submit.json
{"session_id":"session-1","turn_id":"turn-1","transcript_path":"/tmp/rollout-1.jsonl","cwd":"/work/codexctl","hook_event_name":"UserPromptSubmit","permission_mode":"default","prompt":"do not persist me"}
// pre-tool-use.json
{"session_id":"session-1","turn_id":"turn-1","transcript_path":"/tmp/rollout-1.jsonl","cwd":"/work/codexctl","hook_event_name":"PreToolUse","permission_mode":"default","tool_name":"Bash","tool_input":{"command":"do not persist me"},"tool_use_id":"call-1"}
// post-tool-use.json
{"session_id":"session-1","turn_id":"turn-1","transcript_path":"/tmp/rollout-1.jsonl","cwd":"/work/codexctl","hook_event_name":"PostToolUse","permission_mode":"default","tool_name":"Bash","tool_input":{"command":"do not persist me"},"tool_response":"do not persist me","tool_use_id":"call-1"}
// subagent-start.json
{"session_id":"session-1","turn_id":"turn-1","transcript_path":"/tmp/rollout-1.jsonl","cwd":"/work/codexctl","hook_event_name":"SubagentStart","permission_mode":"default","agent_id":"agent-1","agent_type":"explorer"}
// subagent-stop.json
{"session_id":"session-1","turn_id":"turn-1","transcript_path":"/tmp/rollout-1.jsonl","cwd":"/work/codexctl","hook_event_name":"SubagentStop","permission_mode":"default","agent_id":"agent-1","agent_type":"explorer"}
// stop.json
{"session_id":"session-1","turn_id":"turn-1","transcript_path":"/tmp/rollout-1.jsonl","cwd":"/work/codexctl","hook_event_name":"Stop","permission_mode":"default","stop_hook_active":false}
// permission-request.json
{"session_id":"session-1","turn_id":"turn-1","transcript_path":"/tmp/rollout-1.jsonl","cwd":"/work/codexctl","hook_event_name":"PermissionRequest","permission_mode":"default","tool_name":"apply_patch","tool_input":{"patch":"do not persist me"}}
```

- [ ] **Step 3: Write parser tests first**

In `lifecycle/input.rs`, add table-driven tests that include every generic fixture and explicit size failures:

```rust
#[test]
fn parses_installed_generic_events_without_sensitive_bodies() {
    for raw in [
        include_bytes!("../../../../tests/fixtures/hooks/session-start.json").as_slice(),
        include_bytes!("../../../../tests/fixtures/hooks/user-prompt-submit.json").as_slice(),
        include_bytes!("../../../../tests/fixtures/hooks/pre-tool-use.json").as_slice(),
        include_bytes!("../../../../tests/fixtures/hooks/post-tool-use.json").as_slice(),
        include_bytes!("../../../../tests/fixtures/hooks/subagent-start.json").as_slice(),
        include_bytes!("../../../../tests/fixtures/hooks/subagent-stop.json").as_slice(),
        include_bytes!("../../../../tests/fixtures/hooks/stop.json").as_slice(),
    ] {
        let event = LifecycleEvent::parse(raw).unwrap();
        let persisted = serde_json::to_string(&event).unwrap();
        assert!(!persisted.contains("do not persist me"));
    }
}

#[test]
fn rejects_oversized_identity_and_path() {
    let raw = event_json("UserPromptSubmit", "x".repeat(513), "turn-1", "/work");
    assert!(matches!(LifecycleEvent::parse(raw.as_bytes()), Err(LifecycleInputError::TooLong("session_id"))));
    let raw = event_json("UserPromptSubmit", "session-1", "turn-1", &format!("/{}", "x".repeat(4096)));
    assert!(matches!(LifecycleEvent::parse(raw.as_bytes()), Err(LifecycleInputError::TooLong("cwd"))));
}
```

Also assert that `PermissionRequest`, `PreCompact`, `PostCompact`, an empty `turn_id`, a missing `source`, and a missing `agent_id` are rejected by the generic parser. Permission input is accepted only through the typed `permission` constructor used in Task 3.

Test `LifecycleIdentity::try_new` directly with empty and 513-byte ids, a
4,097-byte path, and normalized versus non-normalized path inputs. Add a
runtime test that
`LifecycleEvent::permission` rejects a validated identity without a turn and
that serialized event identity cannot diverge from the identity supplied to
the constructor. Field privacy compiler-enforces the adapter boundary.

- [ ] **Step 4: Write projection tests first**

Cover the approved state machine with named tests and fixed millisecond timestamps:

```rust
#[test]
fn only_user_prompt_can_supersede_an_open_turn() {
    let mut snapshot = LifecycleSnapshot::default();
    snapshot.apply(prompt("turn-1"), 1_000);
    assert_eq!(snapshot.apply(pre_tool("turn-2"), 2_000), ApplyOutcome::Ignored(IgnoreReason::AmbiguousTurn));
    assert_eq!(snapshot.apply(prompt("turn-2"), 3_000), ApplyOutcome::Applied);
    let state = snapshot.sessions.get("session-1").unwrap();
    assert_eq!(state.current_turn.as_deref(), Some("turn-2"));
    assert!(state.recent_turns.iter().any(|turn| turn == "turn-1"));
}

#[test]
fn subagent_stop_is_idempotent_and_does_not_close_parent() {
    let mut snapshot = LifecycleSnapshot::default();
    snapshot.apply(prompt("turn-1"), 1_000);
    snapshot.apply(subagent_start("turn-1", "agent-1"), 2_000);
    snapshot.apply(subagent_stop("turn-1", "agent-1"), 3_000);
    snapshot.apply(subagent_stop("turn-1", "agent-1"), 4_000);
    let state = snapshot.sessions.get("session-1").unwrap();
    assert!(state.turn_open);
    assert!(state.active_subagents.is_empty());
}
```

Add focused tests for duplicate events, delayed `Stop`, unknown-agent stop, 32-entry recent-turn eviction, the 64-agent rejection, `SessionStart` clearing transient status without discarding recent turns, permission decided versus needs-input, and all event/status mappings.

Add a small test-only reference transition function and enumerate every permutation up to length three from `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, and `Stop` across current/old/other turn ids. Assert the implementation and reference model agree on current turn, open/closed state, projected status, and ignored reason.

- [ ] **Step 5: Run the focused tests and verify RED**

```bash
cargo test -p codexctl-core lifecycle::input::tests
cargo test -p codexctl-core lifecycle::projection::tests
```

Expected: compilation fails because the lifecycle module and public interfaces do not exist.

- [ ] **Step 6: Implement the validated types and pure transition**

Use Serde helper structs with `#[serde(default)]` optional fields and convert immediately through `LifecycleIdentity::try_new`. Keep persisted fields private so parsing and typed constructors are the only construction paths. `LifecycleEvent` must contain only:

```rust
pub struct LifecycleEvent {
    identity: LifecycleIdentity,
    kind: LifecycleEventKind,
}

pub struct LifecycleIdentity {
    session_id: String,
    turn_id: Option<String>,
    transcript_path: Option<PathBuf>,
    cwd: PathBuf,
}

pub enum LifecycleEventKind {
    SessionStart { source: SessionStartSource },
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PermissionRequest { disposition: PermissionDisposition },
    SubagentStart { agent_id: String },
    SubagentStop { agent_id: String },
    Stop,
}
```

Implement `LifecycleSnapshot::apply` as an exhaustive `match`; assign `next_sequence` only after validation and turn-order checks accept the update. `SessionLifecycleState` records both the latest accepted event/sequence/time for diagnostics and the last status-setting event/time for lease calculation. `SubagentStop` updates diagnostics and the active-agent map without refreshing the parent's status lease. Store the last ignored reason on the session without changing its projected status. Cap recent turns and active agents during the same transition, not during later serialization.

- [ ] **Step 7: Verify Task 1 GREEN and review**

```bash
cargo fmt --all --check
cargo test -p codexctl-core lifecycle::input::tests
cargo test -p codexctl-core lifecycle::projection::tests
jj --no-pager diff --git
jj --no-pager st
```

Expected: all focused tests pass; only the new lifecycle module, fixture files, and `lib.rs` are changed in this task changeset.

---

### Task 2: Persist the Bounded Snapshot Safely Across Hook Processes

**Files:**
- Modify: `crates/codexctl-core/Cargo.toml:15-28`
- Modify: `Cargo.lock`
- Create: `crates/codexctl-core/src/lifecycle/store.rs`
- Modify: `crates/codexctl-core/src/lifecycle/mod.rs`
- Create: `crates/codexctl-core/tests/lifecycle_store.rs`

**Interfaces:**
- Consumes: `LifecycleEvent`, `LifecycleSnapshot`, and `ApplyOutcome` from Task 1.
- Produces: `LifecycleStore::at(root: impl Into<PathBuf>) -> Self`, where `root` is the codexctl state root and files live under `root/hooks/`.
- Produces: `compatibility_state_root() -> PathBuf`, the sole production `HOME/.codexctl` resolver.
- Produces: `LifecycleStore::read() -> Result<StoreView, StoreError>` and `LifecycleStore::record(event) -> Result<ApplyOutcome, StoreError>`.
- Produces: `StoreCondition::{Healthy, Missing, Corrupt, NewerSchema(u32)}` for doctor and session diagnostics.

**Acceptance Criteria:**
- Writers serialize under a stable advisory lock, wait no longer than 100 ms, and atomically replace the snapshot without `fsync`.
- Separate processes cannot lose accepted updates and every accepted update receives a unique increasing sequence; lock-timeout rejections are reported separately rather than treated as lost writes.
- The state path and permissions, retention, session/turn/subagent/size bounds, abandoned-temp cleanup, atomic replacement, and active-capacity rejection match the global constraints.
- Newer schemas are never mutated. Corrupt current-schema files are quarantined and rebuilt; quarantine failure preserves the original.
- Readers return a condition instead of mutating corrupt or newer-schema state. The App retains its last valid attached observation across a transient read/lock failure and still subjects it to normal leases; the store itself retains no process-global cache.

- [ ] **Step 1: Start the task changeset**

```bash
jj new -m "✨ feat: persist lifecycle hook state atomically (codexctl-rqm)"
jj --no-pager st
```

- [ ] **Step 2: Add dependencies and failing store tests**

Move `tempfile = "3"` into normal `codexctl-core` dependencies and add `fs2 = "0.4"`. Keep `tempfile` available to tests through the normal dependency rather than duplicating it.

In `lifecycle/store.rs`, add deterministic unit tests for paths, schema conditions, permissions, pruning, capacity, quarantine retention, abandoned `lifecycle.tmp-*` cleanup, serialized-size rejection, and atomic replacement. The atomic-replacement test repeatedly reads while replacing a known old snapshot with a known new snapshot and asserts every observed file is one complete valid version, never a partial serialization. The path test must assert:

```rust
let store = LifecycleStore::at("/state/codexctl");
assert_eq!(store.snapshot_path(), Path::new("/state/codexctl/hooks/lifecycle.json"));
assert_eq!(store.lock_path(), Path::new("/state/codexctl/hooks/lifecycle.lock"));
```

The newer-schema regression must write `{"schema_version":2}` and assert both `StoreCondition::NewerSchema(2)` and byte-for-byte file preservation after `record` returns `StoreError::NewerSchema(2)`.

- [ ] **Step 3: Add the separate-process concurrency test**

Use the integration-test binary itself as the child helper:

```rust
#[test]
fn concurrent_processes_preserve_all_accepted_updates() {
    let temp = tempfile::tempdir().unwrap();
    let children: Vec<_> = (0..4)
        .map(|index| Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "child_records_one_event"])
            .env("CODEXCTL_LIFECYCLE_CHILD_ROOT", temp.path())
            .env("CODEXCTL_LIFECYCLE_CHILD_INDEX", index.to_string())
            .stdout(Stdio::piped())
            .spawn().unwrap())
        .collect();
    let mut accepted = Vec::new();
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let report = String::from_utf8(output.stdout).unwrap();
        if let Some(index) = report.strip_prefix("accepted:") {
            accepted.push(index.trim().parse::<usize>().unwrap());
        } else {
            assert!(report.starts_with("rejected:lock-timeout:"), "{report}");
        }
    }
    assert!(!accepted.is_empty());
    let view = LifecycleStore::at(temp.path()).read().unwrap();
    let snapshot = view.snapshot.unwrap();
    for index in &accepted {
        assert!(snapshot.sessions.contains_key(&format!("session-{index}")));
    }
    assert_eq!(snapshot.sessions.len(), accepted.len());
    let mut sequences = snapshot.sessions.values()
        .map(|s| s.last_sequence).collect::<Vec<_>>();
    sequences.sort_unstable();
    sequences.dedup();
    assert_eq!(sequences.len(), accepted.len());
}

#[test]
#[ignore]
fn child_records_one_event() {
    let Some(root) = std::env::var_os("CODEXCTL_LIFECYCLE_CHILD_ROOT") else { return; };
    let index: usize = std::env::var("CODEXCTL_LIFECYCLE_CHILD_INDEX").unwrap().parse().unwrap();
    match LifecycleStore::at(root).record(prompt_for(index)) {
        Ok(ApplyOutcome::Applied) => println!("accepted:{index}"),
        Err(StoreError::LockTimeout) => println!("rejected:lock-timeout:{index}"),
        result => panic!("unexpected child result: {result:?}"),
    }
}
```

Add a second child helper that signals readiness only after holding the lock,
keeps it longer than 100 ms, and assert the competing writer deterministically
returns `StoreError::LockTimeout` without changing the snapshot. Keep this
timeout test separate from the accepted-write concurrency assertion. Also keep
abandoned-temp cleanup, quarantine retention, and atomic-replacement assertions
as independent tests so each failure identifies one persistence invariant.

- [ ] **Step 4: Run focused tests and verify RED**

```bash
cargo test -p codexctl-core lifecycle::store::tests
cargo test -p codexctl-core --test lifecycle_store
```

Expected: compilation fails because `LifecycleStore` and its dependencies are absent.

- [ ] **Step 5: Implement locking, recovery, and atomic replacement**

Use a stable `lifecycle.lock` opened read/write, `fs2::FileExt::try_lock_exclusive`, and a deadline loop with at most 5 ms between attempts. Under the exclusive lock:

1. remove sibling names beginning `lifecycle.tmp-`;
2. read at most `MAX_SNAPSHOT_BYTES + 1`;
3. return without mutation for a newer schema;
4. rename corrupt current-schema bytes to a unique `.corrupt-<epoch-ms>` sibling, pruning oldest quarantines after three;
5. apply and prune the event;
6. serialize and reject output over 1 MiB; and
7. write through `tempfile::Builder::new().prefix("lifecycle.tmp-").tempfile_in(hooks_dir)` and `persist(snapshot_path)`.

Create the directory before the lock, then enforce Unix modes with `PermissionsExt`: directory `0o700`, lock/temp/final files `0o600`. Do not call `sync_all` or sync the parent directory.

Readers acquire a shared lock with the same deadline and return `StoreView { snapshot: None, condition }` for missing, corrupt, or newer-schema files. Unlock through `fs2::FileExt::unlock` on every success and error path by using a small private guard type.

- [ ] **Step 6: Verify Task 2 GREEN and review**

```bash
cargo fmt --all --check
cargo test -p codexctl-core lifecycle::store::tests
cargo test -p codexctl-core --test lifecycle_store
cargo clippy -p codexctl-core --all-targets -- -D warnings
jj --no-pager diff --git
jj --no-pager st
```

Expected: store and cross-process tests pass; the diff is limited to dependencies, lockfile, and lifecycle store files.

---

### Task 3: Add Generic and Permission Hook Adapters

**Files:**
- Create: `src/lifecycle_hook.rs`
- Modify: `src/main.rs:296-307`
- Modify: `src/main.rs:480-492`
- Modify: `src/main.rs:620-632`
- Modify: `src/brain/permission_hook.rs:17-210`
- Modify: `src/brain/permission_hook.rs:330-510`
- Create: `tests/lifecycle_hook_cli.rs`

**Interfaces:**
- Consumes: `LifecycleEvent::parse`, `LifecycleEvent::permission`, `LifecycleStore`, and the compatibility resolver.
- Produces: one `read_bounded_hook_input` helper shared by the generic and permission adapters; neither adapter may call unbounded `read_to_string` or `read_to_end` on stdin.
- Produces: hidden `codexctl --lifecycle-hook`, which reads one bounded payload, records it, writes diagnostics only to stderr, and exits successfully with empty stdout.
- Preserves: hidden `--permission-hook` response JSON, audit-before-response ordering, Bash-only inference, 25-second inference clamp, and all current allow/deny thresholds.
- Produces: status-only `NeedsInput` observations for non-Bash, disabled brain, gate off, abstention, inference failure, unsupported action, and below-threshold Bash results; confident Bash allow/deny records `Processing`.

**Acceptance Criteria:**
- The generic handler accepts exactly the seven installed lifecycle events, caps stdin at 64 KiB, never prints stdout, and cannot block Codex on malformed input or persistence failure.
- Both handlers cap stdin at 64 KiB. On overflow the permission handler performs no parsing, inference, lifecycle write, audit, or decision; it emits one bounded stderr diagnostic and exits successfully with empty stdout.
- The permission handler parses common identity for every tool, invokes inference only for valid Bash commands, and emits no decision for non-Bash requests.
- Lifecycle persistence failure never suppresses a valid permission allow/deny response and never creates one.
- Binary subprocess tests prove stdout, stderr, exit status, state, input bounds, and permission JSON behavior with temporary homes, including byte-for-byte identical valid decision stdout when lifecycle persistence fails.
- The first-run star prompt never runs from either internal hook adapter.

- [ ] **Step 1: Start the task changeset**

```bash
jj new -m "✨ feat: consume Codex lifecycle hook input (codexctl-rqm)"
jj --no-pager st
```

- [ ] **Step 2: Write generic adapter unit and subprocess tests first**

The adapter must expose an injectable function for unit tests:

```rust
pub(crate) fn run_with<R: Read, W: Write, E: Write>(
    stdin: R,
    stdout: W,
    stderr: E,
    store: &LifecycleStore,
)
```

Test valid input, malformed JSON, a 65,537-byte stream, lock timeout, and newer-schema state. Every case exits through the function without panicking; success has empty stdout/stderr, while failures have empty stdout and one bounded diagnostic without raw payload values. Put the bounded reader in this module as `pub(crate)` and test exact-boundary inputs of 65,536 and 65,537 bytes before either adapter uses it.

In `tests/lifecycle_hook_cli.rs`, use `env!("CARGO_BIN_EXE_codexctl")`, set a temporary `HOME`, pipe `user-prompt-submit.json`, and assert successful exit, empty stdout, and a projected `Processing` state under `<home>/.codexctl/hooks/lifecycle.json`.

- [ ] **Step 3: Rewrite permission tests for all-tool observation**

Replace `rejects_non_bash_tool` with:

```rust
#[test]
fn non_bash_records_needs_input_without_inference_or_response() {
    let home = tempfile::tempdir().unwrap();
    let store = LifecycleStore::at(home.path().join(".codexctl"));
    let input = include_str!("../../tests/fixtures/hooks/permission-request.json");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    run_with_gate_and_store(
        Cursor::new(input), &mut stdout, &mut stderr,
        Some(&enabled_config()), "on", &store,
        |_, _| panic!("non-Bash permission must not reach inference"),
    );
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    assert_eq!(projected_status(&store, "session-1"), ProjectedStatus::NeedsInput);
}
```

For existing disabled/gate-off/abstain/error/below-threshold tests, inject a temporary store and assert `NeedsInput`. For confident allow/deny, assert the existing response and audit plus `Processing`. For invalid common identity or invalid Bash command, assert no lifecycle entry.

Add a permission-adapter test with a 65,537-byte reader and inference closure
that panics if invoked. Assert empty stdout, no audit or lifecycle file, one
bounded stderr diagnostic without payload bytes, and a successful adapter
return. Add subprocess coverage that runs the same valid confident Bash
decision once against a healthy store and once against a forced lifecycle
failure, then asserts stdout bytes are identical in both runs and still match
the existing response fixture.

- [ ] **Step 4: Run focused tests and verify RED**

```bash
cargo test --bin codexctl lifecycle_hook::tests
cargo test --bin codexctl brain::permission_hook::tests
cargo test --test lifecycle_hook_cli
```

Expected: compilation or assertions fail because the generic flag, bounded reader, all-tool permission parser, and lifecycle side effects are absent.

- [ ] **Step 5: Implement the generic adapter and hidden dispatch**

Implement `read_bounded_hook_input` with `take((MAX_HOOK_INPUT_BYTES + 1) as u64)` and reject when the resulting buffer exceeds the cap. Use it before parsing in both internal hook adapters. The generic adapter then calls `LifecycleEvent::parse` and `store.record`. Convert every error to `codexctl lifecycle hook: <bounded reason>` on stderr and return normally.

Add hidden `lifecycle_hook: bool` beside `permission_hook` in `Cli`; dispatch it before endpoint warnings and ordinary command behavior. Replace `is_permission_hook` in `main()` with:

```rust
let is_internal_hook = cli.permission_hook || cli.lifecycle_hook;
let result = run_main(cli);
if result.is_ok() && !is_internal_hook {
    maybe_print_star_prompt(is_demo);
}
```

- [ ] **Step 6: Refactor permission parsing and outcome persistence**

Deserialize `tool_input` as `serde_json::Value`. Split common identity from the Bash request, constructing `lifecycle` only through `LifecycleIdentity::try_new`:

```rust
struct ParsedPermissionRequest {
    lifecycle: LifecycleIdentity,
    tool_name: String,
    command: Option<String>,
}

enum PermissionEvaluation {
    Decision(HookDecision),
    NeedsInput(LifecycleIdentity),
}
```

Non-Bash returns `NeedsInput` before config or inference checks. Bash extracts a non-empty string `tool_input.command` and follows the existing query path. Persist `NeedsInput` immediately before a no-decision return. For a valid decision, preserve serialization and durable audit ordering, then best-effort record `PermissionDisposition::Decided` before writing the already prepared response. A lifecycle-store error writes a diagnostic but does not alter the response bytes.

The valid-decision sequence is exact: serialize the response bytes, durably
append the existing audit record, best-effort record lifecycle status, then
write the already serialized bytes to stdout. Overflow and parse failure occur
before every one of those steps and therefore cannot authorize, audit, or
persist anything.

- [ ] **Step 7: Verify Task 3 GREEN and review**

```bash
cargo fmt --all --check
cargo test --bin codexctl lifecycle_hook::tests
cargo test --bin codexctl brain::permission_hook::tests
cargo test --test lifecycle_hook_cli
cargo clippy --bin codexctl --all-targets -- -D warnings
jj --no-pager diff --git
jj --no-pager st
```

Expected: all adapter tests pass, stdout remains protocol-clean, and no live user state was touched.

---

### Task 4: Install, Remove, and Diagnose the Managed Hook Set

**Files:**
- Modify: `src/init/hooks.rs:1-470`
- Modify: `src/init/hooks.rs:495-570`
- Modify: `src/init/hooks.rs:575-1585`
- Modify: `src/init/state.rs:200-280`
- Modify: `src/doctor.rs:180-245`
- Modify: `src/doctor.rs:500-590`
- Modify: `nix/home-manager.nix:30-103`
- Modify: `nix/tests/home-manager-module.nix:1-245`

**Interfaces:**
- Consumes: the two hidden adapter commands from Task 3.
- Produces: imperative and declarative definitions for `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `SubagentStart`, `SubagentStop`, and `Stop` with the exact matchers/timeouts in the design.
- Preserves: exact managed-command ownership, structural preservation of unrelated handlers/matcher groups, legacy `--json` cleanup-only recognition, and conservative terminal-fallback blocking.
- Produces: `discover_lifecycle_hooks_at(home, cwd) -> LifecycleHookDiscovery` with per-event missing/current/stale/disabled/unavailable state, duplicate-scope detection, and a separate trust-unverified flag.
- Produces: separate doctor checks for structurally managed definitions and Codex trust. A complete current definition set passes; the independent trust check remains advisory and directs the operator to `/hooks`.

**Acceptance Criteria:**
- Fresh init installs exactly the approved eight events; re-init is structurally idempotent and uninit removes only exact codexctl-managed handlers.
- Permission uses matcher `*` and 30 seconds; lifecycle matchable events use the approved matcher and 2 seconds; UserPromptSubmit and Stop omit `matcher`.
- Global/project duplicates, missing events, stale commands or timeouts, disabled entries, and unavailable executables are diagnosed without claiming trust.
- Structurally current definitions report `Pass`; trust reports a separate `Advisory`. Corrupt/newer lifecycle state is diagnosed separately from both hook-definition checks.
- Home Manager emits the same definitions using the selected package's absolute executable and preserves independently supplied hooks.
- Downgrade fixtures prove the newer remover strips exact bare and absolute lifecycle commands while preserving lookalikes, user hooks, and state; legacy lifecycle commands remain cleanup-only. Declarative rollback is covered independently by reverting/removing the Home Manager definitions and rebuilding before downgrade.

- [ ] **Step 1: Start the task changeset**

```bash
jj new -m "✨ feat: install managed lifecycle hooks (codexctl-rqm)"
jj --no-pager st
```

- [ ] **Step 2: Rewrite imperative installer tests first**

Change `HookSpec.matcher` to `Option<&'static str>` in the test expectations and assert this exact matrix:

```rust
const EXPECTED: &[(&str, Option<&str>, &str, u64)] = &[
    ("SessionStart", Some("startup|resume|clear|compact"), "--lifecycle-hook", 2),
    ("UserPromptSubmit", None, "--lifecycle-hook", 2),
    ("PreToolUse", Some("*"), "--lifecycle-hook", 2),
    ("PermissionRequest", Some("*"), "--permission-hook", 30),
    ("PostToolUse", Some("*"), "--lifecycle-hook", 2),
    ("SubagentStart", Some("*"), "--lifecycle-hook", 2),
    ("SubagentStop", Some("*"), "--lifecycle-hook", 2),
    ("Stop", None, "--lifecycle-hook", 2),
];
```

Add idempotent merge/uninit fixtures containing a user-owned `PreToolUse` command in the same matcher, an absolute Nix-store codexctl lifecycle command, a lookalike relative `bin/codexctl --lifecycle-hook` that remains user-owned, legacy `PostToolUse`/`Stop` `codexctl --json` handlers that are removed, and exact new handlers removed by `init --remove` without deleting `~/.codexctl`.

- [ ] **Step 3: Add discovery and doctor tests first**

Build temporary global/project hook files from the expected matrix. Add named tests for complete/current, one missing event, wrong timeout, permission matcher still `Bash`, disabled handler, nonexistent absolute executable, duplicate scopes, unrelated hooks only, corrupt lifecycle state, and newer-schema lifecycle state.

For a complete current set, assert the managed-definition check passes with `definitions current` and a separate trust check is advisory with `trust unverified; review /hooks`. Missing, stale, disabled, duplicate, and unavailable definition cases must name the exact event/scope/condition and provide an init or `/hooks` recovery hint. Corrupt-store and newer-schema cases belong to a separate lifecycle-state check and provide quarantine or upgrade guidance without changing the definition result. Inject a temporary `LifecycleStore` into the pure doctor helper rather than reading the operator's compatibility root in tests.

- [ ] **Step 4: Run focused Rust tests and verify RED**

```bash
cargo test --bin codexctl init::hooks::tests
cargo test --bin codexctl init::state::tests
cargo test --bin codexctl doctor::tests
```

Expected: current one-hook output fails the eight-event matrix and lifecycle discovery symbols are missing.

- [ ] **Step 5: Implement exact managed installation and discovery**

Populate `HOOKS` with the expected matrix. Omit the JSON `matcher` key when the spec has `None`; do not emit an empty string. Add `is_current_lifecycle_command` using the existing exact bare-or-absolute executable parser. `is_managed_command` recognizes new lifecycle commands only on the seven lifecycle events, the permission command only on `PermissionRequest`, and legacy `--json` only on `PostToolUse`/`Stop` for cleanup.

Discovery must inspect every applicable global/project file and aggregate an event-keyed state:

```rust
pub struct ManagedHookEventState {
    pub configured: bool,
    pub current: bool,
    pub stale: bool,
    pub disabled: bool,
    pub unavailable: bool,
}

pub struct LifecycleHookScope {
    pub events: BTreeMap<ManagedHookEvent, ManagedHookEventState>,
}
```

Retain `PermissionHookDiscovery::blocks_terminal_fallback`, updating its current matcher from `Bash` to `*`. Hook trust stays `unverified` whenever a definition is otherwise enabled because hooks JSON cannot prove the Codex trust decision.

- [ ] **Step 6: Rewrite Home Manager definitions and assertions**

Use `lib.getExe cfg.package` for every command. Under `programs.codex.hooks`, emit the seven lifecycle entries plus permission. For example:

```nix
programs.codex.hooks = {
  SessionStart = lib.mkAfter [{
    matcher = "startup|resume|clear|compact";
    hooks = [{ type = "command"; command = "${executable} --lifecycle-hook"; timeout = 2; }];
  }];
  UserPromptSubmit = lib.mkAfter [{
    hooks = [{ type = "command"; command = "${executable} --lifecycle-hook"; timeout = 2; }];
  }];
  PermissionRequest = lib.mkAfter [{
    matcher = "*";
    hooks = [{ type = "command"; command = "${executable} --permission-hook"; timeout = 30; statusMessage = "Brain reviewing permission…"; }];
  }];
  Stop = lib.mkAfter [{
    hooks = [{ type = "command"; command = "${executable} --lifecycle-hook"; timeout = 2; }];
  }];
};
```

Add analogous `PreToolUse`, `PostToolUse`, `SubagentStart`, and `SubagentStop` entries with matcher `*`. Assert exact commands, matchers, and timeouts; assert the pre-existing independent Stop handler remains present and unchanged before the appended codexctl handler.

Add a declarative rollback evaluation with the codexctl hook option removed or
disabled: the generated hook set contains no exact codexctl lifecycle or
permission command, while independently supplied hooks remain unchanged. This
is the Home Manager counterpart to the imperative uninit fixture.

- [ ] **Step 7: Verify Task 4 GREEN and review**

```bash
cargo fmt --all --check
cargo test --bin codexctl init::hooks::tests
cargo test --bin codexctl init::state::tests
cargo test --bin codexctl doctor::tests
nix fmt -- --check .
nix build .#checks.x86_64-linux.home-manager-module
jj --no-pager diff --git
jj --no-pager st
```

Expected: imperative and declarative tests pass, unrelated hooks remain intact, managed definitions pass when current, and the separate doctor trust check remains advisory rather than claiming trust.

---

### Task 5: Reconcile Hook Evidence with Transcript Semantics

**Files:**
- Create: `crates/codexctl-core/src/lifecycle/reconcile.rs`
- Modify: `crates/codexctl-core/src/lifecycle/mod.rs`
- Modify: `crates/codexctl-core/src/codex_transcript.rs:1-180`
- Modify: `crates/codexctl-core/src/codex_transcript.rs:250-380`
- Modify: `crates/codexctl-core/src/session.rs:1-210`
- Modify: `crates/codexctl-core/src/session.rs:330-410`
- Modify: `crates/codexctl-core/src/monitor.rs:295-465`
- Modify: `crates/codexctl-core/src/monitor.rs:500-730`
- Modify: `crates/codexctl-core/src/monitor.rs:930-1120`

**Interfaces:**
- Consumes: projected `SessionLifecycleState` and `StoreCondition` from Tasks 1-2.
- Produces: `parse_timed_line(&str) -> Option<TimedCodexEvent>` while retaining `parse_line(&str) -> Option<CodexEvent>` for existing discovery callers.
- Produces: `TranscriptSemantic::{Progress, Complete, ExplicitInput}` and `TranscriptEvidence { semantic, observed_at_ms: Option<u64> }` persisted on `CodexSession` across refresh ticks.
- Produces: `LifecycleDiagnostic { available, event, age_ms, contributing, ignored_reason, store_condition }` and a hook evidence field on `CodexSession` that contains no approval/tool input.
- Produces: `reconcile_status(session, now_ms)` called by `monitor::infer_status` at the approved precedence point.
- Defines: transcript invalidation ordering as top-level transcript event timestamp versus lifecycle receipt timestamp. Only a strictly newer, non-future transcript timestamp invalidates hook evidence; equal, missing, or more-than-five-seconds-future timestamps do not.

**Acceptance Criteria:**
- Transcript entry timestamps and lifecycle meaning, never file mtime alone, determine hook/transcript correction. Invalidation requires `transcript.timestamp_ms > lifecycle.received_at_ms`; equal, missing, and more-than-five-seconds-future timestamps are non-invalidating.
- Explicit terminal approval and transcript `request_user_input` always produce `NeedsInput`; hook evidence never fabricates their identity fields.
- New transcript progress overrides an older hook Stop; transcript completion ends older hook Processing; matching completion does not erase hook Idle.
- Event leases are exactly 30 seconds for prompt/automatic permission/PostToolUse and 10 minutes for PreToolUse/SubagentStart/NeedsInput/Stop, with the approved semantic invalidators.
- Future receive timestamps over five seconds are ignored; missing/expired/disabled evidence falls through to current transcript/CPU inference.
- Existing transcript, approval, status inference, token, and cost tests remain green.

- [ ] **Step 1: Start the task changeset**

```bash
jj new -m "✨ feat: reconcile lifecycle and transcript status (codexctl-rqm)"
jj --no-pager st
```

- [ ] **Step 2: Add timed transcript parser tests first**

Add `TimedCodexEvent` as a wrapper, not a breaking replacement:

```rust
pub struct TimedCodexEvent {
    pub event: CodexEvent,
    pub timestamp_ms: Option<u64>,
}

pub fn parse_line(line: &str) -> Option<CodexEvent> {
    parse_timed_line(line).map(|timed| timed.event)
}
```

Tests parse RFC3339 timestamps from `task_started`, `task_complete`, user messages, function calls/outputs, and `request_user_input`. Missing or malformed timestamps yield `None` for `timestamp_ms` without discarding an otherwise valid event.

- [ ] **Step 3: Add precedence and semantic reconciliation tests first**

Construct sessions directly and use a fixed `now_ms`. Cover every row in this matrix:

```rust
#[test]
fn explicit_input_and_confirmed_approval_outrank_hooks() {
    let mut session = session_with_hook(ProjectedStatus::Processing, 1_000);
    session.explicit_input_required = true;
    infer_status_at(&mut session, "assistant", "tool_use", false, 2_000);
    assert_eq!(session.status, SessionStatus::NeedsInput);

    session.explicit_input_required = false;
    session.approval = ApprovalObservation::Confirmed(approval_evidence());
    infer_status_at(&mut session, "assistant", "tool_use", false, 2_000);
    assert_eq!(session.status, SessionStatus::NeedsInput);
}

#[test]
fn newer_transcript_progress_overrides_hook_stop() {
    let mut session = session_with_hook(ProjectedStatus::Idle, 1_000);
    session.transcript_evidence = Some(TranscriptEvidence::progress(2_000));
    infer_status_at(&mut session, "assistant", "", false, 3_000);
    assert_eq!(session.status, SessionStatus::Processing);
    assert!(!session.lifecycle_diagnostic.contributing);
}
```

Add cases for task completion strictly after hook Processing, an old completion followed by a newer prompt hook, matching Stop plus completion, equal hook/transcript timestamps, missing transcript timestamps, transcript and hook timestamps over `now + 5_000`, each lease just before/at expiry, low/high CPU fallback, missing transcript, process-finished bypass, and proof that lifecycle application leaves `pending_tool_*`, `approval`, terminal target, and file path untouched. Equal, missing, old, or future transcript timestamps must not invalidate fresh hook evidence; confirmed approval and explicit `request_user_input` must still win through their earlier precedence branches.

- [ ] **Step 4: Run focused tests and verify RED**

```bash
cargo test -p codexctl-core codex_transcript::tests
cargo test -p codexctl-core monitor::tests
cargo test -p codexctl-core lifecycle::reconcile::tests
```

Expected: new timed-parser, diagnostic, and reconciliation symbols are missing.

- [ ] **Step 5: Implement semantic evidence and approved precedence**

Parse the top-level transcript `timestamp` with the existing `time` dependency. In `monitor::update_codex_tokens`, update `TranscriptEvidence` from actual entry timestamps:

- task start, user message, agent message, reasoning, tool call, and tool output → `Progress`;
- `task_complete` and `turn_aborted` → `Complete`; and
- a function/custom call named `request_user_input` → `ExplicitInput` as well as the existing `explicit_input_required` flag.

Add `infer_status_at(..., now_ms)` for deterministic tests; keep public `infer_status` as the wall-clock wrapper. Its early-return order must be: finished/non-live result supplied by the caller; confirmed approval; explicit input; `reconcile::contributing_status`; current transcript `task_state`; CPU over 5%; existing legacy fallbacks.

`contributing_status` computes age with checked/saturating arithmetic and rejects lifecycle receipt timestamps over `now + 5_000`. For semantic invalidation, reject transcript timestamps over the same future bound and require `transcript.timestamp_ms > lifecycle.received_at_ms`; `None`, equality, and older timestamps are non-invalidating. Apply semantic class only after that strict ordering check. Apply these exact leases and invalidators: prompt/decided-permission/PostToolUse expire after 30 seconds; PreToolUse expires after 10 minutes or PostToolUse/Stop/new turn/strictly newer transcript progress; SubagentStart expires after 10 minutes or its stop/strictly newer transcript progress; NeedsInput expires after 10 minutes; Stop Idle expires after 10 minutes or any strictly newer hook/transcript event. Confirmed approval and explicit input remain earlier unconditional precedence branches. The function only returns `SessionStatus`; it never mutates actionable fields.

- [ ] **Step 6: Verify Task 5 GREEN and review**

```bash
cargo fmt --all --check
cargo test -p codexctl-core codex_transcript::tests
cargo test -p codexctl-core lifecycle::reconcile::tests
cargo test -p codexctl-core monitor::tests
cargo clippy -p codexctl-core --all-targets -- -D warnings
jj --no-pager diff --git
jj --no-pager st
```

Expected: semantic ordering tests pass and existing telemetry/status suites remain unchanged outside the new evidence fields.

---

### Task 6: Bind Lifecycle State to Live Sessions and Expose Provenance

**Files:**
- Modify: `crates/codexctl-core/src/discovery.rs:330-610`
- Modify: `crates/codexctl-core/src/discovery.rs:740-835`
- Modify: `crates/codexctl-core/src/discovery.rs:1380-1530`
- Modify: `crates/codexctl-core/src/lifecycle/reconcile.rs`
- Modify: `crates/codexctl-core/src/session.rs:790-855`
- Modify: `crates/codexctl-tui/src/app.rs:300-550`
- Modify: `crates/codexctl-tui/src/app.rs:680-760`
- Modify: `crates/codexctl-tui/src/app.rs:3120-3240`
- Modify: `crates/codexctl-tui/src/ui/detail.rs:20-115`
- Modify: `crates/codexctl-tui/src/ui/detail.rs:135-230`

**Interfaces:**
- Consumes: `LifecycleStore::read`, `StoreView`, and reconciliation types from Tasks 2 and 5.
- Produces in core: `apply_store_view(sessions: &mut [CodexSession], view: &StoreView, now_ms: u64)` for exact-id attachment, guarded SessionStart hints, non-healthy-view clearing, and per-session diagnostics.
- Produces in core: `retain_after_store_error(sessions: &mut [CodexSession], error: &StoreError, now_ms: u64)` for bounded diagnostic updates while retaining previously valid evidence only through its ordinary lease.
- Produces: a private `App::build(lifecycle_store)` constructor that does not refresh, plus `App::new()` using the compatibility resolver and a test-only `App::with_lifecycle_store(store)` that refreshes only when the test explicitly requests it.
- Produces: a private App refresh seam taking a `FnOnce() -> Result<StoreView, StoreError>` so tests can prove zero reads for non-local sessions and exactly one read for any number of eligible local sessions without adding a store trait.
- Extends: `CodexSession::to_json_value()` with a non-sensitive `lifecycle` object and the detail panel with lifecycle availability/event/age/contribution/ignored reason.

**Acceptance Criteria:**
- Exact session-id matches attach lifecycle evidence only to live process-backed sessions with matching cwd/transcript identity.
- A SessionStart hint binds an unmatched live process only when it is at most 30 seconds old, has a non-null existing transcript whose `session_meta` exactly matches id/cwd, has post-process-start transcript activity, and has one unambiguous compatible process.
- Null paths, stale hints, mismatched metadata, multiple same-cwd processes, or multiply claimed transcripts fall back without assignment and expose an ignored reason.
- At most one snapshot is read per dashboard refresh before status inference, and no read occurs when there are no local process-backed sessions.
- Transient lock/I/O errors retain prior valid evidence only until its normal lease expires and expose the error diagnostically. Missing, corrupt, and newer-schema views clear contribution and attach no new evidence.
- JSON and TUI detail expose provenance without raw payloads; demo/remote sessions do not read or claim local lifecycle state.
- Resume, clear, compact, transcript replacement, and retained session merge tests remain green.

- [ ] **Step 1: Start the task changeset**

```bash
jj new -m "✨ feat: bind lifecycle state to live sessions (codexctl-rqm)"
jj --no-pager st
```

- [ ] **Step 2: Add guarded identity tests first**

Expose a read-only transcript summary helper from discovery and test `apply_store_view` with temporary rollouts. A positive fixture must contain a matching `session_meta`, have mtime at or after process start, and be the only compatible candidate.

Add negative tests for all of these exact cases:

```text
stale SessionStart older than 30 seconds
future SessionStart more than 5 seconds ahead
null transcript_path
missing transcript file
session_meta id mismatch
session_meta cwd mismatch
transcript activity before process start
two unbound processes with the same cwd
two lifecycle sessions claiming one transcript
non-process-backed remote or demo session
```

Every negative case leaves the placeholder `codex-<pid>` id and `jsonl_path` unchanged and sets a bounded ignored reason.

- [ ] **Step 3: Add dashboard and output tests first**

Test `apply_store_view` and `retain_after_store_error` directly in core. Start with valid attached evidence and assert a lock/I/O error retains it before lease expiry but not at expiry; assert missing, corrupt, and newer-schema views clear contribution immediately and cannot attach new evidence.

In App tests, inject `LifecycleStore::at(temp.path())`, write one event, build without an implicit refresh, call a private `reconcile_discovered_sessions(&mut sessions, now_ms)` helper with constructed live sessions, and assert the resulting status/provenance. Exercise its private `FnOnce` read seam with a counter: two eligible local sessions invoke it exactly once, while only demo/remote/non-process sessions invoke a panic closure zero times. Add JSON assertions:

```rust
let lifecycle = session.to_json_value()["lifecycle"].clone();
assert_eq!(lifecycle["available"], true);
assert_eq!(lifecycle["last_event"], "PreToolUse");
assert_eq!(lifecycle["contributing"], true);
assert!(lifecycle.get("prompt").is_none());
assert!(lifecycle.get("tool_input").is_none());
```

Render the detail panel and assert it contains `Lifecycle`, the event, age, and either `contributing` or the ignored reason. Missing store state renders `unavailable` without changing the session status.

- [ ] **Step 4: Run focused tests and verify RED**

```bash
cargo test -p codexctl-core discovery::tests
cargo test -p codexctl-core session::tests
cargo test -p codexctl-tui app::tests
cargo test -p codexctl-tui ui::detail::tests
```

Expected: lifecycle binding, injected App construction, JSON provenance, and detail rendering are absent.

- [ ] **Step 5: Implement exact and guarded attachment**

Keep `apply_store_view` and `retain_after_store_error` in `codexctl-core`; neither TUI nor JSON code reimplements attachment or retention. `apply_store_view` first attaches entries whose session id already equals a live process-backed session and whose cwd plus optional transcript path agree. Normalize existing paths with `canonicalize`; when a path does not exist, compare a lexical normalization that removes `.` and resolves `..` without crossing the root. For unbound placeholder sessions, consider only a fresh `SessionStart` entry and validate the transcript's first `session_meta` through the discovery helper. Count compatible processes and transcript claims before mutating; require exactly one of each. Never use cwd alone.

Attach only a compact evidence copy to `CodexSession`; do not copy the whole snapshot or active-agent map. Derive `active_subagent_count` from the accepted state only when it is newer than transcript subagent evidence; otherwise preserve transcript rollups.

- [ ] **Step 6: Wire one store read into the refresh loop and outputs**

Add a `LifecycleStore` field to `App`. In `refresh`, first check whether any local process-backed session is eligible. If none is eligible, skip the store entirely. Otherwise call the private `FnOnce` read seam exactly once after live process enrichment and before incremental transcript parsing: successful reads go to core `apply_store_view`, while transient errors go to core `retain_after_store_error`. A transient read or lock error updates the store condition but retains the last valid attached observation only until its ordinary lease expires; missing, corrupt, or newer-schema state clears contribution and supplies no new evidence. `merge_discovered_session` preserves accumulated lifecycle/transcript evidence only when process/transcript identity is unchanged; a clear/resume transition resets it through the existing new-session path.

Add this JSON shape:

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

The detail panel renders the same fields in one compact section. Do not expose session prompts, tool names/inputs, paths, or agent ids in lifecycle diagnostics.

- [ ] **Step 7: Verify Task 6 GREEN and review**

```bash
cargo fmt --all --check
cargo test -p codexctl-core discovery::tests
cargo test -p codexctl-core session::tests
cargo test -p codexctl-tui app::tests
cargo test -p codexctl-tui ui::detail::tests
cargo clippy --workspace --all-targets -- -D warnings
jj --no-pager diff --git
jj --no-pager st
```

Expected: identity ambiguity safely falls back, dashboard/JSON provenance is non-sensitive, and existing transcript transition tests pass.

---

### Task 7: Document Rollout, Exercise the Real Binary, and Run Release Gates

**Files:**
- Modify: `docs/configuration.md:35-48`
- Modify: `docs/troubleshooting.md:1-50`
- Modify: `docs/reference.md:1-60`
- Modify: `tests/lifecycle_hook_cli.rs`

**Interfaces:**
- Consumes: the completed handler, installer, store, reconciliation, and diagnostics behavior.
- Produces: operator-facing install/trust/downgrade/recovery guidance and an ignored warm-subprocess benchmark.
- Produces: final evidence that the complete Cargo and Nix gate set passes without changing live user state.
- Consumes: the `implementation-base` change id recorded in `codexctl-rqm` before Task 1; final scope verification never assumes an exact changeset count.

**Acceptance Criteria:**
- Documentation explains explicit init/rebuild, `/hooks` trust review, state location, status-only semantics, separate doctor definition/trust/state checks, corrupt/newer-schema recovery, and both imperative and Home Manager downgrade sequences.
- An ignored temporary-home smoke/benchmark test invokes the real binary repeatedly, reports warm latency percentiles, and targets under 50 ms without a timing assertion.
- The smoke path verifies generated hook JSON, lifecycle stdout silence, permission response JSON, snapshot diagnostics, and exact uninit preservation.
- Every final Cargo and Nix quality gate passes; the final diff is limited to `codexctl-rqm` files.

- [ ] **Step 1: Start the task changeset**

```bash
jj new -m "📝 docs: document lifecycle hook rollout (codexctl-rqm)"
jj --no-pager st
```

- [ ] **Step 2: Add the ignored real-binary benchmark/smoke test**

In `tests/lifecycle_hook_cli.rs`, add an ignored test that creates a temporary `HOME`, runs `codexctl init --plugin-only`, validates all eight generated definitions, then invokes `--lifecycle-hook` 101 times with a fresh turn id. Discard the first sample, sort the remaining durations, and print p50/p95:

```rust
#[test]
#[ignore = "local warm hook latency smoke; not a CI timing gate"]
fn warm_lifecycle_hook_latency_and_roundtrip() {
    let mut samples = Vec::new();
    for index in 0..101 {
        let started = Instant::now();
        let output = run_lifecycle_child(&temp_home, prompt_payload(index));
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        if index > 0 { samples.push(started.elapsed()); }
    }
    samples.sort_unstable();
    let p50 = samples[samples.len() / 2];
    let p95 = samples[samples.len() * 95 / 100];
    eprintln!("warm lifecycle hook latency: p50={p50:?} p95={p95:?}; target <50ms");
}
```

The same test runs `init --remove` and asserts unrelated hook JSON and lifecycle state remain. It never asserts latency and never uses the operator's HOME.

- [ ] **Step 3: Update configuration and troubleshooting documentation**

Document the eight-event matrix, the absolute Home Manager executable, and that installation does not imply trust. Add the imperative downgrade sequence exactly:

```text
codexctl init --remove     # run with the newer binary
# downgrade codexctl
codexctl init              # reinstall the older managed hooks if wanted
```

For Home Manager, document a distinct sequence: revert or remove the lifecycle
hook definitions from the configuration, rebuild/switch to confirm they are
gone, then downgrade the selected `codexctl` package and rebuild again if
needed. Do not direct declarative users through imperative `init --remove` as
the primary rollback.

If an imperative downgrade happened first, direct the operator to restore the
newer binary and run `init --remove`, or manually remove only handlers whose
bare or absolute executable resolves to codexctl and whose exact argument is
`--lifecycle-hook` or `--permission-hook`; preserve lookalikes and neighboring
user hooks. State that expired state is ignored and `init --remove` preserves
it; `init --purge --yes` remains the destructive cleanup path.

Troubleshooting distinguishes missing, stale, disabled, duplicate, unavailable/mismatched, corrupt, newer-schema, and trust-unverified conditions. Present managed-definition health, lifecycle-state health, and trust as separate doctor dimensions. For trust, direct the user to Codex `/hooks`; never claim codexctl can inspect the decision.

- [ ] **Step 4: Run the smoke test manually**

```bash
cargo test --test lifecycle_hook_cli warm_lifecycle_hook_latency_and_roundtrip -- --ignored --exact --nocapture
```

Expected: hook round-trip and removal assertions pass; p50/p95 are printed. If p95 exceeds 50 ms, record it in the task bead and profile before release, but do not make CI timing-dependent.

- [ ] **Step 5: Run final repository gates**

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
nix fmt -- --check .
nix flake check
```

Expected: every command exits 0.

- [ ] **Step 6: Verify scope and hand off**

```bash
jj --no-pager diff --stat --from '<implementation-base-change-id>' --to @
jj --no-pager log -r '(<implementation-base-change-id>..@) & ~empty()' --no-graph -T 'change_id.short() ++ " " ++ description.first_line() ++ "\n"'
bd -C ~/.beads-planning show codexctl-rqm
bd -C ~/.beads-planning lint codexctl-rqm
```

Expected: every non-empty implementation changeset after the recorded base has an emoji conventional task description containing `codexctl-rqm`; automation-created empty interstitial changesets do not affect the audit. No unrelated files changed, all seven child Beads and their dependency edges exist and lint, and the parent feature has evidence for every acceptance criterion. Do not push without explicit user authorization.

## Stress Test Results: Codex Lifecycle Hook Implementation Plan

### Resolved Decisions

1. Task 5 depends on Task 2 because reconciliation consumes store conditions;
   the full dependent-first Beads edge set is explicit.
2. `LifecycleIdentity` is privately represented and validated once; generic
   and permission adapters cannot construct divergent persisted identity.
3. Cross-process tests distinguish accepted writes from lock-timeout
   rejections and test timeout, cleanup, quarantine, and atomicity separately.
4. Generic and permission adapters share the 64 KiB reader, and authorization
   response bytes remain independent of lifecycle persistence.
5. Only strictly newer, non-future transcript timestamps invalidate hook
   evidence; ambiguous timestamps do not close a newer turn.
6. Core owns matching and retention policy; App performs one guarded read only
   when eligible local sessions exist.
7. Doctor reports definition health, lifecycle-state health, and unobservable
   Codex trust separately; imperative and Home Manager rollback paths differ.
8. Execution records a stable implementation baseline and audits all non-empty
   changesets instead of relying on an ancestor count.

### Changes Made

- Corrected the dependency graph and execution-time Beads instructions.
- Tightened identity construction, hook input limits, decision ordering, and
  timestamp reconciliation contracts and tests.
- Made persistence concurrency assertions deterministic and invariant-focused.
- Consolidated core/App responsibilities and transient-error behavior.
- Split doctor dimensions and added declarative rollback coverage.
- Replaced seven-ancestor scope verification with a recorded change-id base and
  task-aware jj description audit.

### Deferred / Parking Lot

- Broader non-Bash authorization remains `codexctl-85x`.
- XDG state migration remains `codexctl-2yk`; this implementation preserves
  the current compatibility path.
- Lifecycle durability still deliberately omits `fsync`; the snapshot remains
  derivative status evidence with safe fallback.

### Reflexion Pass

Re-read all task interfaces, acceptance criteria, RED/GREEN commands, and final
handoff checks after the edits. Verified that Task 5 now consumes Task 2,
permission construction returns validation errors, transcript timestamps are
optional at the parser boundary, doctor no longer conflates structural health
with trust, and no ancestor-count verification remains. No additional design
branch or implementation task was required.

### Confidence Assessment

Overall: High for implementation readiness.

Areas of concern: the remaining risk is concentrated in
cross-platform lock/rename behavior and real Codex hook payload drift; the plan
now isolates both behind fixtures, subprocess tests, and explicit fail-open
behavior before dashboard integration.
