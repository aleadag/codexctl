# Lifecycle Activity and Project Label Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use beads-superpowers:subagent-driven-development (recommended) or beads-superpowers:executing-plans to implement this plan task-by-task. Each Task becomes a bead (`bd create -t task --parent <epic-id>`). Steps within tasks use checkbox (`- [ ]`) syntax for human readability.

**Goal:** Keep lifecycle audit records out of Brain activity lists and render useful project names when persisted activity lacks an explicit label.

**Architecture:** Extend the version-1 activity record with an additive `ActivityKind` enum and have each producer write its semantic kind. Normalize legacy lifecycle IDs while reading, enforce per-activity kind consistency, and filter lifecycle items in the core snapshot projection. Keep project identity unchanged and derive the Live display label from recorded cwd evidence.

**Tech Stack:** Rust, Serde/JSONL persistence, Ratatui, Cargo tests, Jujutsu, Beads

## Global Constraints

- Keep `ACTIVITY_SCHEMA_VERSION` at `1`; the new field must be additive and readable by old binaries.
- `ActivityKind` may affect persistence and UI projection only; authorization must never consult it.
- Preserve raw lifecycle audit rows and diagnostic errors.
- Do not change `.coding-brain/project.toml`; it remains identity-only.
- Follow red-green-refactor: each production behavior starts with a focused failing test.
- Keep all edits in the existing described jj stack; do not push.

---

### Task 1: Persist and Validate Activity Kinds

**Files:**
- Modify: `crates/coding-brain-core/src/brain_activity.rs`
- Modify: `src/brain/activity.rs`
- Modify: `src/brain/permission_hook.rs`
- Modify: `src/lifecycle_hook.rs`
- Modify: `src/brain/outcomes.rs`
- Modify: `src/runtime/brain.rs`
- Modify activity-item fixtures in `crates/coding-brain-tui/src/brain_app.rs` and `crates/coding-brain-tui/src/ui/brain/mod.rs`
- Modify fixtures containing `ActivityEvent` literals in `src/brain/decisions.rs`, `src/commands.rs`, `tests/activity_scale.rs`, `tests/headless_activity.rs`, and `tests/integration_tests.rs`

**Interfaces:**
- Produces: `pub enum ActivityKind { Decision, Lifecycle, Diagnostic }`
- Produces: `ActivityEvent.kind: ActivityKind`
- Produces: `ActivityItem.kind: ActivityKind`
- Consumes: existing `ActivityEvent::normalized`, `ActivityEvent::has_consistent_payload`, and `ActivityStore::read_unlocked`

**Acceptance Criteria:**
- New activity rows serialize `kind` as `decision`, `lifecycle`, or `diagnostic` while retaining schema version 1.
- A legacy `lifecycle_*` row without `kind` reads as `ActivityKind::Lifecycle`.
- Permission decisions and their outcomes/corrections remain `Decision`; generic lifecycle rows are `Lifecycle`; orphan attribution failures are `Diagnostic`.
- Lifecycle rows carrying decision/outcome/correction payloads are rejected.
- Conflicting kinds for one `activity_id` are diagnosed as malformed and cannot silently change the projected kind.

- [ ] **Step 1: Write failing activity-kind serialization and validation tests**

Add focused tests to `crates/coding-brain-core/src/brain_activity.rs`:

```rust
#[test]
fn activity_kind_is_additive_and_serialized() {
    let mut activity = event("cargo test", "safe", "note");
    activity.kind = ActivityKind::Decision;
    let value = serde_json::to_value(activity).unwrap();
    assert_eq!(value["schema_version"], ACTIVITY_SCHEMA_VERSION);
    assert_eq!(value["kind"], "decision");
}

#[test]
fn lifecycle_kind_rejects_decision_evidence() {
    let mut activity = event("cargo test", "safe", "note");
    activity.kind = ActivityKind::Lifecycle;
    assert!(!activity.has_consistent_payload());
}
```

- [ ] **Step 2: Run the core tests and confirm RED**

Run:

```bash
cargo test -p coding-brain-core brain_activity::tests::activity_kind_is_additive_and_serialized
cargo test -p coding-brain-core brain_activity::tests::lifecycle_kind_rejects_decision_evidence
```

Expected: compilation fails because `ActivityKind` and `ActivityEvent.kind` do not exist.

- [ ] **Step 3: Add the kind contract and payload validation**

In `crates/coding-brain-core/src/brain_activity.rs`, add:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    #[default]
    Decision,
    Lifecycle,
    Diagnostic,
}
```

Add `pub kind: ActivityKind` to both `ActivityEvent` and `ActivityItem`, with `#[serde(default)]` on the persisted event field. Extend `has_consistent_payload` so `Lifecycle` accepts only non-decision audit evidence and `Diagnostic` accepts an `Error` without outcome/correction payloads:

```rust
let state_payload_is_valid = match self.state {
    ActivityState::Outcome => {
        self.outcome.is_some() && self.correction.is_none() && self.note.is_none()
    }
    ActivityState::Correction => self.outcome.is_none() && self.correction.is_some(),
    _ => self.outcome.is_none() && self.correction.is_none() && self.note.is_none(),
};
let kind_payload_is_valid = match self.kind {
    ActivityKind::Decision => true,
    ActivityKind::Lifecycle => {
        self.state == ActivityState::Abstained
            && self.decision_id.is_none()
            && self.normalized_command.is_none()
            && self.rule_id.is_none()
            && self.confidence.is_none()
            && self.threshold.is_none()
    }
    ActivityKind::Diagnostic => {
        self.state == ActivityState::Error
            && self.decision_id.is_none()
            && self.outcome.is_none()
            && self.correction.is_none()
    }
};
state_payload_is_valid && kind_payload_is_valid
```

- [ ] **Step 4: Assign kinds at every production boundary**

Use `ActivityKind::Decision` in `HookActivity::event`, correction writes, matched outcomes, and normal runtime/headless activity. Use `ActivityKind::Lifecycle` in `append_observation`. Preserve `matched.kind` for linked outcomes. Use `ActivityKind::Diagnostic` in `append_orphan` and `append_orphan_activity`. Update test-only `ActivityEvent` literals with the kind matching their fixture purpose.

The producer edits are the following field assignments, inserted into their
existing complete literals:

```rust
kind: ActivityKind::Decision,
kind: ActivityKind::Lifecycle,
kind: ActivityKind::Diagnostic,
kind: matched.kind,
```

- [ ] **Step 5: Write failing legacy and mixed-kind reader tests**

Add tests to `src/brain/activity.rs` that write raw schema-v1 JSON without `kind` and that append conflicting kinds for one ID:

```rust
#[test]
fn legacy_lifecycle_ids_are_normalized_on_read() {
    let (root, store) = fixture_store();
    let mut event = event("lifecycle_1", ActivityState::Abstained);
    event.normalized_command = None;
    event.fingerprint = None;
    event.rule_id = None;
    event.confidence = None;
    event.threshold = None;
    event.reasoning = None;
    event.decision_id = None;
    event.tool = Some("SessionStart".into());
    let mut legacy = serde_json::to_value(event).unwrap();
    legacy.as_object_mut().unwrap().remove("kind");
    fs::write(root.path().join("activity.jsonl"), format!("{legacy}\n")).unwrap();
    assert_eq!(store.read().unwrap().events()[0].kind, ActivityKind::Lifecycle);
}

#[test]
fn mixed_activity_kinds_are_diagnosed() {
    let (_root, store) = fixture_store();
    let first = event("same", ActivityState::Denied);
    let mut conflicting = first.clone();
    conflicting.kind = ActivityKind::Diagnostic;
    conflicting.state = ActivityState::Error;
    conflicting.decision_id = None;
    store.append(first).unwrap();
    store.append(conflicting).unwrap();
    let log = store.read().unwrap();
    assert_eq!(log.events().len(), 1);
    assert_eq!(log.diagnostics().malformed_rows, 1);
}
```

- [ ] **Step 6: Run the reader tests and confirm RED**

Run:

```bash
cargo test --lib brain::activity::tests::legacy_lifecycle_ids_are_normalized_on_read
cargo test --lib brain::activity::tests::mixed_activity_kinds_are_diagnosed
```

Expected: the legacy row reads as `Decision`, and the mixed-kind row is retained without a malformed diagnostic.

- [ ] **Step 7: Normalize legacy rows and reject conflicting group kinds**

In `ActivityStore::read_unlocked`, normalize each successfully parsed event before payload validation or storage, then track the first kind per `activity_id`:

```rust
let mut activity_kinds = HashMap::<String, ActivityKind>::new();
// inside the successful ActivityEvent parse branch:
let mut event = event;
if event.activity_id.starts_with("lifecycle_") {
    event.kind = ActivityKind::Lifecycle;
}
if event.schema_version != ACTIVITY_SCHEMA_VERSION || !event.has_consistent_payload() {
    record_malformed(&mut log.diagnostics, offset);
} else if activity_kinds
    .get(&event.activity_id)
    .is_some_and(|kind| *kind != event.kind)
{
    record_malformed(&mut log.diagnostics, offset);
} else {
    activity_kinds.insert(event.activity_id.clone(), event.kind);
    log.events.push(event);
}
```

Keep diagnostic rows and existing schema/payload checks unchanged.

- [ ] **Step 8: Run Task 1 verification**

Run:

```bash
cargo test -p coding-brain-core brain_activity::tests
cargo test --lib brain::activity::tests
cargo test --lib lifecycle_hook::tests
cargo test --lib brain::permission_hook::tests
cargo check --all-targets
```

Expected: all commands pass with no compilation errors or failed tests.

- [ ] **Step 9: Check the jj diff without committing or pushing**

Run:

```bash
jj --no-pager diff --git
jj --no-pager st
```

Expected: only activity-kind contract, producer assignments, compatibility logic, and corresponding fixtures/tests are changed.

### Task 2: Exclude Lifecycle Activity From Core Snapshots

**Files:**
- Modify: `src/brain/activity.rs`

**Interfaces:**
- Consumes: `ActivityItem.kind: ActivityKind` from Task 1
- Produces: `ActivitySnapshot` values containing only decision and diagnostic activities

**Acceptance Criteria:**
- Lifecycle activities remain available from `ActivityStore::read()` but appear in neither `ActivitySnapshot.attention` nor `ActivitySnapshot.recent`.
- Lifecycle activities do not contribute to `unresolved_count` or overflow.
- Ordinary decision abstentions and diagnostic errors remain visible in Needs Attention.

- [ ] **Step 1: Write a failing snapshot projection test**

Add to `src/brain/activity.rs`:

```rust
#[test]
fn lifecycle_activity_is_audited_but_absent_from_live_snapshot() {
    let (_root, store) = fixture_store();
    let mut lifecycle = event("lifecycle_1", ActivityState::Abstained);
    lifecycle.kind = ActivityKind::Lifecycle;
    lifecycle.normalized_command = None;
    lifecycle.fingerprint = None;
    lifecycle.rule_id = None;
    lifecycle.confidence = None;
    lifecycle.threshold = None;
    lifecycle.reasoning = None;
    lifecycle.decision_id = None;
    lifecycle.tool = Some("SessionStart".into());
    store.append(lifecycle).unwrap();
    store.append(event("decision-1", ActivityState::Abstained)).unwrap();
    let mut diagnostic = event("orphan_1", ActivityState::Error);
    diagnostic.kind = ActivityKind::Diagnostic;
    diagnostic.decision_id = None;
    store.append(diagnostic).unwrap();

    assert_eq!(store.read().unwrap().events().len(), 3);
    let snapshot = store.snapshot(SnapshotLimits::default()).unwrap();
    assert_eq!(snapshot.attention.len(), 2);
    assert_eq!(snapshot.unresolved_count, 2);
    assert!(snapshot.recent.is_empty());
    assert!(snapshot.attention.iter().all(|item| item.kind != ActivityKind::Lifecycle));
}
```

- [ ] **Step 2: Run the projection test and confirm RED**

Run:

```bash
cargo test --lib brain::activity::tests::lifecycle_activity_is_audited_but_absent_from_live_snapshot
```

Expected: FAIL because the lifecycle abstention appears in attention and increments `unresolved_count`.

- [ ] **Step 3: Filter lifecycle items before classification**

In `project_snapshot`, immediately after `project_activity`:

```rust
let item = project_activity(&events, limits.interrupted_after_ms, now_ms);
if item.kind == ActivityKind::Lifecycle {
    continue;
}
```

In `project_activity`, copy `source.kind` into the returned `ActivityItem`.

- [ ] **Step 4: Run Task 2 verification**

Run:

```bash
cargo test --lib brain::activity::tests::lifecycle_activity_is_audited_but_absent_from_live_snapshot
cargo test --lib brain::activity::tests
```

Expected: both commands pass; lifecycle rows remain in raw reads but not snapshots.

- [ ] **Step 5: Check the focused diff**

Run:

```bash
jj --no-pager diff --git src/brain/activity.rs
```

Expected: one projection guard, the propagated kind, and focused regression tests.

### Task 3: Derive Missing Project Labels From Recorded Paths

**Files:**
- Modify: `crates/coding-brain-tui/src/ui/brain/live.rs`
- Test: `crates/coding-brain-tui/src/ui/brain/mod.rs`

**Interfaces:**
- Consumes: `ActivityItem.project.label` and `ActivityItem.project.cwd`
- Produces: `fn project_label(item: &ActivityItem) -> Cow<'_, str>`

**Acceptance Criteria:**
- A non-empty explicit project label remains authoritative.
- A missing or empty label displays the cwd basename using lossy UTF-8 conversion.
- A root path displays the full path rather than `unknown project`.
- No project manifest or persisted activity row is rewritten.

- [ ] **Step 1: Write failing Ratatui rendering tests**

In `crates/coding-brain-tui/src/ui/brain/mod.rs`, add:

```rust
#[test]
fn live_derives_missing_project_label_from_cwd() {
    let mut item = activity("attention-1", DeliveryState::Unknown);
    item.project.label = None;
    item.project.cwd = PathBuf::from("/work/codexctl");
    let mock = MockBrainRuntime {
        activity_snapshot: ActivitySnapshot {
            attention: vec![AttentionItem {
                activity: item,
                occurrences: 1,
                unresolved_occurrences: 1,
            }],
            unresolved_count: 1,
            ..ActivitySnapshot::default()
        },
        endpoint_health: online(),
        ..MockBrainRuntime::default()
    };
    let text = render_text(&fixture_app(mock));
    assert!(text.contains("codexctl"));
    assert!(!text.contains("unknown project"));
}

#[test]
fn live_keeps_explicit_label_and_handles_root_cwd() {
    let mut explicit = activity("explicit", DeliveryState::Unknown);
    explicit.project.label = Some("friendly".into());
    explicit.project.cwd = PathBuf::from("/work/ignored");
    assert_eq!(live::project_label(&explicit), "friendly");

    explicit.project.label = None;
    explicit.project.cwd = PathBuf::from("/");
    assert_eq!(live::project_label(&explicit), "/");
}
```

Expose `project_label` as `pub(super)` so the parent UI test module can verify edge cases directly.

- [ ] **Step 2: Run the TUI tests and confirm RED**

Run:

```bash
cargo test -p coding-brain-tui ui::brain::tests::live_derives_missing_project_label_from_cwd
cargo test -p coding-brain-tui ui::brain::tests::live_keeps_explicit_label_and_handles_root_cwd
```

Expected: the first test renders `unknown project`; the second fails because `project_label` is private and returns a borrowed `&str` without cwd fallback.

- [ ] **Step 3: Implement the display fallback**

In `crates/coding-brain-tui/src/ui/brain/live.rs`, import `std::borrow::Cow` and replace the helper with:

```rust
pub(super) fn project_label(item: &ActivityItem) -> Cow<'_, str> {
    if let Some(label) = item.project.label.as_deref().filter(|label| !label.is_empty()) {
        return Cow::Borrowed(label);
    }
    if let Some(name) = item.project.cwd.file_name() {
        return name.to_string_lossy();
    }
    let path = item.project.cwd.to_string_lossy();
    if path.is_empty() {
        Cow::Borrowed("unknown project")
    } else {
        path
    }
}
```

The existing formatting sites accept `Cow<'_, str>` through its `Display`
implementation, so their call structure remains unchanged.

- [ ] **Step 4: Run Task 3 verification**

Run:

```bash
cargo test -p coding-brain-tui ui::brain::tests
```

Expected: all Brain UI tests pass, including explicit, basename, and root label cases.

- [ ] **Step 5: Run repository quality gates**

Run:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build
```

Expected: all four commands exit successfully with no warnings.

- [ ] **Step 6: Final jj and Beads verification**

Run:

```bash
jj --no-pager diff --git
jj --no-pager st
```

Expected: the diff is limited to the approved spec/plan, activity-kind contract and producers, snapshot projection/tests, and Live label rendering/tests. Close the implementation task beads only after their acceptance criteria and quality gates pass; do not push.
