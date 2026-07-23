# PostToolUse Outcome Telemetry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use beads-superpowers:subagent-driven-development (recommended) or beads-superpowers:executing-plans to implement this plan task-by-task. Reuse and claim the existing epic/tasks listed below; do not create or import duplicate beads. Steps within tasks use checkbox (`- [ ]`) syntax for human readability.
>
> **Beads epic:** `codexctl-i0y.3` (tasks already created and dependency-wired)

**Goal:** Make current Codex PostToolUse events observable and safely attributable to delivered Bash decisions even though PermissionRequest omits `tool_use_id`.

**Architecture:** Advance the append-only activity format to schema v2 with a neutral `Completed` outcome while retaining v1 reads and row versions. Record every PostToolUse independently, correlate by exact IDs first and then by a unique PreToolUse-anchored Bash interval, expose bounded zero-coverage checks in Doctor, and make Live wording reflect the evidence actually available.

**Tech Stack:** Rust 2024, Serde/serde_json, append-only JSONL activity store, Cargo workspace tests, Ratatui.

## Global Constraints

- Exact `(session_id, turn_id, tool_use_id)` correlation always takes precedence.
- Exact and fallback correlation require the Decision lifecycle's first terminal state to be Allowed; Denied, Abstained, and Error are never eligible.
- Bash fallback requires a unique matching PreToolUse anchor, a lossless normalized command that was neither redacted nor truncated, and exactly one matching terminal Decision before the next same-turn PreToolUse.
- Ambiguity fails closed: append no Outcome and emit only metadata-safe diagnostics.
- Never persist raw PostToolUse commands, responses, command hashes, or fingerprints.
- Opaque responses map to neutral `ActivityOutcome::Completed`; only explicit structured evidence maps to Succeeded, Failed, or Cancelled.
- Activity writers emit schema v2; readers accept v1 and v2; compaction preserves retained row schema versions.
- Downgrade after v2 writes is unsupported; there is no destructive migration or historical backfill.
- Doctor examines at most 100 unique lifecycle invocations and 20 unique eligible decisions, with 10-PreToolUse and 5-decision minimums.
- Delivered rows without Outcome say `allowed · response delivered`; Unknown and Failed delivery retain `execution not confirmed`.
- Permission enforcement and hook protocol output remain unchanged.
- Do not commit, push, publish, or merge without explicit user authorization; commit commands below are approval-gated checkpoints.

---

## File Structure

- `crates/coding-brain-core/src/brain_activity.rs`: activity schema constants, neutral Completed outcome, and the shared bounded/redacted command normalizer.
- `src/brain/activity.rs`: v1/v2 JSONL reading, v2-only appends, schema-preserving compaction, neutral Completed projection behavior, and atomic snapshot-plus-batch appends.
- `src/brain/permission_hook.rs`: consume the shared command normalizer so PermissionRequest and PostToolUse compare identical representations.
- `src/lifecycle_hook.rs`: independent PostToolUse observation, exact/fallback correlation, classification, idempotency, and metadata-only diagnostics.
- `src/doctor.rs`: bounded activity telemetry health check and exact threshold behavior.
- `crates/coding-brain-tui/src/ui/brain/live.rs`: Completed label and evidence-sensitive delivery wording.
- `crates/coding-brain-tui/src/ui/brain/mod.rs`: rendered Live status regressions.
- `tests/hook_activity.rs`: real-binary PermissionRequest-without-ID → PostToolUse regression.
- `CHANGELOG.md`: Unreleased operator-facing behavior and schema downgrade boundary.

### Task 1: Activity schema v2 and neutral completion (`codexctl-e0s`)

**Files:**
- Modify: `crates/coding-brain-core/src/brain_activity.rs:7-205`
- Modify: `src/brain/activity.rs:1-430,512-730,823-1425`
- Modify: `src/brain/permission_hook.rs:8-15,141-205`
- Modify: `crates/coding-brain-tui/src/ui/brain/live.rs:159-167` (exhaustive `Completed` arm only; Task 4 retains wording/tests)

**Interfaces:**
- Consumes: existing `ActivityEvent`, `ActivityOutcome`, `ActivityStore`, and PermissionRequest command persistence.
- Produces: `ACTIVITY_SCHEMA_VERSION == 2`, `MIN_ACTIVITY_SCHEMA_VERSION == 1`, `ActivityOutcome::Completed`, `bounded_activity_identifier(&str) -> String`, `lossless_redacted_activity_text(&str) -> Option<String>`, and `ActivityStore::append_from_snapshot(F)`.

**Acceptance Criteria:**
- New rows use schema v2, while v1 and v2 events and diagnostic rows can coexist in one readable log.
- Compaction preserves each retained event's original schema version and never upgrades v1 rows.
- Completed confirms execution but is neutral: it resolves the activity, is not failed attention, and does not supersede another activity as success.
- PermissionRequest command persistence uses the same shared bounded/redacted function later used by PostToolUse.
- Appending a v1 event through the current writer remains rejected.
- Concurrent snapshot-plus-batch operations serialize under the existing exclusive lock and each closure sees prior committed writes.
- The TUI's exhaustive Outcome match accepts Completed so schema-focused workspace tests compile; no other Live wording changes occur in this task.

- [ ] **Step 0: Establish the focused baseline**

Run these separately before editing:

```bash
cargo test -p coding-brain-core brain_activity
cargo test -p coding-brain brain::activity
cargo test -p coding-brain brain::permission_hook
cargo test -p coding-brain-tui ui::brain::tests::outcome_is_the_only_execution_confirmation
```

Expected: all existing focused tests pass. If they do not, stop and record the pre-existing failure on `codexctl-e0s` before changing code.

- [ ] **Step 1: Write failing schema and projection tests**

In `crates/coding-brain-core/src/brain_activity.rs`, add serialization and normalizer coverage:

```rust
#[test]
fn schema_v2_serializes_neutral_completed() {
    let mut activity = event("cargo test", "safe", "note");
    activity.state = ActivityState::Outcome;
    activity.outcome = Some(ActivityOutcome::Completed);
    let value = serde_json::to_value(activity).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["outcome"], "completed");
}

#[test]
fn shared_command_normalizer_redacts_and_bounds() {
    let command = format!("curl -H 'Authorization: Bearer secret' {}", "x".repeat(5000));
    assert_eq!(lossless_redacted_activity_text(&command), None);
    assert_eq!(bounded_activity_identifier(&"x".repeat(5000)).len(), MAX_ACTIVITY_FIELD_BYTES);
}
```

In `src/brain/activity.rs`, add tests that write one hand-serialized v1 row and one v2 row, then compact and re-read:

```rust
#[test]
fn mixed_v1_v2_rows_read_and_compact_without_version_rewrite() {
    let (temp, store) = fixture_store();
    let store = store.with_limits(ActivityLimits {
        compact_at_bytes: 1,
        retained_lifecycles: 10,
        ..ActivityLimits::default()
    });
    let mut v1 = event_at("v1", ActivityState::Allowed, 1);
    v1.schema_version = 1;
    std::fs::write(
        temp.path().join("activity.jsonl"),
        format!("{}\n", serde_json::to_string(&v1).unwrap()),
    ).unwrap();
    store.append(event_at("v2", ActivityState::Allowed, 2)).unwrap();

    let versions = store.read().unwrap().events().iter()
        .map(|event| event.schema_version).collect::<Vec<_>>();
    assert_eq!(versions, [1, 2]);
    assert!(store.compact_if_needed().unwrap());
    let versions = store.read().unwrap().events().iter()
        .map(|event| event.schema_version).collect::<Vec<_>>();
    assert_eq!(versions, [1, 2]);
}

#[test]
fn completed_is_neutral_and_does_not_supersede() {
    let (_, store) = fixture_store();
    store.append(event_at("denied", ActivityState::Denied, 1)).unwrap();
    store.append(event_at("denied", ActivityState::DeliveryFailed, 2)).unwrap();
    let mut completed = event_at("completed", ActivityState::Allowed, 3);
    completed.state = ActivityState::Outcome;
    completed.outcome = Some(ActivityOutcome::Completed);
    completed.supersedes = Some("denied".into());
    store.append(completed).unwrap();
    let snapshot = store.snapshot(SnapshotLimits::default()).unwrap();
    assert!(snapshot.attention.iter().any(|item| item.activity_id == "denied"));
}

#[test]
fn v1_diagnostic_rows_remain_readable() {
    let (temp, store) = fixture_store();
    std::fs::write(
        temp.path().join("activity.jsonl"),
        b"{\"schema_version\":1,\"diagnostic\":{\"kind\":\"malformed_rows\",\"count\":2}}\n",
    ).unwrap();
    assert_eq!(store.read().unwrap().diagnostics().malformed_rows, 2);
}

#[test]
fn current_writer_rejects_v1_and_reader_diagnoses_v3() {
    let (temp, store) = fixture_store();
    let mut v1 = event_at("v1", ActivityState::Allowed, 1);
    v1.schema_version = 1;
    assert!(matches!(store.append(v1), Err(ActivityStoreError::UnsupportedSchema(1))));

    let mut v3 = event_at("v3", ActivityState::Allowed, 3);
    v3.schema_version = 3;
    std::fs::write(
        temp.path().join("activity.jsonl"),
        format!("{}\n", serde_json::to_string(&v3).unwrap()),
    ).unwrap();
    assert_eq!(store.read().unwrap().diagnostics().malformed_rows, 1);
}

#[test]
fn v1_decision_and_v2_outcome_project_and_compact_together() {
    let (temp, store) = fixture_store();
    let store = store.with_limits(ActivityLimits {
        compact_at_bytes: 1,
        retained_lifecycles: 10,
        ..ActivityLimits::default()
    });
    let mut decision = event_at("mixed", ActivityState::Allowed, 1);
    decision.schema_version = 1;
    std::fs::write(
        temp.path().join("activity.jsonl"),
        format!("{}\n", serde_json::to_string(&decision).unwrap()),
    ).unwrap();
    let mut outcome = event_at("mixed", ActivityState::Outcome, 2);
    outcome.outcome = Some(ActivityOutcome::Completed);
    store.append(outcome).unwrap();
    assert_eq!(store.snapshot(SnapshotLimits::default()).unwrap().recent[0].outcome,
        Some(ActivityOutcome::Completed));
    assert!(store.compact_if_needed().unwrap());
    assert_eq!(store.read().unwrap().events().iter()
        .map(|event| event.schema_version).collect::<Vec<_>>(), [1, 2]);
}
```

- [ ] **Step 2: Run the focused tests and confirm the intended failures**

Run:

```bash
cargo test -p coding-brain-core brain_activity::tests::schema_v2_serializes_neutral_completed
cargo test -p coding-brain brain::activity::tests::mixed_v1_v2_rows_read_and_compact_without_version_rewrite
cargo test -p coding-brain brain::activity::tests::completed_is_neutral_and_does_not_supersede
```

Expected: compilation fails because `Completed`, `MIN_ACTIVITY_SCHEMA_VERSION`, or `bounded_redacted_activity_text` does not exist, and the v1 reader test fails under the current exact-schema check.

- [ ] **Step 3: Implement the minimal compatible schema changes**

In `brain_activity.rs`, expose one shared normalizer and add Completed:

```rust
pub const MIN_ACTIVITY_SCHEMA_VERSION: u32 = 1;
pub const ACTIVITY_SCHEMA_VERSION: u32 = 2;

pub enum ActivityOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Completed,
}

pub fn bounded_activity_identifier(value: &str) -> String {
    bounded(value, false)
}

pub fn lossless_redacted_activity_text(value: &str) -> Option<String> {
    let redacted = redact_activity_text(value);
    (!redacted.contains("[REDACTED]") && redacted.len() <= MAX_ACTIVITY_FIELD_BYTES)
        .then_some(redacted)
}
```

Keep `ActivityEvent::normalized` calling the same private `bounded` primitive. PermissionRequest must continue storing bounded/redacted commands even when they are lossy, so expose a separate `bounded_redacted_activity_text(&str) -> String` wrapper for that existing persistence behavior. In `permission_hook.rs`, remove its duplicate normalizer and import/call the shared wrapper for commands, diagnostics, and reasoning. PostToolUse fallback alone uses the lossless `Option` helper.

In `ActivityStore`, keep `append` v2-only but accept the inclusive supported range when reading events and diagnostic rows:

```rust
fn supported_activity_schema(version: u32) -> bool {
    (MIN_ACTIVITY_SCHEMA_VERSION..=ACTIVITY_SCHEMA_VERSION).contains(&version)
}

if !supported_activity_schema(event.schema_version)
    || !event.has_consistent_payload()
    || activity_kinds.get(&event.activity_id).is_some_and(|kind| *kind != event.kind)
{
    record_malformed(&mut log.diagnostics, offset);
} else {
    activity_kinds.insert(event.activity_id.clone(), event.kind);
    log.events.push(event);
}
```

Update `apply_diagnostic` with the same range check. Do not change compaction serialization: writing each retained `ActivityEvent` directly is what preserves its original `schema_version`. Keep supersession limited to `ActivityOutcome::Succeeded`; Completed will resolve its own grouped activity through `item.outcome.is_some()` without becoming positive evidence.

Refactor the current locked append body into a private `append_events_unlocked(&[ActivityEvent])` helper, then add this one-use transaction boundary:

```rust
pub(crate) fn append_from_snapshot<F>(&self, build: F) -> Result<(), ActivityStoreError>
where
    F: FnOnce(&ActivityLog) -> Vec<ActivityEvent>,
{
    let lock = self.open_lock()?;
    let _guard = lock_with_timeout(&lock, self.limits.lock_timeout_ms, LockKind::Exclusive)?;
    let log = self.read_unlocked()?;
    let events = build(&log);
    self.append_events_unlocked(&events)
}
```

Normalize, validate, and serialize the entire returned batch before writing its first byte. Repair a crash tail once, write rows in vector order, flush, and sync once. This guarantees PostToolUse observation precedes its optional Outcome while the exclusive lock makes the read/check/append idempotent across processes. A crash may leave the first complete row plus a partial second row; the existing tail repair preserves that observation on the next write.

Add this two-thread store test; each closure appends a marker but only appends an Outcome when its locked snapshot lacks one:

```rust
#[test]
fn append_from_snapshot_serializes_concurrent_idempotency_checks() {
    let (_, store) = fixture_store();
    store.append(event_at("target", ActivityState::Allowed, 1)).unwrap();
    let store = std::sync::Arc::new(store);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles = (0..2).map(|index| {
        let store = store.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let marker = event_at(&format!("marker-{index}"), ActivityState::Observed, index + 2);
            let mut outcome = event_at("target", ActivityState::Outcome, index + 4);
            outcome.outcome = Some(ActivityOutcome::Completed);
            barrier.wait();
            store.append_from_snapshot(|log| {
                let mut rows = vec![marker];
                if !log.events().iter().any(|event| {
                    event.activity_id == "target" && event.state == ActivityState::Outcome
                }) {
                    rows.push(outcome);
                }
                rows
            }).unwrap();
        })
    }).collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }
    let events = store.read().unwrap().events().to_vec();
    assert_eq!(events.iter().filter(|event| event.activity_id.starts_with("marker-")).count(), 2);
    assert_eq!(events.iter().filter(|event| event.state == ActivityState::Outcome).count(), 1);
}
```

- [ ] **Step 4: Run schema, storage, and permission-hook tests**

Run:

```bash
cargo test -p coding-brain-core brain_activity
cargo test -p coding-brain brain::activity
cargo test -p coding-brain brain::permission_hook
```

Expected: all selected tests pass; the mixed log reports zero malformed rows and retains `[1, 2]` after compaction.

- [ ] **Step 5: Review checkpoint**

Run `git diff --check` and `git diff -- crates/coding-brain-core/src/brain_activity.rs src/brain/activity.rs src/brain/permission_hook.rs`. Do not commit unless the user explicitly authorizes it. If authorized, use: `git add crates/coding-brain-core/src/brain_activity.rs src/brain/activity.rs src/brain/permission_hook.rs && git commit -m "🗃️ feat: support neutral completed activity outcomes"`.

### Task 2: PreToolUse-anchored PostToolUse correlation (`codexctl-tjf`)

**Files:**
- Modify: `src/lifecycle_hook.rs:55-290,334-790`

**Interfaces:**
- Consumes: `ActivityLog::events()`, `bounded_redacted_activity_text`, schema v2, and `ActivityOutcome::Completed` from Task 1.
- Produces: `correlate_outcome(&ActivityLog, &LifecycleEvent, &LifecycleActivityInput) -> Correlation`, exact-ID-first correlation, anchored Bash fallback, an atomic observation-first batch, and structured response classification.

**Acceptance Criteria:**
- Every valid PostToolUse appends a lifecycle observation when the activity store is writable, even when correlation finds no decision or is ambiguous; storage failures remain diagnostic and hook-protocol fail-open.
- Exact stable IDs still correlate first and duplicate PostToolUse delivery is idempotent.
- Fallback requires Bash, non-empty `tool_input.command`, lossless normalization with neither redaction nor truncation, a unique PreToolUse anchor by normalized Post `tool_use_id`, and exactly one command-matching terminal Decision before the next same-turn PreToolUse.
- Interleaved PreToolUse events, repeated identical commands, and redaction collisions never guess an Outcome.
- Opaque strings and unknown structures map to Completed; explicit structured success, failure, and cancellation remain distinct.
- No lifecycle or diagnostic row persists raw input, response, command hash, or fingerprint.
- A large-log fixture proves tail correlation correctness without a timing assertion.
- Concurrent duplicate PostToolUse calls append two observations but exactly one Outcome.
- Exact and fallback paths refuse Denied, Abstained, and Error decisions; Delivered is not required after Allowed.

- [ ] **Step 0: Establish the focused baseline**

Run `cargo test -p coding-brain lifecycle_hook::tests -- --nocapture` before editing. Expected: all existing lifecycle-hook tests pass.

- [ ] **Step 1: Write failing classification and observation tests**

Extend `LifecycleActivityInput` fixtures to include `tool_input`. Add table-driven classification coverage:

```rust
#[test]
fn outcome_classification_requires_explicit_structured_evidence() {
    let cases = [
        (serde_json::json!("opaque unified-exec response"), ActivityOutcome::Completed),
        (serde_json::json!({"exit_code": 0}), ActivityOutcome::Succeeded),
        (serde_json::json!({"success": true}), ActivityOutcome::Succeeded),
        (serde_json::json!({"exit_code": 7}), ActivityOutcome::Failed),
        (serde_json::json!({"is_error": true}), ActivityOutcome::Failed),
        (serde_json::json!({"cancelled": true}), ActivityOutcome::Cancelled),
        (serde_json::json!({"status": "cancelled"}), ActivityOutcome::Cancelled),
        (serde_json::json!({"message": "done"}), ActivityOutcome::Completed),
    ];
    for (response, expected) in cases {
        assert_eq!(normalized_outcome(Some(&response)), expected);
    }
}
```

Change the exact-ID test to expect both a PostToolUse lifecycle row and an Outcome row. Add a no-decision test that expects both PreToolUse and PostToolUse observations and no diagnostic.

Use these concrete test helpers rather than inventing new fixture interfaces during execution:

```rust
fn decision_event(
    cwd: &Path,
    activity_id: &str,
    recorded_at_ms: u64,
    tool_use_id: Option<&str>,
    command: &str,
    state: ActivityState,
) -> ActivityEvent {
    let project_id = ProjectId::Temporary("project-1".into());
    ActivityEvent {
        schema_version: ACTIVITY_SCHEMA_VERSION,
        kind: ActivityKind::Decision,
        activity_id: activity_id.into(),
        recorded_at_ms,
        project: ProjectEvidence {
            project_id: project_id.clone(),
            cwd: cwd.to_path_buf(),
            label: Some("project".into()),
        },
        session: Some(SessionTarget {
            session_id: "session-1".into(),
            turn_id: Some("turn-1".into()),
            tool_use_id: tool_use_id.map(str::to_owned),
            project_id,
            cwd: cwd.to_path_buf(),
            provider_hints: Vec::new(),
        }),
        state,
        tool: Some("Bash".into()),
        normalized_command: Some(bounded_redacted_activity_text(command)),
        fingerprint: None,
        rule_id: None,
        confidence: Some(0.9),
        threshold: Some(0.6),
        reasoning: Some("safe".into()),
        decision_id: Some(format!("decision-{activity_id}")),
        outcome: None,
        correction: None,
        note: None,
        supersedes: None,
    }
}

fn hook_payload(cwd: &Path, event: &str, call: &str, command: &str, response: Option<Value>) -> Value {
    let mut value = serde_json::json!({
        "session_id": "session-1",
        "turn_id": "turn-1",
        "cwd": cwd,
        "hook_event_name": event,
        "tool_name": "Bash",
        "tool_use_id": call,
        "tool_input": {"command": command}
    });
    if let Some(response) = response {
        value["tool_response"] = response;
    }
    value
}

fn invoke_activity_hook(
    lifecycle: &LifecycleStore,
    activity: &ActivityStore,
    payload: Value,
) -> String {
    let mut stderr = Vec::new();
    run_with_activity(
        Cursor::new(payload.to_string()),
        Vec::new(),
        &mut stderr,
        lifecycle,
        Some(activity),
    );
    String::from_utf8(stderr).unwrap()
}
```

- [ ] **Step 2: Write failing anchored-correlation adversarial tests**

Add helpers local to the test module for appending lifecycle observations and terminal decisions. Cover the successful missing-ID path with this event order:

```text
PreToolUse(call-1, session-1, turn-1)
Decision(activity-1, no tool_use_id, Bash, "cargo test")
PostToolUse(call-1, Bash, "cargo test", opaque response)
```

Assert the appended Outcome targets `activity-1`, copies `call-1` into its session evidence, and is Completed. Then add separate tests for:

```text
Pre(call-1), Pre(call-2), Decision("cargo test"), Post(call-1)  => no Outcome
Pre(call-1), Decision A("cargo test"), Decision B("cargo test"), Post(call-1) => no Outcome
Pre(call-1), Decision("curl --token alpha"), Post("curl --token beta") => no Outcome after redaction collision
Pre(call-1), Decision("cargo test"), Post(call-1), Post(call-1) => exactly one Outcome
```

For every ambiguous case, assert a metadata-only Diagnostic with `normalized_command`, `fingerprint`, and `note` all `None`, and assert the raw commands/responses are absent from serialized activity JSON.

Write the successful fallback test in full:

```rust
#[test]
fn post_tool_use_falls_back_within_unique_pre_interval() {
    let temp = tempfile::tempdir().unwrap();
    let lifecycle = LifecycleStore::at(temp.path().join("lifecycle"));
    let activity = ActivityStore::at(temp.path().join("activity.jsonl"));
    assert!(invoke_activity_hook(
        &lifecycle,
        &activity,
        hook_payload(temp.path(), "PreToolUse", "call-1", "cargo test", None),
    ).is_empty());
    activity.append(decision_event(
        temp.path(), "activity-1", 2, None, "cargo test", ActivityState::Allowed,
    )).unwrap();

    let stderr = invoke_activity_hook(
        &lifecycle,
        &activity,
        hook_payload(
            temp.path(),
            "PostToolUse",
            "call-1",
            "cargo test",
            Some(serde_json::json!("opaque unified-exec response")),
        ),
    );

    assert!(stderr.is_empty(), "{stderr}");
    let events = activity.read().unwrap().events().to_vec();
    let outcome = events.iter().find(|event| event.state == ActivityState::Outcome).unwrap();
    assert_eq!(outcome.activity_id, "activity-1");
    assert_eq!(outcome.schema_version, ACTIVITY_SCHEMA_VERSION);
    assert_eq!(outcome.outcome, Some(ActivityOutcome::Completed));
    assert_eq!(outcome.session.as_ref().unwrap().tool_use_id.as_deref(), Some("call-1"));
    assert!(events.iter().any(|event| event.kind == ActivityKind::Lifecycle && event.tool.as_deref() == Some("PostToolUse")));
}
```

Use this counter and implement every adversarial case explicitly:

```rust
fn outcome_and_diagnostic_counts(store: &ActivityStore) -> (usize, usize) {
    let events = store.read().unwrap().events().to_vec();
    (
        events.iter().filter(|event| event.state == ActivityState::Outcome).count(),
        events.iter().filter(|event| event.kind == ActivityKind::Diagnostic).count(),
    )
}

#[test]
fn interleaved_pre_tools_do_not_guess() {
    let temp = tempfile::tempdir().unwrap();
    let lifecycle = LifecycleStore::at(temp.path().join("lifecycle"));
    let activity = ActivityStore::at(temp.path().join("activity.jsonl"));
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PreToolUse", "call-1", "cargo test", None));
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PreToolUse", "call-2", "cargo test", None));
    activity.append(decision_event(temp.path(), "activity-1", 3, None, "cargo test", ActivityState::Allowed)).unwrap();
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PostToolUse", "call-1", "cargo test", Some(serde_json::json!("done"))));
    assert_eq!(outcome_and_diagnostic_counts(&activity), (0, 1));
}

#[test]
fn repeated_identical_decisions_are_ambiguous() {
    let temp = tempfile::tempdir().unwrap();
    let lifecycle = LifecycleStore::at(temp.path().join("lifecycle"));
    let activity = ActivityStore::at(temp.path().join("activity.jsonl"));
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PreToolUse", "call-1", "cargo test", None));
    for (index, id) in ["activity-a", "activity-b"].into_iter().enumerate() {
        activity.append(decision_event(temp.path(), id, index as u64 + 2, None, "cargo test", ActivityState::Allowed)).unwrap();
    }
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PostToolUse", "call-1", "cargo test", Some(serde_json::json!("done"))));
    assert_eq!(outcome_and_diagnostic_counts(&activity), (0, 1));
}

#[test]
fn lossy_commands_do_not_correlate() {
    for (decision_command, post_command) in [
        ("curl --token alpha".to_string(), "curl --token beta".to_string()),
        (
            format!("{}a", "x".repeat(MAX_ACTIVITY_FIELD_BYTES)),
            format!("{}b", "x".repeat(MAX_ACTIVITY_FIELD_BYTES)),
        ),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let lifecycle = LifecycleStore::at(temp.path().join("lifecycle"));
        let activity = ActivityStore::at(temp.path().join("activity.jsonl"));
        invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PreToolUse", "call-1", &decision_command, None));
        activity.append(decision_event(temp.path(), "activity-1", 2, None, &decision_command, ActivityState::Allowed)).unwrap();
        invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PostToolUse", "call-1", &post_command, Some(serde_json::json!("done"))));
        assert_eq!(outcome_and_diagnostic_counts(&activity), (0, 1));
    }
}

#[test]
fn duplicate_post_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let lifecycle = LifecycleStore::at(temp.path().join("lifecycle"));
    let activity = ActivityStore::at(temp.path().join("activity.jsonl"));
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PreToolUse", "call-1", "cargo test", None));
    activity.append(decision_event(temp.path(), "activity-1", 2, None, "cargo test", ActivityState::Allowed)).unwrap();
    let post = hook_payload(temp.path(), "PostToolUse", "call-1", "cargo test", Some(serde_json::json!("done")));
    invoke_activity_hook(&lifecycle, &activity, post.clone());
    invoke_activity_hook(&lifecycle, &activity, post);
    assert_eq!(outcome_and_diagnostic_counts(&activity).0, 1);
    assert_eq!(activity.read().unwrap().events().iter().filter(|event| event.tool.as_deref() == Some("PostToolUse")).count(), 2);
}

#[test]
fn non_allowed_terminal_states_never_receive_outcomes() {
    for state in [ActivityState::Denied, ActivityState::Abstained, ActivityState::Error] {
        let temp = tempfile::tempdir().unwrap();
        let lifecycle = LifecycleStore::at(temp.path().join("lifecycle"));
        let activity = ActivityStore::at(temp.path().join("activity.jsonl"));
        invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PreToolUse", "call-1", "cargo test", None));
        activity.append(decision_event(temp.path(), "activity-1", 2, None, "cargo test", state)).unwrap();
        invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PostToolUse", "call-1", "cargo test", Some(serde_json::json!("done"))));
        assert_eq!(outcome_and_diagnostic_counts(&activity).0, 0);
    }
}

#[test]
fn oversized_ids_use_the_same_bounded_comparison_form() {
    let temp = tempfile::tempdir().unwrap();
    let lifecycle = LifecycleStore::at(temp.path().join("lifecycle"));
    let activity = ActivityStore::at(temp.path().join("activity.jsonl"));
    let call = "c".repeat(MAX_ACTIVITY_FIELD_BYTES + 100);
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PreToolUse", &call, "cargo test", None));
    activity.append(decision_event(temp.path(), "activity-1", 2, None, "cargo test", ActivityState::Allowed)).unwrap();
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PostToolUse", &call, "cargo test", Some(serde_json::json!("done"))));
    let events = activity.read().unwrap().events().to_vec();
    assert_eq!(events.iter().filter(|event| event.state == ActivityState::Outcome).count(), 1);
    assert!(events.iter().filter_map(|event| event.session.as_ref())
        .filter_map(|session| session.tool_use_id.as_ref())
        .all(|id| id.len() <= MAX_ACTIVITY_FIELD_BYTES));
}
```

Add the same non-Allowed table to the exact-ID path by passing `Some("call-1")` to `decision_event`. For every Diagnostic row, assert `normalized_command`, `fingerprint`, and `note` are `None`; serialize the rows and assert secret command values and response strings are absent.

- [ ] **Step 3: Run lifecycle-hook tests and confirm failure**

Run `cargo test -p coding-brain lifecycle_hook::tests -- --nocapture`.

Expected: the new tests fail because PostToolUse currently skips its lifecycle observation, exact matching is the only strategy, string responses become Succeeded, and duplicates append repeated Outcomes.

- [ ] **Step 4: Implement one-read observation and correlation flow**

Deserialize `tool_input` without persisting it, normalize IDs before correlation, and make lossless command eligibility explicit:

```rust
#[derive(Debug, Deserialize)]
struct LifecycleActivityInput {
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(default)]
    tool_input: Value,
    #[serde(default)]
    tool_response: Option<Value>,
}

impl LifecycleActivityInput {
    fn normalized_bash_command(&self) -> Option<String> {
        (self.tool_name.as_deref() == Some("Bash"))
            .then(|| self.tool_input.get("command")?.as_str())
            .flatten()
            .filter(|command| !command.trim().is_empty())
            .and_then(lossless_redacted_activity_text)
    }

    fn normalized_tool_use_id(&self) -> Option<String> {
        self.tool_use_id.as_deref().map(bounded_activity_identifier)
    }
}
```

Replace `append_outcome`/`append_orphan` side effects with a pure correlation result:

```rust
enum Correlation {
    Outcome(ActivityEvent),
    Diagnostic { event: ActivityEvent, message: &'static str },
    None,
}

let observation = match observation_event(&event, &activity_input) {
    Ok(observation) => observation,
    Err(error) => {
        write_diagnostic(&mut stderr, error);
        return;
    }
};
let mut correlation_message = None;
let result = activity.append_from_snapshot(|log| {
    let mut events = vec![observation];
    match correlate_outcome(log, &event, &activity_input) {
        Correlation::Outcome(outcome) => events.push(outcome),
        Correlation::Diagnostic { event, message } => {
            correlation_message = Some(message);
            events.push(event);
        }
        Correlation::None => {}
    }
    events
});
if let Err(error) = result {
    write_diagnostic(&mut stderr, error);
} else if let Some(message) = correlation_message {
    write_diagnostic(&mut stderr, message);
}
let _ = activity.compact_if_needed();
```

Implement exact matching first. Every candidate lifecycle must have Allowed as its first terminal state; Denied, Abstained, and Error are ineligible, while a missing Delivered row is allowed. Every newly built Outcome sets `schema_version: ACTIVITY_SCHEMA_VERSION`, including Outcomes targeting v1 Decisions. If exact matching finds no candidate, require `normalized_bash_command()`; `None` covers non-Bash, empty, redacted, and truncated commands. Locate exactly one PreToolUse lifecycle row with the normalized Post identity, calculate the half-open event-index interval `(pre_index, next_same_turn_pre_index)`, and collect unique eligible Decision activity IDs inside it. Require `tool == "Bash"`, identical normalized command, first terminal Allowed, and `decision_id`. If the candidate count is not one, fail closed. Return `Correlation::None` when the locked snapshot already contains an Outcome for the same activity/Post identity, making concurrent retries idempotent.

Implement classification with precedence Cancelled → Failed → Succeeded → Completed:

```rust
fn normalized_outcome(response: Option<&Value>) -> ActivityOutcome {
    let Some(Value::Object(response)) = response else {
        return ActivityOutcome::Completed;
    };
    let status = response.get("status").and_then(Value::as_str);
    if response.get("cancelled").and_then(Value::as_bool) == Some(true)
        || matches!(status, Some("cancelled" | "canceled"))
    {
        ActivityOutcome::Cancelled
    } else if response.get("is_error").and_then(Value::as_bool) == Some(true)
        || response.get("exit_code").and_then(Value::as_i64).is_some_and(|code| code != 0)
        || response.get("success").and_then(Value::as_bool) == Some(false)
        || matches!(status, Some("failed" | "error"))
    {
        ActivityOutcome::Failed
    } else if response.get("exit_code").and_then(Value::as_i64) == Some(0)
        || response.get("success").and_then(Value::as_bool) == Some(true)
        || response.get("is_error").and_then(Value::as_bool) == Some(false)
        || matches!(status, Some("succeeded" | "success"))
    {
        ActivityOutcome::Succeeded
    } else {
        ActivityOutcome::Completed
    }
}
```

- [ ] **Step 5: Add the large-log correctness fixture**

Append at least 10,000 irrelevant lifecycle/decision events, then append the unique tail Pre → Decision → Post sequence. Assert only the tail Decision gets one Outcome. Do not measure elapsed time and do not add an index.

Add the concurrent regression with independently constructed store handles sharing one path:

```rust
#[test]
fn concurrent_duplicate_post_appends_one_outcome() {
    let temp = tempfile::tempdir().unwrap();
    let lifecycle_path = temp.path().join("lifecycle");
    let activity_path = temp.path().join("activity.jsonl");
    let lifecycle = LifecycleStore::at(&lifecycle_path);
    let activity = ActivityStore::at(&activity_path);
    invoke_activity_hook(&lifecycle, &activity, hook_payload(temp.path(), "PreToolUse", "call-1", "cargo test", None));
    activity.append(decision_event(temp.path(), "activity-1", 2, None, "cargo test", ActivityState::Allowed)).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles = (0..2).map(|_| {
        let barrier = barrier.clone();
        let cwd = temp.path().to_path_buf();
        let lifecycle_path = lifecycle_path.clone();
        let activity_path = activity_path.clone();
        std::thread::spawn(move || {
            barrier.wait();
            invoke_activity_hook(
                &LifecycleStore::at(lifecycle_path),
                &ActivityStore::at(activity_path),
                hook_payload(&cwd, "PostToolUse", "call-1", "cargo test", Some(serde_json::json!("done"))),
            )
        })
    }).collect::<Vec<_>>();
    for handle in handles {
        assert!(handle.join().unwrap().is_empty());
    }
    assert_eq!(outcome_and_diagnostic_counts(&activity).0, 1);
    assert_eq!(activity.read().unwrap().events().iter().filter(|event| event.tool.as_deref() == Some("PostToolUse")).count(), 2);
}
```

For lock contention, exclusively lock `activity.jsonl`'s sibling `activity.lock` with `fs2::FileExt`, invoke PostToolUse, and assert bounded stderr, empty stdout, and no panic. For storage failure, create a directory at the `activity.jsonl` path and assert the same fail-open behavior. These cases do not promise durable observation.

- [ ] **Step 6: Run lifecycle-hook and activity-scale tests**

Run:

```bash
cargo test -p coding-brain lifecycle_hook::tests -- --nocapture
cargo test -p coding-brain --test activity_scale
```

Expected: all tests pass; ambiguity tests contain no Outcome; duplicate Post contains one Outcome; the large-log fixture selects the tail Decision.

- [ ] **Step 7: Review checkpoint**

Run `git diff --check` and inspect `git diff -- src/lifecycle_hook.rs`. Do not commit without explicit authorization. If authorized: `git add src/lifecycle_hook.rs && git commit -m "🔗 fix: correlate post-tool outcomes safely"`.

### Task 3: Doctor outcome-telemetry coverage (`codexctl-kji`)

**Files:**
- Modify: `src/doctor.rs:14-85,308-352,535-940`

**Interfaces:**
- Consumes: `ActivityStore::read()`, lifecycle observations from Task 2, and Decision/Outcome activity groups from Tasks 1–2.
- Produces: `check_outcome_telemetry_with_store(&ActivityStore) -> Check`, added to `run_all_checks()` after lifecycle state.

**Acceptance Criteria:**
- Fewer than 10 unique recent PreToolUse invocation keys is Skipped.
- Exactly 10 or more PreToolUse keys with zero PostToolUse in the latest 100 unique keys is Advisory with restart, `/hooks`, and completed-tool guidance.
- Once PostToolUse exists, fewer than 5 unique eligible allowed-and-delivered decisions is Skipped.
- Exactly 5 or more eligible decisions with zero Outcomes in the latest 20 is Advisory with attribution guidance.
- At least one Outcome in the bounded eligible window is Pass.
- Retry rows cannot inflate counts, older evidence outside each recent window cannot hide zero current coverage, and store read failures are non-fatal Advisory.
- Window membership is based on invocation recency and decision delivery time; delayed evidence cannot reorder either window.

- [ ] **Step 0: Establish the focused baseline**

Run `cargo test -p coding-brain doctor::tests -- --nocapture` before editing. Expected: all existing Doctor tests pass.

- [ ] **Step 1: Write failing threshold, deduplication, and expiry tests**

Add fixture helpers that append lifecycle rows keyed by `(session, turn, tool_use_id)` and grouped Decision rows by `activity_id`. Add tests equivalent to:

```rust
#[test]
fn outcome_telemetry_has_exact_minimum_boundaries() {
    let (_, store) = fixture_activity_store();
    for index in 0..9 {
        append_tool_invocation(&store, index, false);
    }
    assert_eq!(check_outcome_telemetry_with_store(&store).status, CheckStatus::Skipped);
    append_tool_invocation(&store, 9, false);
    assert_eq!(check_outcome_telemetry_with_store(&store).status, CheckStatus::Advisory);

    let (_, store) = fixture_activity_store();
    for index in 0..10 {
        append_tool_invocation(&store, index, true);
    }
    for index in 0..4 {
        append_delivered_decision(&store, index, false);
    }
    assert_eq!(check_outcome_telemetry_with_store(&store).status, CheckStatus::Skipped);
    append_delivered_decision(&store, 4, false);
    assert_eq!(check_outcome_telemetry_with_store(&store).status, CheckStatus::Advisory);
}
```

Add a retry test that repeats one invocation more than ten times but remains Skipped, and expiry tests where one old PostToolUse or Outcome falls outside the latest 100/20 bounded window and therefore does not produce Pass.

Use one complete event builder for all Doctor fixtures:

```rust
fn telemetry_event(
    activity_id: &str,
    kind: ActivityKind,
    state: ActivityState,
    recorded_at_ms: u64,
    tool: &str,
    tool_use_id: Option<&str>,
    outcome: Option<ActivityOutcome>,
) -> ActivityEvent {
    let project_id = ProjectId::Temporary("doctor-project".into());
    ActivityEvent {
        schema_version: ACTIVITY_SCHEMA_VERSION,
        kind,
        activity_id: activity_id.into(),
        recorded_at_ms,
        project: ProjectEvidence {
            project_id: project_id.clone(),
            cwd: PathBuf::from("/work/doctor-project"),
            label: Some("doctor-project".into()),
        },
        session: Some(SessionTarget {
            session_id: "doctor-session".into(),
            turn_id: Some("doctor-turn".into()),
            tool_use_id: tool_use_id.map(str::to_owned),
            project_id,
            cwd: PathBuf::from("/work/doctor-project"),
            provider_hints: Vec::new(),
        }),
        state,
        tool: Some(tool.into()),
        normalized_command: (kind == ActivityKind::Decision).then(|| "cargo test".into()),
        fingerprint: None,
        rule_id: None,
        confidence: None,
        threshold: None,
        reasoning: None,
        decision_id: (kind == ActivityKind::Decision).then(|| format!("decision-{activity_id}")),
        outcome,
        correction: None,
        note: None,
        supersedes: None,
    }
}

fn append_tool_invocation(store: &ActivityStore, index: usize, with_post: bool) {
    let call = format!("call-{index}");
    store.append(telemetry_event(
        &format!("pre-{index}"), ActivityKind::Lifecycle, ActivityState::Abstained,
        (index * 2) as u64, "PreToolUse", Some(&call), None,
    )).unwrap();
    if with_post {
        store.append(telemetry_event(
            &format!("post-{index}"), ActivityKind::Lifecycle, ActivityState::Abstained,
            (index * 2 + 1) as u64, "PostToolUse", Some(&call), None,
        )).unwrap();
    }
}

fn append_delivered_decision(store: &ActivityStore, index: usize, with_outcome: bool) {
    let id = format!("activity-{index}");
    store.append(telemetry_event(
        &id, ActivityKind::Decision, ActivityState::Allowed,
        (10_000 + index * 3) as u64, "Bash", None, None,
    )).unwrap();
    store.append(telemetry_event(
        &id, ActivityKind::Decision, ActivityState::Delivered,
        (10_001 + index * 3) as u64, "Bash", None, None,
    )).unwrap();
    if with_outcome {
        store.append(telemetry_event(
            &id, ActivityKind::Decision, ActivityState::Outcome,
            (10_002 + index * 3) as u64, "Bash", Some(&format!("call-{index}")),
            Some(ActivityOutcome::Completed),
        )).unwrap();
    }
}
```

```rust
fn fixture_activity_store() -> (tempfile::TempDir, ActivityStore) {
    let temp = tempfile::tempdir().unwrap();
    let store = ActivityStore::at(temp.path().join("activity.jsonl"));
    (temp, store)
}

#[test]
fn telemetry_retries_do_not_inflate_unique_invocations() {
    let (_, store) = fixture_activity_store();
    for index in 0..11 {
        store.append(telemetry_event(
            &format!("retry-{index}"), ActivityKind::Lifecycle, ActivityState::Abstained,
            index as u64, "PreToolUse", Some("same-call"), None,
        )).unwrap();
    }
    assert_eq!(check_outcome_telemetry_with_store(&store).status, CheckStatus::Skipped);
}

#[test]
fn old_post_evidence_expires_from_the_hundred_key_window() {
    let (_, store) = fixture_activity_store();
    append_tool_invocation(&store, 0, true);
    for index in 1..=100 {
        append_tool_invocation(&store, index, false);
    }
    let check = check_outcome_telemetry_with_store(&store);
    assert_eq!(check.status, CheckStatus::Advisory);
    assert!(check.message.contains("no PostToolUse evidence"));
}

#[test]
fn delayed_outcome_does_not_reorder_the_decision_window() {
    let (_, store) = fixture_activity_store();
    for index in 0..10 {
        append_tool_invocation(&store, index, true);
    }
    for index in 0..21 {
        append_delivered_decision(&store, index, false);
    }
    store.append(telemetry_event(
        "activity-0", ActivityKind::Decision, ActivityState::Outcome,
        99_999, "Bash", Some("call-0"), Some(ActivityOutcome::Completed),
    )).unwrap();
    let check = check_outcome_telemetry_with_store(&store);
    assert_eq!(check.status, CheckStatus::Advisory);
    assert!(check.message.contains("0/20"));
}

#[test]
fn reverse_post_rows_do_not_hide_selected_pre_rows() {
    let (_, store) = fixture_activity_store();
    for index in 0..100 {
        append_tool_invocation(&store, index, true);
    }
    let check = check_outcome_telemetry_with_store(&store);
    assert_eq!(check.status, CheckStatus::Skipped);
    assert!(!check.message.contains("insufficient activity"));
    assert!(!check.message.contains("no PostToolUse evidence"));
}
```

- [ ] **Step 2: Run Doctor tests and confirm failure**

Run `cargo test -p coding-brain doctor::tests::outcome_telemetry -- --nocapture`.

Expected: compilation fails because the new check and fixture helpers do not exist.

- [ ] **Step 3: Implement the bounded zero-coverage check**

Import `HashMap`/`HashSet`, `ActivityKind`, `ActivityState`, and `ActivityStore`. Construct the production store from `CodingBrainPaths::resolve` in `check_outcome_telemetry()`, delegating to a store-injected helper for tests.

Use two in-memory passes over the single `ActivityLog` read. The first reverse pass selects the latest 100 distinct owned `(session_id, turn_id, tool_use_id)` keys by their newest lifecycle event; keys missing any ID component are ineligible. The second reverse pass ignores older unselected keys and fills PreToolUse/PostToolUse flags for the selected keys, so reaching 100 Post rows cannot hide their older matching Pre rows.

Separately group Decision rows by `activity_id`, retain only groups containing Allowed plus Delivered, record the Delivered timestamp as recency, sort by that timestamp, truncate to 20, and only then inspect Outcome presence. A delayed Outcome must not move an old decision into the recent window. Return one of these exact shapes:

```rust
Check {
    name: "outcome telemetry".into(),
    status: CheckStatus::Skipped,
    message: format!("insufficient activity ({pre_count}/10 tool invocations)"),
    fix_hint: None,
}

Check {
    name: "outcome telemetry".into(),
    status: CheckStatus::Advisory,
    message: format!("no PostToolUse evidence across {pre_count} recent invocations"),
    fix_hint: Some("Upgrade or restart Codex, review `/hooks`, complete local tools, and rerun `coding-brain doctor`.".into()),
}

Check {
    name: "outcome telemetry".into(),
    status: CheckStatus::Advisory,
    message: format!("PostToolUse observed but 0/{eligible_count} recent decisions have outcomes"),
    fix_hint: Some("Run current Codex hooks and inspect lifecycle-hook attribution diagnostics.".into()),
}
```

Pass reports the compact observed counts. Read/path errors remain Advisory with state-directory ownership and permissions guidance. Do not change `exit_code`; Advisory remains zero.

- [ ] **Step 4: Run all Doctor tests**

Run `cargo test -p coding-brain doctor::tests -- --nocapture`.

Expected: all Doctor tests pass, including 9/10 and 4/5 boundaries, retry deduplication, expiry, read failure, and zero exit code for Advisory.

Also require regressions where 100 Post rows appear before their matching Pre rows in reverse traversal and where a delayed Outcome targets a decision older than the 20-decision delivery window.

- [ ] **Step 5: Review checkpoint**

Run `git diff --check` and inspect `git diff -- src/doctor.rs`. Do not commit without explicit authorization. If authorized: `git add src/doctor.rs && git commit -m "🩺 feat: diagnose missing outcome telemetry"`.

### Task 4: Live evidence wording (`codexctl-3xn`)

**Files:**
- Modify: `crates/coding-brain-tui/src/ui/brain/live.rs:156-201`
- Modify: `crates/coding-brain-tui/src/ui/brain/mod.rs:300-475`

**Interfaces:**
- Consumes: `ActivityOutcome::Completed` and existing `ActivityItem` projection.
- Produces: exact user-facing labels for Completed and delivery-without-outcome states.

**Acceptance Criteria:**
- Completed, succeeded, failed, and cancelled render distinctly.
- Delivered Allowed without Outcome renders `allowed · response delivered` and never claims execution success.
- Unknown and Failed delivery without Outcome retain `execution not confirmed`.
- Delivered Denied remains `blocked · command did not execute`.

- [ ] **Step 0: Establish the focused baseline**

Run `cargo test -p coding-brain-tui ui::brain -- --nocapture` before editing. Expected: all existing Brain UI tests pass.

- [ ] **Step 1: Write failing rendered-text tests**

Replace the single success-only test with explicit cases:

```rust
#[test]
fn live_status_distinguishes_outcomes_and_delivery_evidence() {
    for (outcome, label) in [
        (ActivityOutcome::Completed, "completed"),
        (ActivityOutcome::Succeeded, "succeeded"),
        (ActivityOutcome::Failed, "failed"),
        (ActivityOutcome::Cancelled, "cancelled"),
    ] {
        let mut item = activity(label, DeliveryState::Delivered);
        item.outcome = Some(outcome);
        item.tool_execution_confirmed = true;
        assert!(live::activity_status(&item).contains(&format!("outcome confirmed: {label}")));
    }

    let delivered = activity("delivered", DeliveryState::Delivered);
    assert_eq!(live::activity_status(&delivered), "allowed · response delivered");
    assert!(live::activity_status(&activity("unknown", DeliveryState::Unknown))
        .contains("execution not confirmed"));
    assert!(live::activity_status(&activity("failed", DeliveryState::Failed))
        .contains("execution not confirmed"));
}
```

Use the existing test helper's default state; set it to Allowed locally if needed.

- [ ] **Step 2: Run the TUI test and confirm failure**

Run `cargo test -p coding-brain-tui ui::brain::tests::live_status_distinguishes_outcomes_and_delivery_evidence`.

Expected: exhaustive matching fails for Completed and Delivered still includes `execution not confirmed`.

- [ ] **Step 3: Implement the two surgical wording changes**

Add `ActivityOutcome::Completed => "completed"` to the existing outcome match. Change only the Delivered arm:

```rust
DeliveryState::Delivered => format!("{} · response delivered", decision_state(item.state)),
```

Leave Failed, Unknown, NotApplicable, and the delivered-denial guard unchanged.

- [ ] **Step 4: Run TUI tests**

Run `cargo test -p coding-brain-tui ui::brain -- --nocapture`.

Expected: all Live, Review, and Scorecard UI tests pass.

- [ ] **Step 5: Review checkpoint**

Run `git diff --check` and inspect `git diff -- crates/coding-brain-tui/src/ui/brain/live.rs crates/coding-brain-tui/src/ui/brain/mod.rs`. Do not commit without explicit authorization. If authorized: `git add crates/coding-brain-tui/src/ui/brain/live.rs crates/coding-brain-tui/src/ui/brain/mod.rs && git commit -m "🖥️ fix: clarify live execution evidence"`.

### Task 5: Current-Codex end-to-end regression and release note (`codexctl-8c6`)

**Files:**
- Modify: `tests/hook_activity.rs:1-80,252-280,472-550`
- Modify: `CHANGELOG.md:5-25`

**Interfaces:**
- Consumes: the v2 store, PreToolUse-anchored correlation, Completed projection, Doctor semantics, and Live labels from Tasks 1–4.
- Produces: real-binary proof for the current Codex payload shape and an operator-facing upgrade/downgrade note.

**Acceptance Criteria:**
- The shared PermissionRequest fixture omits `tool_use_id`, matching current Codex.
- A PreToolUse with `tool_use_id`, that PermissionRequest, and a PostToolUse with the same ID plus string `tool_response` produce one original Decision with a neutral Completed confirmation.
- PostToolUse observation exists independently and no raw command or response is stored in its lifecycle/diagnostic rows.
- Existing delivery-failure and killed-after-stdout scenarios remain correct after fixture changes.
- Unreleased changelog explains corrected PostToolUse attribution, Doctor advisories, neutral Completed, schema v2, and unsupported downgrade after v2 writes.
- Formatting, tests, Clippy, and build all pass.

- [ ] **Step 0: Establish the focused integration baseline**

Run `cargo test -p coding-brain --test hook_activity -- --nocapture` before editing this task. Expected: the pre-existing integration suite passes after Tasks 1–4; record any unrelated failure before modifying fixtures.

- [ ] **Step 1: Write the failing real-binary regression**

Remove `tool_use_id` from `permission_payload`. Add helpers:

```rust
fn pre_tool_payload(cwd: &Path, command: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "session_id": "session-1",
        "turn_id": "turn-1",
        "tool_use_id": "call-1",
        "cwd": cwd,
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command}
    })).unwrap()
}

fn post_tool_payload(cwd: &Path, command: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "session_id": "session-1",
        "turn_id": "turn-1",
        "tool_use_id": "call-1",
        "cwd": cwd,
        "hook_event_name": "PostToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command},
        "tool_response": "Process exited with code 0"
    })).unwrap()
}
```

Add an integration test that runs PreToolUse, PermissionRequest, and PostToolUse in that order. Assert the terminal Decision's session has no tool ID, the projected item retains its original activity/decision identity, `outcome == Some(ActivityOutcome::Completed)`, and `tool_execution_confirmed` is true. Serialize lifecycle/diagnostic rows and assert neither the command nor response string appears.

Use this complete test body:

```rust
#[test]
fn current_codex_post_tool_use_confirms_idless_permission_decision() {
    let home = tempfile::tempdir().unwrap();
    install_model_fixture(home.path(), "approve");
    let command = "cargo test --workspace";

    let pre = run_lifecycle_hook(home.path(), &pre_tool_payload(home.path(), command));
    assert!(pre.status.success());
    assert!(pre.stderr.is_empty());
    let permission = run_permission_hook(home.path(), &permission_payload(home.path(), command));
    assert!(permission.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&permission.stdout).unwrap()
            ["hookSpecificOutput"]["decision"]["behavior"],
        "allow"
    );
    let before = activity(home.path()).read().unwrap().events().to_vec();
    let decision = before.iter()
        .find(|event| event.state == ActivityState::Allowed)
        .unwrap();
    let activity_id = decision.activity_id.clone();
    let decision_id = decision.decision_id.clone();
    assert_eq!(decision.session.as_ref().unwrap().tool_use_id, None);

    let post = run_lifecycle_hook(home.path(), &post_tool_payload(home.path(), command));
    assert!(post.status.success());
    assert!(post.stderr.is_empty(), "{}", String::from_utf8_lossy(&post.stderr));
    let store = activity(home.path());
    let events = store.read().unwrap().events().to_vec();
    let outcome = events.iter()
        .find(|event| event.activity_id == activity_id && event.state == ActivityState::Outcome)
        .unwrap();
    assert_eq!(outcome.decision_id, decision_id);
    assert_eq!(outcome.outcome, Some(ActivityOutcome::Completed));
    let projected = store.snapshot(SnapshotLimits::default()).unwrap().recent
        .into_iter().find(|item| item.activity_id == activity_id).unwrap();
    assert_eq!(projected.outcome, Some(ActivityOutcome::Completed));
    assert!(projected.tool_execution_confirmed);

    let persisted = std::fs::read_to_string(
        home.path().join(".local/state/coding-brain/activity.jsonl"),
    ).unwrap();
    for event in events.iter().filter(|event| event.kind != ActivityKind::Decision) {
        assert!(event.normalized_command.is_none());
        assert!(event.fingerprint.is_none());
        assert!(event.note.is_none());
    }
    assert!(!persisted.contains("Process exited with code 0"));
}
```

Import `ActivityKind` and `ActivityOutcome` alongside the existing activity types. The command is expected in the pre-existing Decision rows, so the privacy assertion deliberately inspects non-Decision rows rather than claiming the entire activity log omits it.

- [ ] **Step 2: Run the end-to-end test and confirm failure**

Run `cargo test -p coding-brain --test hook_activity post_tool_use -- --nocapture`.

Expected: the current exact-ID-only implementation cannot attach the Outcome to the ID-less PermissionRequest Decision.

- [ ] **Step 3: Update affected integration fixtures minimally**

Where `killed_after_stdout_is_unknown_until_later_outcome` expects later confirmation, run `pre_tool_payload` before PermissionRequest and replace its hand-built PostToolUse body with `post_tool_payload` or an explicit structured response when the test specifically needs Succeeded. Do not add tool IDs back to PermissionRequest.

- [ ] **Step 4: Add the Unreleased changelog entry**

Under `## [Unreleased]` / `### Changed`, add:

```markdown
- PostToolUse telemetry now records hook receipt independently and safely
  correlates current Codex Bash executions even though PermissionRequest omits
  `tool_use_id`. Opaque unified-exec responses are shown as neutral completed
  outcomes, and `coding-brain doctor` advises when runtime or attribution
  coverage remains zero. Activity rows now use schema v2 while retaining v1
  reads; downgrading after v2 rows are written is unsupported, so back up
  `~/.local/state/coding-brain/activity.jsonl` before upgrading if rollback is
  required.
```

Use `beads-superpowers:write-documentation` for this changelog edit and confirm the wording does not imply that Completed means success.

- [ ] **Step 5: Run focused integration suites**

Run:

```bash
cargo test -p coding-brain --test hook_activity -- --nocapture
cargo test -p coding-brain --test lifecycle_hook_cli -- --nocapture
cargo test -p coding-brain --test activity_scale -- --nocapture
```

Expected: all focused integration suites pass.

- [ ] **Step 6: Run complete quality gates**

Run in this order:

```bash
cargo fmt
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build
```

Expected: every command exits 0 with no formatter diff, test failure, Clippy warning, or build error.

- [ ] **Step 7: Final scope and privacy audit**

Run:

```bash
git diff --check
git status --short
git diff --stat
rg -n 'tool_response|normalized_command|fingerprint' src/lifecycle_hook.rs
```

Inspect every changed line against the approved spec. Confirm lifecycle and Diagnostic construction leaves `normalized_command`, `fingerprint`, and `note` unset, and no raw response is copied into persisted fields.

- [ ] **Step 8: Request code review and verify completion evidence**

Invoke `beads-superpowers:requesting-code-review` against the complete diff, address any actionable findings through `beads-superpowers:receiving-code-review`, then invoke `beads-superpowers:verification-before-completion` and rerun any gate it requires. Do not claim the issue fixed from earlier command output.

- [ ] **Step 9: Review checkpoint**

Do not commit without explicit authorization. If authorized after all gates pass: `git add crates/coding-brain-core/src/brain_activity.rs src/brain/activity.rs src/brain/permission_hook.rs src/lifecycle_hook.rs src/doctor.rs crates/coding-brain-tui/src/ui/brain/live.rs crates/coding-brain-tui/src/ui/brain/mod.rs tests/hook_activity.rs CHANGELOG.md && git commit -m "🔗 fix: restore PostToolUse outcome telemetry"`.

## Stress Test Results: PostToolUse Implementation Plan

### Resolved Decisions

- Concurrent idempotency: correlate and append from one exclusive-lock snapshot so racing PostToolUse processes store two observations but one Outcome.
- Task graph: reuse the existing epic and five task IDs; execution skills must not create duplicates.
- TDD executability: provide complete fixture interfaces and test bodies for correlation, Doctor, and real-binary coverage.
- Schema compatibility: all new Outcomes are v2 even when attached to v1 Decisions; explicitly test mixed grouped lifecycles, v1 append rejection, and v3 diagnostics.
- Doctor windows: preserve window membership while collecting older matching evidence and rank decisions by delivery time.
- Scale and failure: keep compaction outside the atomic lock operation, test contention/storage errors, and make persistence guarantees conditional on a writable store.
- Verification and release: establish focused baselines, run actual formatting, require documentation/review/verification workflows, and name the backup path.
- Security and privacy: content fallback requires lossless normalization, identifiers are bounded before comparison, and collision/secret persistence cases are adversarially tested.
- Decision eligibility: exact and fallback paths accept only first-terminal Allowed lifecycles, without requiring Delivered evidence.

### Changes Made

- Added an atomic `ActivityStore::append_from_snapshot` interface and concurrent regression.
- Replaced redaction-marker-only eligibility with redaction-and-truncation losslessness.
- Added concrete fixture builders and complete representative failing tests.
- Corrected mixed-schema Outcome emission and compatibility coverage.
- Stabilized Doctor's lifecycle and decision windows against event-order artifacts.
- Qualified observation durability under store failures and added contention coverage.
- Strengthened baseline, formatting, documentation, review, verification, backup, and privacy gates.
- Added negative eligibility coverage for Denied, Abstained, and Error decisions.

### Deferred / Parking Lot

- General non-Bash correlation without stable permission-side IDs remains outside this issue.
- Downgrade after schema v2 writes remains unsupported; rollback requires a pre-upgrade backup.
- No activity index or wall-clock performance contract is introduced.

### Confidence Assessment

- Overall: High; the required one-pass reflexion review added and resolved the Decision-eligibility guard.
- Areas of concern: the atomic store helper must preserve existing crash-tail repair and bounded lock behavior without duplicating append logic.
