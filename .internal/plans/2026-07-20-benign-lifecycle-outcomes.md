# Benign Lifecycle Outcomes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use beads-superpowers:subagent-driven-development (recommended) or beads-superpowers:executing-plans to implement this plan task-by-task. Each Task becomes a bead (`bd create -t task --parent <epic-id>`). Steps within tasks use checkbox (`- [ ]`) syntax for human readability.

**Goal:** Stop ordinary tool completions without Brain decisions from appearing as orphan errors while preserving diagnostics for incomplete Decision activity.

**Architecture:** Inspect exact-identity activity before attributing a `PostToolUse` outcome. Treat the absence of Decision activity as a benign no-op, retain the current exact terminal-decision join, and diagnose only when Decision activity exists but cannot supply an attributable terminal decision.

**Tech Stack:** Rust, JSONL activity persistence, Cargo tests, Jujutsu, Beads

## Global Constraints

- Do not change permission evaluation or authorization behavior.
- Do not match outcomes by tool name, timing, or approximate session evidence.
- Do not change activity schema version 1 or persistent paths.
- Keep the patch within `src/lifecycle_hook.rs` and its focused tests unless verification exposes a direct dependency.
- Follow red-green-refactor and do not push.

---

### Task 1: Distinguish Benign and Broken Outcome Joins

**Files:**
- Modify: `src/lifecycle_hook.rs`

**Interfaces:**
- Consumes: `LifecycleEvent::identity`, `ActivityLog::events`, `ActivityKind::Decision`
- Produces: unchanged `append_outcome(...) -> Result<(), String>` behavior with a benign no-decision branch

**Acceptance Criteria:**
- `PostToolUse` with no Decision activity for its exact identity appends neither an outcome nor a diagnostic.
- `PostToolUse` with an attributable terminal Decision appends the existing outcome.
- `PostToolUse` with matching but incomplete Decision activity appends the existing orphan diagnostic.
- Focused lifecycle-hook tests and repository quality gates pass.

- [ ] **Step 1: Replace the empty-store orphan regression with a failing benign-path test**

Rename `unmatched_post_tool_use_appends_orphan_diagnostic_without_guessing` to
`post_tool_use_without_decision_activity_is_ignored` and assert empty stderr and
an activity log containing only the lifecycle observation.

- [ ] **Step 2: Run the focused test and confirm RED**

Run:

```bash
cargo test --lib lifecycle_hook::tests::post_tool_use_without_decision_activity_is_ignored -- --exact
```

Expected: failure because `append_outcome` appends a diagnostic and writes
`orphan outcome` to stderr.

- [ ] **Step 3: Add an incomplete-decision characterization regression**

Persist `Observed` Decision activity with the same session, turn, and tool-use
ID, run `PostToolUse`, and assert that one Diagnostic error is appended with the
existing orphan reason. Run it before production edits and confirm that this
behavior already passes; it is the safety boundary the fix must preserve.

- [ ] **Step 4: Implement the minimal exact-identity classification**

In `append_outcome`, first collect whether an exact-identity Decision activity
exists. Keep the current reverse terminal-decision lookup. If it does not match
and no exact Decision activity exists, return `Ok(())`; otherwise append the
orphan diagnostic and return its error as today.

- [ ] **Step 5: Run focused tests and confirm GREEN**

Run:

```bash
cargo test --lib lifecycle_hook::tests
```

Expected: all lifecycle-hook unit tests pass.

- [ ] **Step 6: Run repository verification**

Run:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
```

Expected: all commands exit zero with no warnings.

- [ ] **Step 7: Inspect the final changeset**

Run:

```bash
jj --no-pager diff --git
jj --no-pager st
```

Expected: only the focused lifecycle outcome semantics, regressions, and the
approved design/plan artifacts are changed; no push occurs.
