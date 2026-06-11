# Codex-Only Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the active runtime path discover, parse, and display Codex sessions instead of Codex sessions.

**Architecture:** Keep the existing crates and broad session struct for phase 1, but switch the data source to Codex JSONL transcripts. Add a Codex transcript parser and transcript-backed session discovery, then guard process enrichment so transcript-only sessions are not incorrectly marked finished. Replace the active Codex adapter stub and Codex hook install path after the core session path works.

**Tech Stack:** Rust 2024, serde_json, clap, ratatui, Cargo integration/unit tests.

---

## File Structure

- Modify `crates/codexctl-core/src/session.rs`: add a `process_backed` flag and a constructor for Codex transcript sessions while leaving the existing struct name in place.
- Modify `crates/codexctl-core/src/discovery.rs`: replace Codex pointer-file scanning with Codex JSONL scanning; add `CODEXCTL_CODEX_HOME` test override.
- Create `crates/codexctl-core/src/codex_transcript.rs`: parse Codex JSONL events into normalized metadata and transcript events.
- Modify `crates/codexctl-core/src/transcript.rs`: keep Codex parsing for fixture compatibility only; do not extend it with Codex logic.
- Modify `crates/codexctl-core/src/monitor.rs`: use Codex parser when a transcript is marked as Codex; preserve existing Codex parser tests during transition.
- Modify `crates/codexctl-core/src/process.rs`: skip live `ps` termination for transcript-backed sessions.
- Modify `crates/codexctl-core/src/lib.rs`: export the new Codex parser module.
- Modify `crates/codexctl-tui/src/app.rs`: key preserved session state by session id instead of PID for Codex transcript sessions.
- Modify `src/coord/adapter.rs` and `src/coord/adapter_codex.rs`: remove Codex adapter registration and make Codex discovery real.
- Modify `src/init/hooks.rs`: write Codex hook config under `~/.codex/hooks.json` or `.codex/hooks.json`.
- Modify `src/main.rs`, `src/commands.rs`, `Cargo.toml`, and `flake.nix`: phase-1 public naming from `codexctl` to `codexctl` for the binary/help and hook commands.
- Add tests in `tests/integration_tests.rs` and fixtures under `tests/fixtures/`.

## Task 1: Codex Transcript Parser

**Files:**
- Create: `crates/codexctl-core/src/codex_transcript.rs`
- Modify: `crates/codexctl-core/src/lib.rs`
- Test: `tests/fixtures/codex-session-meta.json`
- Test: `tests/fixtures/codex-tool-call.json`

- [ ] **Step 1: Write failing parser tests**

Add unit tests in `codex_transcript.rs` that parse:

```rust
#[test]
fn parses_session_meta() {
    let line = include_str!("../../../tests/fixtures/codex-session-meta.json");
    let Some(CodexEvent::SessionMeta(meta)) = parse_line(line.trim()) else {
        panic!("expected session meta");
    };
    assert_eq!(meta.session_id, "019eb6ac-6d30-7301-885d-ff4d354c0116");
    assert_eq!(meta.cwd, "/home/alexander/hacking/aleadag/codexctl");
    assert_eq!(meta.model_provider.as_deref(), Some("openai"));
}

#[test]
fn parses_function_call() {
    let line = include_str!("../../../tests/fixtures/codex-tool-call.json");
    let Some(CodexEvent::ResponseItem(item)) = parse_line(line.trim()) else {
        panic!("expected response item");
    };
    assert_eq!(item.kind, CodexResponseKind::FunctionCall);
    assert_eq!(item.name.as_deref(), Some("exec_command"));
    assert!(item.arguments.as_deref().unwrap().contains("cargo test"));
}
```

- [ ] **Step 2: Run parser tests and verify RED**

Run: `cargo test -p codexctl-core codex_transcript -- --nocapture`

Expected: FAIL because `codex_transcript` does not exist.

- [ ] **Step 3: Implement minimal parser**

Create `CodexEvent`, `CodexSessionMeta`, `CodexResponseItem`, and `parse_line`.
Parse only the fields the tests assert plus optional `turn_context` status data.

- [ ] **Step 4: Run parser tests and verify GREEN**

Run: `cargo test -p codexctl-core codex_transcript -- --nocapture`

Expected: parser tests PASS.

## Task 2: Codex Session Discovery

**Files:**
- Modify: `crates/codexctl-core/src/session.rs`
- Modify: `crates/codexctl-core/src/discovery.rs`
- Test: `tests/integration_tests.rs`

- [ ] **Step 1: Write failing discovery test**

Add a test that creates:

```text
<temp>/.codex/sessions/2026/06/11/rollout-2026-06-11T20-33-34-019eb6ac-6d30-7301-885d-ff4d354c0116.jsonl
```

with a `session_meta` line, sets `CODEXCTL_CODEX_HOME=<temp>/.codex`, calls
`discovery::scan_sessions()`, and asserts:

```rust
assert_eq!(sessions.len(), 1);
assert_eq!(sessions[0].session_id, "019eb6ac-6d30-7301-885d-ff4d354c0116");
assert_eq!(sessions[0].cwd, "/home/alexander/hacking/aleadag/codexctl");
assert_eq!(sessions[0].jsonl_path.as_ref().unwrap(), &jsonl_path);
assert!(!sessions[0].process_backed);
```

- [ ] **Step 2: Run discovery test and verify RED**

Run: `cargo test codex_discovery_scans_rollout_jsonl -- --nocapture`

Expected: FAIL because discovery still scans `~/.codex/sessions`.

- [ ] **Step 3: Implement Codex discovery**

Add `codex_home()`, `sessions_dir()`, recursive JSONL scanning, and
`CodexSession::from_codex_meta(...)`. Use a stable synthetic PID from the
session id hash because the current UI requires a numeric key.

- [ ] **Step 4: Run discovery test and verify GREEN**

Run: `cargo test codex_discovery_scans_rollout_jsonl -- --nocapture`

Expected: PASS.

## Task 3: Monitor Codex JSONL Events

**Files:**
- Modify: `crates/codexctl-core/src/monitor.rs`
- Test: `tests/integration_tests.rs`

- [ ] **Step 1: Write failing monitor test**

Add a test that builds a transcript-backed session with a Codex function call
event and a function call output event, then calls `monitor::update_tokens`.
Assert:

```rust
assert_eq!(session.telemetry_status, TelemetryStatus::Available);
assert_eq!(session.tool_usage.get("exec_command").unwrap().calls, 1);
assert_eq!(session.pending_tool_name, None);
assert!(!session.last_tool_error);
```

- [ ] **Step 2: Run monitor test and verify RED**

Run: `cargo test codex_monitor_records_function_calls -- --nocapture`

Expected: FAIL because monitor treats Codex JSONL as unsupported.

- [ ] **Step 3: Implement Codex event handling in monitor**

When `session.process_backed == false`, parse lines with `codex_transcript`.
Record response item function calls as tool usage. Treat function call outputs
with non-empty output as completed tool calls. Keep unavailable token/cost
metrics as unavailable.

- [ ] **Step 4: Run monitor test and verify GREEN**

Run: `cargo test codex_monitor_records_function_calls -- --nocapture`

Expected: PASS.

## Task 4: TUI and JSON Source Stability

**Files:**
- Modify: `crates/codexctl-core/src/process.rs`
- Modify: `crates/codexctl-tui/src/app.rs`
- Modify: `src/runtime/sessions.rs`
- Test: `tests/integration_tests.rs`

- [ ] **Step 1: Write failing process test**

Add a test that creates a transcript-backed session, calls
`process::fetch_and_enrich`, and asserts the status is not forced to
`Finished`.

- [ ] **Step 2: Run process test and verify RED**

Run: `cargo test transcript_backed_sessions_are_not_marked_finished_by_ps -- --nocapture`

Expected: FAIL because `fetch_and_enrich` marks missing PIDs as finished.

- [ ] **Step 3: Implement process-backed guard**

Skip `ps` lookup for sessions where `process_backed == false`. In app refresh,
preserve existing state by `session_id` for transcript-backed sessions and by
PID for process-backed sessions.

- [ ] **Step 4: Run process test and relevant integration tests**

Run: `cargo test transcript_backed_sessions_are_not_marked_finished_by_ps -- --nocapture`

Expected: PASS.

## Task 5: Codex Adapter and Hook Install

**Files:**
- Modify: `src/coord/adapter.rs`
- Delete or stop using the legacy coord adapter file
- Modify: `src/coord/adapter_codex.rs`
- Modify: `src/init/hooks.rs`
- Test: `tests/unit_tests.rs`

- [ ] **Step 1: Write failing adapter and hook tests**

Adapter test:

```rust
let adapters = adapter::all_adapters();
assert_eq!(adapters.len(), 1);
assert_eq!(adapters[0].family(), AgentFamily::Codex);
assert!(adapter::get_adapter("codex").is_none());
assert!(adapter::get_adapter("codex").is_some());
```

Hook test:

```rust
let hooks = build_hooks_value();
assert!(hooks["hooks"]["PermissionRequest"].is_array());
assert!(hooks.to_string().contains("codexctl"));
assert!(!hooks.to_string().contains("codexctl"));
```

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test adapter hook -- --nocapture`

Expected: FAIL because Codex adapter is still registered and hooks use Codex
paths/commands.

- [ ] **Step 3: Implement Codex-only adapter and hooks**

Remove Codex adapter registration. Make `CodexAdapter::discover_sessions`
call `discovery::scan_sessions()`. Change init hook settings paths to
`~/.codex/hooks.json` and `.codex/hooks.json`, and command strings to
`codexctl`.

- [ ] **Step 4: Run tests and verify GREEN**

Run: `cargo test adapter hook -- --nocapture`

Expected: PASS.

## Task 6: Public Binary Name and Focused Verification

**Files:**
- Modify: `Cargo.toml`
- Modify: `flake.nix`
- Modify: `src/main.rs`
- Modify: `src/commands.rs`

- [ ] **Step 1: Write failing CLI metadata test or run metadata check**

Run: `cargo run -- --help`

Expected before implementation: help displays `codexctl`.

- [ ] **Step 2: Rename binary surface to codexctl**

Change package and binary `name` fields to `codexctl`; leave internal crate
names alone. Update obvious user-facing help text in `src/main.rs` and
`src/commands.rs` that mentions Codex hook setup.

- [ ] **Step 3: Run focused command checks**

Run:

```bash
cargo run -- --help
cargo run -- --json
```

Expected: help displays `codexctl`; JSON command exits successfully and lists
Codex sessions when local transcripts are available.

- [ ] **Step 4: Full verification**

Run:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```

Expected: all commands exit 0, or failures are investigated with
`superpowers:systematic-debugging` before any completion claim.
