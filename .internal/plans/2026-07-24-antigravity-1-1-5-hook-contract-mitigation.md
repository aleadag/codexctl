# Antigravity 1.1.5 Hook Contract Mitigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use beads-superpowers:subagent-driven-development (recommended) or beads-superpowers:executing-plans to implement this plan task-by-task. Each Task becomes a bead (`bd create -t task --parent <epic-id>`). Steps within tasks use checkbox (`- [ ]`) syntax for human readability.

**Goal:** Diagnose the confirmed Antigravity CLI 1.1.5 hook-contract failure and describe hook responses without claiming provider acceptance or execution.

**Architecture:** Keep provider setup integrity and provider compatibility as separate Doctor checks. Reuse the binary crate's existing bounded process runner for one conditional `agy --version` probe, classify only exact version `1.1.5`, and correct Live copy so response emission remains distinct from later outcome evidence.

**Tech Stack:** Rust 2024 workspace, standard-library process management, `libc`, Ratatui TUI tests, Markdown documentation, Beads task tracking.

## Global Constraints

- Preserve the existing Antigravity hook response JSON and provider-policy behavior.
- Do not send terminal keys, enable always-proceed, retry permission decisions, or weaken native permissions.
- Classify only exact `agy` version `1.1.5` as confirmed affected; all other versions remain unverified.
- Probe at most once per Doctor invocation, only when the Antigravity executable is available and managed hooks are current.
- Bound the version probe to 500 milliseconds and 128 stdout bytes; failed, oversized, non-UTF-8, or malformed output produces no compatibility claim.
- Render a successful allow or deny hook write as `response emitted`; only `ActivityOutcome` may claim execution.
- Keep the change surgical and do not commit, push, publish, or file an upstream issue without separate user authorization.

---

## File Map

- Modify `src/provider_hooks/mod.rs`: make the existing bounded process helper available to sibling binary modules on every Unix target and add output-cap regression coverage.
- Modify `src/doctor.rs`: conditionally probe `agy --version`, strictly parse the version, and add the exact 1.1.5 compatibility advisory.
- Modify `crates/coding-brain-tui/src/ui/brain/live.rs`: remove the deny-only execution inference and rename successful delivery copy to response emission.
- Modify `crates/coding-brain-tui/src/ui/brain/mod.rs`: lock the allow and deny evidence wording with rendering tests.
- Modify `docs/troubleshooting.md`: document the confirmed 1.1.5 limitation, safe operator behavior, and future-release revalidation procedure.

### Task 1: Add the bounded Antigravity 1.1.5 Doctor advisory

**Files:**
- Modify: `src/provider_hooks/mod.rs:458-556`
- Modify: `src/provider_hooks/mod.rs:700-725`
- Modify: `src/doctor.rs:68-87`
- Modify: `src/doctor.rs:180-290`
- Test: `src/doctor.rs` unit-test module

**Interfaces:**
- Consumes: `ProviderHookInspection::Current`, `AgentProvider::Antigravity`, `crate::init::state::detect_provider_executables()`, and `crate::provider_hooks::run_bounded_process(&mut Command, Duration, usize) -> Option<Vec<u8>>`.
- Produces: `check_antigravity_hook_contract() -> Option<Check>`, `check_antigravity_hook_contract_with(ProviderSetupEvidence, impl FnOnce() -> Option<[u64; 3]>) -> Option<Check>`, and `parse_antigravity_version(&[u8]) -> Option<[u64; 3]>`.

**Acceptance Criteria:**
- Doctor adds a non-fatal `Antigravity hook contract` advisory only for current managed Antigravity hooks, an available executable, and exact parsed version `1.1.5`.
- Doctor does not run the version probe when hooks are missing, stale, duplicate, invalid, or the executable is absent.
- Versions other than `1.1.5` and failed, oversized, non-UTF-8, or malformed probe output produce no compatibility row and do not change provider setup results.
- The production probe runs at most once, has a 500 millisecond timeout, captures at most 128 stdout bytes, and suppresses stderr.

- [ ] **Step 1: Add failing pure-classifier and strict-parser tests**

Add these tests to the existing `src/doctor.rs` test module:

```rust
#[test]
fn antigravity_1_1_5_with_current_hooks_is_advisory() {
    let check = check_antigravity_hook_contract_with(
        ProviderSetupEvidence {
            recorded: true,
            executable_available: true,
            hooks: ProviderHookInspection::Current,
        },
        || Some([1, 1, 5]),
    )
    .expect("affected version must be visible");

    assert_eq!(check.name, "Antigravity hook contract");
    assert_eq!(check.status, CheckStatus::Advisory);
    assert!(check.message.contains("agy 1.1.5"));
    assert!(check.message.contains("native prompt"));
    assert!(
        check
            .fix_hint
            .as_deref()
            .is_some_and(|hint| hint.contains("upgrade"))
    );
    assert_eq!(exit_code(&[check]), 0);
}

#[test]
fn antigravity_compatibility_probe_is_gated_by_current_setup() {
    use std::cell::Cell;

    for evidence in [
        ProviderSetupEvidence {
            recorded: true,
            executable_available: false,
            hooks: ProviderHookInspection::Current,
        },
        ProviderSetupEvidence {
            recorded: true,
            executable_available: true,
            hooks: ProviderHookInspection::Missing,
        },
        ProviderSetupEvidence {
            recorded: true,
            executable_available: true,
            hooks: ProviderHookInspection::Stale,
        },
    ] {
        let calls = Cell::new(0);
        let check = check_antigravity_hook_contract_with(evidence, || {
            calls.set(calls.get() + 1);
            Some([1, 1, 5])
        });
        assert!(check.is_none());
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn antigravity_unverified_versions_have_no_compatibility_claim() {
    for version in [None, Some([1, 1, 4]), Some([1, 1, 6]), Some([2, 0, 0])] {
        let check = check_antigravity_hook_contract_with(
            ProviderSetupEvidence {
                recorded: true,
                executable_available: true,
                hooks: ProviderHookInspection::Current,
            },
            || version,
        );
        assert!(check.is_none(), "{version:?}");
    }
}

#[test]
fn antigravity_version_parser_accepts_only_one_simple_semver_token() {
    assert_eq!(parse_antigravity_version(b"1.1.5\n"), Some([1, 1, 5]));
    assert_eq!(parse_antigravity_version(b"1.1.5\r\n"), Some([1, 1, 5]));

    for malformed in [
        b"agy 1.1.5".as_slice(),
        b"1.1".as_slice(),
        b"1.1.5-beta".as_slice(),
        b"1.1.5 extra".as_slice(),
        b" 1.1.5".as_slice(),
        b"1.1.5 ".as_slice(),
        b"01.1.5".as_slice(),
        b"\xff\xfe".as_slice(),
    ] {
        assert_eq!(parse_antigravity_version(malformed), None, "{malformed:?}");
    }
}
```

- [ ] **Step 2: Run the Doctor tests and confirm the red state**

Run:

```bash
direnv exec . cargo test --bin coding-brain doctor::tests::antigravity_
```

Expected: compilation fails because `check_antigravity_hook_contract_with` and `parse_antigravity_version` do not exist.

- [ ] **Step 3: Make the existing bounded runner reusable inside the binary crate**

Change the two Unix helper gates and visibility in `src/provider_hooks/mod.rs`:

```rust
#[cfg(unix)]
pub(crate) fn run_bounded_process(
    command: &mut std::process::Command,
    timeout: std::time::Duration,
    output_limit: usize,
) -> Option<Vec<u8>> {
    // Keep the existing implementation unchanged.
}

#[cfg(unix)]
fn terminate_process_group(child: &mut std::process::Child) {
    // Keep the existing implementation unchanged.
}
```

Keep `read_parent_process` gated to non-Linux Unix targets. This compiles the already-tested generic runner on Linux without exposing the terminal crate's internal capture API.

- [ ] **Step 4: Add a failing output-cap regression for the shared runner**

Add to the existing `src/provider_hooks/mod.rs` test module:

```rust
#[cfg(unix)]
#[test]
fn bounded_process_group_collection_rejects_oversized_output() {
    use std::process::Command;
    use std::time::Duration;

    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf 12345"]);
    assert_eq!(
        run_bounded_process(&mut command, Duration::from_millis(100), 4),
        None
    );
}
```

- [ ] **Step 5: Run the bounded-runner regressions**

Run:

```bash
direnv exec . cargo test --bin coding-brain provider_hooks::tests::bounded_process_group_collection_
```

Expected: both timeout cleanup and oversized-output tests pass.

- [ ] **Step 6: Implement the strict parser, gated classifier, and production probe**

Add near the provider setup checks in `src/doctor.rs`:

```rust
const ANTIGRAVITY_VERSION_TIMEOUT: std::time::Duration =
    std::time::Duration::from_millis(500);
const ANTIGRAVITY_VERSION_OUTPUT_LIMIT: usize = 128;

fn parse_antigravity_version(output: &[u8]) -> Option<[u64; 3]> {
    let text = std::str::from_utf8(output).ok()?;
    let token = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    if token.is_empty() || token.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return None;
    }
    let parts = token.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| {
                part.is_empty()
                    || !part.bytes().all(|byte| byte.is_ascii_digit())
                    || (part.len() > 1 && part.starts_with('0'))
            })
    {
        return None;
    }
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ])
}

fn check_antigravity_hook_contract_with(
    evidence: ProviderSetupEvidence,
    probe: impl FnOnce() -> Option<[u64; 3]>,
) -> Option<Check> {
    if !evidence.executable_available
        || evidence.hooks != ProviderHookInspection::Current
    {
        return None;
    }
    (probe()? == [1, 1, 5]).then(|| Check {
        name: "Antigravity hook contract".into(),
        status: CheckStatus::Advisory,
        message:
            "agy 1.1.5 may ignore PreToolUse decisions and retain the native prompt".into(),
        fix_hint: Some(
            "Keep the native prompt authoritative; upgrade agy, then revalidate the real hook contract."
                .into(),
        ),
    })
}

#[cfg(unix)]
fn probe_antigravity_version() -> Option<[u64; 3]> {
    let mut command = std::process::Command::new("agy");
    command.arg("--version");
    let output = crate::provider_hooks::run_bounded_process(
        &mut command,
        ANTIGRAVITY_VERSION_TIMEOUT,
        ANTIGRAVITY_VERSION_OUTPUT_LIMIT,
    )?;
    parse_antigravity_version(&output)
}

#[cfg(not(unix))]
fn probe_antigravity_version() -> Option<[u64; 3]> {
    None
}

fn check_antigravity_hook_contract() -> Option<Check> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let cwd = std::env::current_dir().ok()?;
    let executable_available = crate::init::state::detect_provider_executables()
        .contains(&AgentProvider::Antigravity);
    let hooks = crate::init::provider_hooks::inspect_provider_hooks_at(
        AgentProvider::Antigravity,
        &home,
        &cwd,
    );
    check_antigravity_hook_contract_with(
        ProviderSetupEvidence {
            recorded: false,
            executable_available,
            hooks,
        },
        probe_antigravity_version,
    )
}
```

The `recorded` field is intentionally irrelevant to this compatibility check: current managed hooks plus the executable are the complete gate.

- [ ] **Step 7: Insert the compatibility row after provider setup**

Update `run_all_checks()` in `src/doctor.rs`:

```rust
checks.extend(check_provider_setups());
checks.extend(check_antigravity_hook_contract());
checks.extend([
    check_codex_hook_trust(),
```

`Option<Check>` is an iterator of zero or one row, so the production path probes exactly once at most.

- [ ] **Step 8: Run focused Doctor and bounded-process tests**

Run:

```bash
direnv exec . cargo test --bin coding-brain doctor::tests::antigravity_
direnv exec . cargo test --bin coding-brain provider_hooks::tests::bounded_process_group_collection_
```

Expected: all selected tests pass.

- [ ] **Step 9: Inspect the task diff without committing**

Run:

```bash
git diff -- src/doctor.rs src/provider_hooks/mod.rs
```

Expected: only the bounded-helper visibility/gating, one output-cap test, the conditional compatibility check, and its unit tests are present.

### Task 2: Make Live wording match the evidence boundary

**Files:**
- Modify: `crates/coding-brain-tui/src/ui/brain/live.rs:210-255`
- Modify: `crates/coding-brain-tui/src/ui/brain/mod.rs:390-415`
- Modify: `crates/coding-brain-tui/src/ui/brain/mod.rs:540-570`

**Interfaces:**
- Consumes: `ActivityItem.state`, `ActivityItem.delivery`, and `ActivityItem.outcome`.
- Produces: `activity_status(&ActivityItem) -> String` returning `allowed · response emitted` or `denied · response emitted` for successful response writes without outcome evidence.

**Acceptance Criteria:**
- Delivered allow and deny decisions render `response emitted`.
- A delivered deny never claims `blocked` or `command did not execute` without outcome evidence.
- Existing confirmed outcomes still render `outcome confirmed: <outcome>`.
- Failed and unknown delivery still state that execution is not confirmed.

- [ ] **Step 1: Change the rendering tests to the required evidence wording**

Replace `delivered_deny_is_recent_and_reports_blocked_execution` with:

```rust
#[test]
fn delivered_deny_is_recent_and_reports_response_emission() {
    let mock = MockBrainRuntime {
        activity_snapshot: ActivitySnapshot {
            recent: vec![activity("deny-1", DeliveryState::Delivered)],
            ..ActivitySnapshot::default()
        },
        endpoint_health: online(),
        ..MockBrainRuntime::default()
    };

    let text = render_text(&fixture_app(mock));

    assert!(text.contains("denied · response emitted"));
    assert!(!text.contains("blocked"));
    assert!(!text.contains("command did not execute"));
    assert!(text.contains("No unresolved decisions"));
}
```

Update the delivered section of `live_status_distinguishes_outcomes_and_delivery_evidence`:

```rust
let mut delivered_allow = activity("delivered-allow", DeliveryState::Delivered);
delivered_allow.state = ActivityState::Allowed;
assert_eq!(
    live::activity_status(&delivered_allow),
    "allowed · response emitted"
);

let mut delivered_deny = activity("delivered-deny", DeliveryState::Delivered);
delivered_deny.state = ActivityState::Denied;
assert_eq!(
    live::activity_status(&delivered_deny),
    "denied · response emitted"
);
```

- [ ] **Step 2: Run the two TUI tests and confirm the red state**

Run:

```bash
direnv exec . cargo test -p coding-brain-tui delivered_deny_is_recent_and_reports_response_emission
direnv exec . cargo test -p coding-brain-tui live_status_distinguishes_outcomes_and_delivery_evidence
```

Expected: failures show the current `blocked · command did not execute` and `response delivered` strings.

- [ ] **Step 3: Remove the deny special case and rename the delivered copy**

In `activity_status` in `crates/coding-brain-tui/src/ui/brain/live.rs`, delete:

```rust
if matches!(
    (item.state, item.delivery),
    (ActivityState::Denied, DeliveryState::Delivered)
) {
    return "blocked · command did not execute".into();
}
```

Replace the delivered arm with:

```rust
DeliveryState::Delivered => {
    format!("{} · response emitted", decision_state(item.state))
}
```

- [ ] **Step 4: Run the focused TUI regressions**

Run:

```bash
direnv exec . cargo test -p coding-brain-tui delivered_deny_is_recent_and_reports_response_emission
direnv exec . cargo test -p coding-brain-tui live_status_distinguishes_outcomes_and_delivery_evidence
```

Expected: both tests pass.

- [ ] **Step 5: Inspect the task diff without committing**

Run:

```bash
git diff -- crates/coding-brain-tui/src/ui/brain/live.rs crates/coding-brain-tui/src/ui/brain/mod.rs
```

Expected: one evidence-copy branch is simplified and only the directly affected tests change.

### Task 3: Document safe operation and verify the complete mitigation

**Files:**
- Modify: `docs/troubleshooting.md:42-46`
- Verify unchanged: `src/brain/permission_hook.rs`
- Verify unchanged: `src/provider_hooks/antigravity.rs`
- Test unchanged: `tests/hook_activity.rs`

**Interfaces:**
- Consumes: the Doctor row `Antigravity hook contract`, Live copy `response emitted`, and the existing Antigravity `PreToolUse` response contract.
- Produces: operator guidance for the confirmed 1.1.5 failure and a manual validation checklist for future versions.

**Acceptance Criteria:**
- Troubleshooting identifies `agy` 1.1.5 as a confirmed provider-side hook-contract failure when valid `allow` output still leaves the native prompt.
- Guidance keeps the native prompt authoritative, recommends upgrade plus revalidation, and rejects automatic terminal input or always-proceed.
- Documentation states that `response emitted` proves only a successful hook write and that later lifecycle outcome evidence is required for an execution claim.
- Existing Antigravity response-schema and policy tests pass unchanged.
- Workspace tests, clippy with warnings denied, formatting, and build pass.

- [ ] **Step 1: Replace the native-prompt guidance with evidence-specific copy**

Replace the Antigravity paragraph under `## Permission or recovery stayed at the native prompt` in `docs/troubleshooting.md` with:

```markdown
Codex and Claude use their structured `PermissionRequest` responses for allow
and deny. Antigravity uses structured `PreToolUse`; when Coding Brain abstains
or cannot validate input, it returns `ask` and leaves the native prompt in
control. Antigravity `Stop` can return structured `continue` after a validated
automatic recovery decision.

Antigravity CLI (`agy`) 1.1.5 has a confirmed provider-side contract failure:
it can invoke the managed `PreToolUse` hook, receive a valid
`{"decision":"allow"}` response with a successful exit, and still retain the
native tool confirmation. `coding-brain doctor` reports
`Antigravity hook contract` when it detects this exact affected version with
current managed hooks. Keep the native prompt authoritative and upgrade `agy`;
do not enable always-proceed or automatic terminal input as a workaround.

Live's `response emitted` status proves that Coding Brain wrote the hook
response successfully. It does not prove that the provider accepted the
decision or that the tool ran. Only later lifecycle outcome evidence supports
an execution claim.

Before treating a future `agy` release as fixed, repeat the isolated real-binary
check with a temporary hook that consumes stdin, emits only
`{"decision":"allow"}` on stdout, writes nothing to stderr, and exits zero.
Use a harmless command and confirm both automatic execution and the matching
`PostToolUse` event. Versions other than 1.1.5 remain unverified until that
check passes.
```

- [ ] **Step 2: Run the unchanged Antigravity protocol regressions**

Run:

```bash
direnv exec . cargo test --test hook_activity antigravity
```

Expected: all selected tests pass without changes to `src/brain/permission_hook.rs`, `src/provider_hooks/antigravity.rs`, or `tests/hook_activity.rs`.

- [ ] **Step 3: Run the complete focused regression set**

Run:

```bash
direnv exec . cargo test --bin coding-brain doctor::tests::antigravity_
direnv exec . cargo test --bin coding-brain provider_hooks::tests::bounded_process_group_collection_
direnv exec . cargo test -p coding-brain-tui delivered_deny_is_recent_and_reports_response_emission
direnv exec . cargo test -p coding-brain-tui live_status_distinguishes_outcomes_and_delivery_evidence
direnv exec . cargo test --test hook_activity antigravity
```

Expected: every selected test passes.

- [ ] **Step 4: Run formatting and apply only formatter-owned changes**

Run:

```bash
direnv exec . cargo fmt --all --check
```

Expected: success. If it reports formatting differences, run:

```bash
direnv exec . cargo fmt --all
```

Then rerun `direnv exec . cargo fmt --all --check` and expect success.

- [ ] **Step 5: Run full workspace tests**

Run:

```bash
direnv exec . cargo test --workspace
```

Expected: all workspace tests pass.

- [ ] **Step 6: Run clippy with warnings denied**

Run:

```bash
direnv exec . cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clippy exits successfully with no warnings.

- [ ] **Step 7: Run the workspace build**

Run:

```bash
direnv exec . cargo build --workspace
```

Expected: the workspace builds successfully.

- [ ] **Step 8: Audit scope, security invariants, and repository status**

Run:

```bash
git diff --check
git diff --stat
git diff -- src/brain/permission_hook.rs src/provider_hooks/antigravity.rs tests/hook_activity.rs
git status --short
```

Expected:

- `git diff --check` reports no whitespace errors.
- The protocol implementation and its existing contract test have no diff.
- Changed production files are limited to `src/provider_hooks/mod.rs`, `src/doctor.rs`, `crates/coding-brain-tui/src/ui/brain/live.rs`, `crates/coding-brain-tui/src/ui/brain/mod.rs`, and `docs/troubleshooting.md`.
- The approved research, design, and plan artifacts remain visible as untracked internal files unless separately added.
- No commit or push is performed.

## Plan-to-Spec Coverage

- Doctor compatibility advisory, exact affected version, conditional probe, and bounded failure handling: Task 1.
- No terminal fallback, no policy/schema change, and no speculative future-version claim: Global Constraints plus Tasks 1 and 3.
- Allow/deny response-emission wording and outcome-only execution claims: Task 2.
- Operator guidance and real-binary future-release validation: Task 3.
- Protocol safety and full workspace regression evidence: Task 3.

## Execution Tracking

When execution is authorized, create one implementation epic whose description contains:

```markdown
## Success Criteria

Doctor identifies exact affected `agy` 1.1.5 installations, Live preserves the
response-versus-outcome evidence boundary, troubleshooting gives safe operator
guidance, and all focused and workspace quality gates pass without changing the
Antigravity protocol or native permission controls.
```

Create one child task bead for each `Task N`, copying that task's
`Acceptance Criteria` into its `## Acceptance Criteria` section. Task 3 depends
on Tasks 1 and 2; Tasks 1 and 2 may execute independently. After atomic import,
run:

```bash
bd -C /home/alexander/.beads-planning lint <epic-id>
bd -C /home/alexander/.beads-planning list --parent <epic-id> --json \
  | jq -r '.[].id' \
  | xargs -n1 bd -C /home/alexander/.beads-planning lint
bd -C /home/alexander/.beads-planning ready --parent <epic-id> --explain
```

Expected: the epic and all child tasks pass lint, and only Tasks 1 and 2 are initially ready.
