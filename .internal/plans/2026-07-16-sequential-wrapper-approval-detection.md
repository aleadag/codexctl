# Sequential Wrapper Approval Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use beads-superpowers:subagent-driven-development (recommended) or beads-superpowers:executing-plans to implement this plan task-by-task. Each Task becomes a bead (`bd create -t task --parent <epic-id>`). Steps within tasks use checkbox (`- [ ]`) syntax for human readability.

**Goal:** Detect every sequential shell-permission prompt inside one pending Codex `exec` wrapper, show `Needs Input`, and route the exact displayed command to rules and the brain without weakening guarded Enter delivery.

**Architecture:** Keep `CodexSession::pending_tool_*` as transcript-owned identity for the lifetime of the outer call. Project `ApprovalObservation::Confirmed` through two read-only actionable-identity accessors, use those accessors in approval-sensitive consumers, and leave terminal matching plus final revalidation on raw transcript identity and exact evidence.

**Tech Stack:** Rust 2024 workspace, Cargo unit tests, codexctl-core session/terminal/rule modules, binary brain modules, codexctl-tui runtime projections, Jujutsu (`jj`).

**Tracking:** Beads epic `codexctl-jq1`, discovered from brainstorming bead `codexctl-b3q` and implementing bug `codexctl-fjx`.

## Global Constraints

- `pending_tool_name`, `pending_tool_call_id`, and `pending_tool_input` remain owned by transcript monitoring and are never rewritten by terminal observation.
- Only `ApprovalObservation::Confirmed` may override the actionable tool and command exposed to rules, brain, and TUI consumers.
- `approve_shell_permission_with` must retain its second terminal capture and exact `ApprovalEvidence` equality check before sending Enter.
- Missing, partial, stale, unsupported, ambiguous, or failed terminal captures remain non-actionable.
- Direct shell calls, `request_user_input`, non-approval tools, prompt patterns, terminal targeting, hook integration, and persisted state retain existing behavior.
- Do not add duplicate wrapper fields, persisted state, configuration, migrations, feature flags, or unrelated refactors.
- Use jj only; every implementation changeset description uses `<emoji> <type>: <imperative summary>`.
- Keep all four implementation tasks in one jj changeset, `🐛 fix: preserve sequential approval detection`; Beads tasks provide the per-task review checkpoints.
- Reuse epic `codexctl-jq1` and its existing child tasks during execution; do not create duplicate implementation beads.
- Run root-package brain tests and the full test suite with a temporary `HOME` plus `CARGO_HOME=/home/alexander/.cargo` so verification cannot write the live brain store.
- Changes outside the six scoped source files require a plan amendment and user approval before implementation continues.

## File Structure

- `crates/codexctl-core/src/session.rs`: define the single actionable-identity interface and its fallback semantics.
- `crates/codexctl-core/src/terminals/mod.rs`: stop mutating transcript identity and reproduce two sequential prompts under one wrapper.
- `crates/codexctl-core/src/rules.rs`: match and describe the terminal-confirmed request.
- `src/brain/context.rs`: present the confirmed request to the model.
- `src/brain/engine.rs`: bind inference, thresholds, retrieval, conflict checks, and decision records to actionable identity.
- `crates/codexctl-tui/src/app.rs`: project actionable identity into observations, decisions, and pending-suggestion messages.

## Execution Order

1. `codexctl-xqo` implements the core identity boundary.
2. `codexctl-06v` (rules) and `codexctl-hvo` (brain) both depend only on `codexctl-xqo`; execute them in either order.
3. `codexctl-5y2` waits for both consumer tasks, migrates the TUI, audits all Rust sources, and runs final gates.

All four tasks reuse one `🐛 fix: preserve sequential approval detection` jj changeset. Claim and close the existing child beads as review checkpoints; do not create replacements.

---

### Task 1: Preserve Transcript Identity and Reproduce Sequential Prompts

**Files:**
- Modify: `crates/codexctl-core/src/session.rs:327`
- Modify: `crates/codexctl-core/src/session.rs:937`
- Modify: `crates/codexctl-core/src/terminals/mod.rs:1464`
- Modify: `crates/codexctl-core/src/terminals/mod.rs:1535`

**Interfaces:**
- Consumes: `ApprovalObservation::Confirmed(ApprovalEvidence)` and existing transcript-populated `pending_tool_name` / `pending_tool_input`.
- Produces: `CodexSession::actionable_tool_name(&self) -> Option<&str>` and `CodexSession::actionable_tool_input(&self) -> Option<&str>` for Tasks 2-4.

**Acceptance Criteria:**
- Confirmed evidence is exposed as actionable identity without altering raw pending fields.
- Non-confirmed observations fall back to raw pending identity.
- Two different complete prompts separated by a no-prompt capture are both confirmed under one unchanged wrapper and call ID.
- The second confirmed prompt passes final revalidation and sends exactly one Enter.
- Existing exact revalidation, incomplete-prompt, mismatch, backend-target, and `request_user_input` tests remain green.

- [ ] **Step 1: Start an isolated jj changeset**

Run:

```bash
jj new -m "🐛 fix: preserve sequential approval detection"
```

Expected: `@` is a new empty changeset with the exact description above.

- [ ] **Step 2: Add the failing sequential-wrapper regression without new APIs**

In `crates/codexctl-core/src/terminals/mod.rs`, first revise `exec_wrapper_uses_last_complete_visible_prompt` so its final assertions preserve raw wrapper identity:

```rust
assert_eq!(session.pending_tool_name.as_deref(), Some("exec"));
assert_eq!(session.pending_tool_input.as_deref(), Some(wrapper));
```

Bind the existing wrapper input to `let wrapper = ...` so the assertion uses the exact original value. Then add:

```rust
#[test]
fn exec_wrapper_confirms_sequential_prompts_without_rewriting_transcript_identity() {
    let fixture = include_str!("../../../../tests/fixtures/codex-shell-approval-pane.txt");
    let clippy = fixture.replace("$ cargo test", "$ cargo clippy");
    let wrapper = "const args = next(); await tools.exec_command(args);";
    let mut session = pending_exec_wrapper_session("call-7", wrapper);
    let io = FakeApprovalIo::with_captures([
        Ok(capture(fixture)),
        Ok(capture(include_str!(
            "../../../../tests/fixtures/codex-running-shell-pane.txt"
        ))),
        Ok(capture(&clippy)),
        Ok(capture(&clippy)),
    ]);

    refresh_approval_observation_with(&io, &mut session, 10_000);
    let ApprovalObservation::Confirmed(first) = &session.approval else {
        panic!("first wrapper approval was not confirmed");
    };
    assert_eq!(first.command, "cargo test");
    assert_eq!(session.pending_tool_name.as_deref(), Some("exec"));
    assert_eq!(session.pending_tool_input.as_deref(), Some(wrapper));

    refresh_approval_observation_with(&io, &mut session, 11_000);
    assert!(matches!(session.approval, ApprovalObservation::Unknown(_)));
    assert_eq!(session.pending_tool_name.as_deref(), Some("exec"));
    assert_eq!(session.pending_tool_input.as_deref(), Some(wrapper));

    refresh_approval_observation_with(&io, &mut session, 12_000);
    let ApprovalObservation::Confirmed(evidence) = &session.approval else {
        panic!("second wrapper approval was not confirmed");
    };
    assert_eq!(evidence.command, "cargo clippy");
    assert_eq!(session.pending_tool_name.as_deref(), Some("exec"));
    assert_eq!(session.pending_tool_input.as_deref(), Some(wrapper));

    approve_shell_permission_with(&io, &session).unwrap();
    assert_eq!(io.sends.load(Ordering::SeqCst), 1);
}
```

- [ ] **Step 3: Run the terminal regression and verify the red state**

Run:

```bash
cargo test -p codexctl-core terminals::tests::exec_wrapper
```

Expected: FAIL because the first confirmed prompt rewrites `pending_tool_name` to `exec_command` and `pending_tool_input` to `cargo test`; the later `cargo clippy` prompt cannot be confirmed under the same outer call.

- [ ] **Step 4: Add actionable-identity unit tests**

Add these tests to `crates/codexctl-core/src/session.rs` inside its existing `tests` module:

```rust
#[test]
fn actionable_identity_prefers_confirmed_evidence() {
    let mut session = make_session();
    session.pending_tool_name = Some("exec".into());
    session.pending_tool_call_id = Some("call-1".into());
    session.pending_tool_input = Some("await tools.exec_command(args);".into());
    session.approval = ApprovalObservation::Confirmed(ApprovalEvidence {
        session_id: session.session_id.clone(),
        tty: session.tty.clone(),
        call_id: "call-1".into(),
        tool: "exec_command".into(),
        command: "install -m 664 source target".into(),
        backend: Terminal::Tmux,
        target: "main:1.0".into(),
        prompt_pattern_version: 1,
        prompt_fingerprint: 42,
    });

    assert_eq!(session.actionable_tool_name(), Some("exec_command"));
    assert_eq!(
        session.actionable_tool_input(),
        Some("install -m 664 source target")
    );
    assert_eq!(session.pending_tool_name.as_deref(), Some("exec"));
    assert_eq!(
        session.pending_tool_input.as_deref(),
        Some("await tools.exec_command(args);")
    );
}

#[test]
fn non_confirmed_identity_falls_back_to_pending_call() {
    for approval in [
        ApprovalObservation::NotChecked,
        ApprovalObservation::Unknown("no matching prompt".into()),
    ] {
        let mut session = make_session();
        session.pending_tool_name = Some("exec".into());
        session.pending_tool_input = Some("await tools.exec_command(args);".into());
        session.approval = approval;

        assert_eq!(session.actionable_tool_name(), Some("exec"));
        assert_eq!(
            session.actionable_tool_input(),
            Some("await tools.exec_command(args);")
        );
    }
}
```

- [ ] **Step 5: Implement the actionable accessors**

Add to `impl CodexSession` in `crates/codexctl-core/src/session.rs`:

```rust
/// Tool identity currently presented to consumers.
///
/// This is a projection, not approval authorization. Guarded input still
/// requires terminal-confirmed evidence and final revalidation.
pub fn actionable_tool_name(&self) -> Option<&str> {
    match &self.approval {
        ApprovalObservation::Confirmed(evidence) => Some(evidence.tool.as_str()),
        ApprovalObservation::NotChecked | ApprovalObservation::Unknown(_) => {
            self.pending_tool_name.as_deref()
        }
    }
}

/// Tool input currently presented to consumers.
///
/// This is a projection, not approval authorization. Guarded input still
/// requires terminal-confirmed evidence and final revalidation.
pub fn actionable_tool_input(&self) -> Option<&str> {
    match &self.approval {
        ApprovalObservation::Confirmed(evidence) => Some(evidence.command.as_str()),
        ApprovalObservation::NotChecked | ApprovalObservation::Unknown(_) => {
            self.pending_tool_input.as_deref()
        }
    }
}
```

- [ ] **Step 6: Stop terminal refresh from mutating transcript identity**

Delete only this block from `refresh_approval_observation_with` in `crates/codexctl-core/src/terminals/mod.rs`:

```rust
if let ApprovalObservation::Confirmed(evidence) = &observation {
    session.pending_tool_name = Some(evidence.tool.clone());
    session.pending_tool_input = Some(evidence.command.clone());
}
```

Keep `session.approval = observation;` and all code in `approve_shell_permission_with` unchanged.

- [ ] **Step 7: Run core tests and verify green**

Run:

```bash
cargo test -p codexctl-core session::tests
cargo test -p codexctl-core terminals::tests
```

Expected: all selected tests pass, including both sequential prompts and every guarded-input regression.

- [ ] **Step 8: Finalize the task changeset**

Run:

```bash
cargo fmt --check
jj --no-pager st
jj --no-pager log -r '@|@-' --no-graph
```

Expected: formatting passes; only the two core files are changed in `🐛 fix: preserve sequential approval detection`.

---

### Task 2: Match Rules Against Actionable Identity

**Files:**
- Modify: `crates/codexctl-core/src/rules.rs:120`
- Modify: `crates/codexctl-core/src/rules.rs:268`

**Interfaces:**
- Consumes: `CodexSession::actionable_tool_name()` and `CodexSession::actionable_tool_input()` from Task 1.
- Produces: rule matching and rule-result messages that describe the confirmed displayed request.

**Acceptance Criteria:**
- A rule can match `exec_command` and `install ...` when raw pending identity is still `exec` plus wrapper JavaScript.
- Rules do not match wrapper source when confirmed evidence is present.
- Deny-rule precedence and guarded approval execution remain unchanged.

- [ ] **Step 1: Confirm the shared implementation changeset**

Run:

```bash
jj --no-pager st
jj --no-pager log -r '@' --no-graph
```

Expected: `@` remains `🐛 fix: preserve sequential approval detection` with Task 1's two core files; do not run `jj new`.

- [ ] **Step 2: Add a failing confirmed-wrapper rule test**

Extend the test imports in `crates/codexctl-core/src/rules.rs` with `ApprovalEvidence`, `ApprovalObservation`, and `crate::terminals::Terminal`, then add:

```rust
#[test]
fn confirmed_wrapper_rules_match_displayed_command() {
    let mut session = make_session();
    session.pending_tool_name = Some("exec".into());
    session.pending_tool_call_id = Some("call-1".into());
    session.pending_tool_input = Some("await tools.exec_command(args);".into());
    session.approval = ApprovalObservation::Confirmed(ApprovalEvidence {
        session_id: session.session_id.clone(),
        tty: session.tty.clone(),
        call_id: "call-1".into(),
        tool: "exec_command".into(),
        command: "install -m 664 source target".into(),
        backend: Terminal::Tmux,
        target: "main:1.0".into(),
        prompt_pattern_version: 1,
        prompt_fingerprint: 42,
    });

    let mut displayed = approve_rule("displayed-command");
    displayed.match_tool = vec!["exec_command".into()];
    displayed.match_command = vec!["install -m 664".into()];
    assert!(evaluate(&[displayed], &session).is_some());

    let mut wrapper = approve_rule("wrapper-source");
    wrapper.match_tool = vec!["exec".into()];
    wrapper.match_command = vec!["tools.exec_command".into()];
    assert!(evaluate(&[wrapper], &session).is_none());
}
```

- [ ] **Step 3: Run the rule test and verify it fails**

Run:

```bash
cargo test -p codexctl-core rules::tests::confirmed_wrapper_rules_match_displayed_command
```

Expected: FAIL because `matches_rule` reads raw wrapper fields.

- [ ] **Step 4: Migrate rule matching and result labels**

Use the accessors in `matches_rule`:

```rust
if !rule.match_tool.is_empty() {
    let tool = match session.actionable_tool_name() {
        Some(tool) => tool.to_lowercase(),
        None => return false,
    };
    let any_match = rule.match_tool.iter().any(|value| tool == value.to_lowercase());
    if !any_match {
        return false;
    }
}

if !rule.match_command.is_empty() {
    let command = match session.actionable_tool_input() {
        Some(command) => command.to_lowercase(),
        None => return false,
    };
    let any_match = rule
        .match_command
        .iter()
        .any(|pattern| command.contains(&pattern.to_lowercase()));
    if !any_match {
        return false;
    }
}
```

In the approve and deny result messages, replace both raw tool lookups with:

```rust
session.actionable_tool_name().unwrap_or("?")
```

Do not change the `ApprovalObservation::Confirmed` execution gate or `terminals::approve_shell_permission(session)`.

- [ ] **Step 5: Run all rule tests**

Run:

```bash
cargo test -p codexctl-core rules::tests
```

Expected: all rule tests pass, including deny precedence and the confirmed-wrapper test.

- [ ] **Step 6: Finalize the task changeset**

Run:

```bash
cargo fmt --check
jj --no-pager st
jj --no-pager log -r '@|@-' --no-graph
```

Expected: the same `🐛 fix: preserve sequential approval detection` changeset now contains the two core files plus `crates/codexctl-core/src/rules.rs`.

---

### Task 3: Bind Brain Context and Decisions to the Confirmed Request

**Files:**
- Modify: `src/brain/context.rs:60`
- Modify: `src/brain/context.rs:492`
- Modify: `src/brain/engine.rs:21`
- Modify: `src/brain/engine.rs:150`
- Modify: `src/brain/engine.rs:388`
- Modify: `src/brain/engine.rs:748`
- Modify: `src/brain/engine.rs:854`

**Interfaces:**
- Consumes: actionable identity accessors from Task 1; continues calling the existing rules interface without depending on Task 2's implementation.
- Produces: brain prompts, target snapshots, retrieval keys, thresholds, conflict classification, and decision records bound to the confirmed displayed request.

**Acceptance Criteria:**
- Brain context contains `exec_command` and the displayed nested command, not outer wrapper source.
- `BrainTargetIdentity` stores actionable tool/input while retaining call ID and full approval evidence.
- All production decision logging, adaptive threshold, similar-decision retrieval, and file-conflict classification read actionable identity.
- A suggestion bound to prompt A expires when the same wrapper presents prompt B.

- [ ] **Step 1: Confirm the shared implementation changeset**

Run:

```bash
jj --no-pager st
jj --no-pager log -r '@' --no-graph
```

Expected: `@` remains `🐛 fix: preserve sequential approval detection` with completed core work; Task 2 may also be present, but Task 3 does not require it. Do not run `jj new`.

- [ ] **Step 2: Add failing brain-context coverage**

Extend the `src/brain/context.rs` test imports with `ApprovalEvidence`, `ApprovalObservation`, and `codexctl_core::terminals::Terminal`. Add this helper and test:

```rust
fn confirm_wrapper_command(session: &mut CodexSession, command: &str) {
    session.pending_tool_name = Some("exec".into());
    session.pending_tool_call_id = Some("call-1".into());
    session.pending_tool_input = Some("await tools.exec_command(args);".into());
    session.approval = ApprovalObservation::Confirmed(ApprovalEvidence {
        session_id: session.session_id.clone(),
        tty: session.tty.clone(),
        call_id: "call-1".into(),
        tool: "exec_command".into(),
        command: command.into(),
        backend: Terminal::Tmux,
        target: "main:1.0".into(),
        prompt_pattern_version: 1,
        prompt_fingerprint: 42,
    });
}

#[test]
fn brain_context_uses_confirmed_wrapper_command() {
    let mut session = make_session();
    confirm_wrapper_command(&mut session, "install -m 664 source target");

    let summary = format_session_summary(&session);
    let prompt = format_decision_prompt(&session);
    let context = build_context(&session, std::slice::from_ref(&session), 4000);

    assert!(summary.contains("exec_command"));
    assert!(summary.contains("install -m 664 source target"));
    assert!(!summary.contains("await tools.exec_command(args);"));
    assert!(prompt.contains("exec_command"));
    assert!(!prompt.contains("await tools.exec_command(args);"));
    assert!(context.session_summary.contains("exec_command"));
    assert!(
        context
            .session_summary
            .contains("install -m 664 source target")
    );
    assert!(
        !context
            .session_summary
            .contains("await tools.exec_command(args);")
    );
}
```

- [ ] **Step 3: Add failing brain-target coverage**

In `src/brain/engine.rs`, add a wrapper helper beside `confirmed_shell_session`:

```rust
fn confirmed_wrapper_session(command: &str, fingerprint: u64) -> CodexSession {
    let mut session = confirmed_shell_session("call-wrapper", command);
    session.pending_tool_name = Some("exec".into());
    session.pending_tool_input = Some("await tools.exec_command(args);".into());
    let ApprovalObservation::Confirmed(evidence) = &mut session.approval else {
        unreachable!();
    };
    evidence.prompt_fingerprint = fingerprint;
    session
}
```

Add:

```rust
#[test]
fn brain_target_uses_actionable_wrapper_identity() {
    let session = confirmed_wrapper_session("cargo test", 41);
    let target = BrainTargetIdentity::from_session(&session);

    assert_eq!(target.pending_tool_name.as_deref(), Some("exec_command"));
    assert_eq!(target.pending_tool_input.as_deref(), Some("cargo test"));
}

#[test]
fn wrapper_prompt_change_expires_pending_suggestion() {
    let mut engine = BrainEngine::new(make_config());
    let original = confirmed_wrapper_session("cargo test", 41);
    let replacement = confirmed_wrapper_session("cargo clippy", 42);
    engine.pending.insert(
        original.pid,
        PendingBrainSuggestion::bound(
            suggestion(RuleAction::Approve, 1.0),
            Some(BrainTargetIdentity::from_session(&original)),
        ),
    );

    let message = engine.accept(original.pid, &replacement).unwrap();

    assert!(message.contains("expired"));
    assert!(engine.pending.is_empty());
}
```

- [ ] **Step 4: Run focused brain tests and verify the red state**

Run:

```bash
HOME="$(mktemp -d)" CARGO_HOME=/home/alexander/.cargo cargo test --lib brain::context::tests::brain_context_uses_confirmed_wrapper_command
HOME="$(mktemp -d)" CARGO_HOME=/home/alexander/.cargo cargo test --lib brain::engine::tests::brain_target_uses_actionable_wrapper_identity
HOME="$(mktemp -d)" CARGO_HOME=/home/alexander/.cargo cargo test --lib brain::engine::tests::wrapper_prompt_change_expires_pending_suggestion
```

Expected: context and target-identity tests fail because production code reads raw wrapper fields. The expiry test passes as a security guard through full approval-evidence inequality; it is not part of the red-state proof.

- [ ] **Step 5: Migrate brain context**

Replace the pending section in `format_session_summary` with:

```rust
if let Some(tool) = session.actionable_tool_name() {
    summary.push_str(&format!(" | Pending tool: {tool}"));
    if let Some(input) = session.actionable_tool_input() {
        let truncated = if input.len() > 200 {
            format!("{}...", session::truncate_str(input, 200))
        } else {
            input.to_string()
        };
        summary.push_str(&format!(" | Command: {truncated}"));
    }
}
```

In the `NeedsInput` arm of `format_decision_prompt`, replace the tool lookup with:

```rust
let tool = session.actionable_tool_name().unwrap_or("unknown");
```

Replace `tool_info` in `format_global_session_map` with:

```rust
let tool_info = match s.actionable_tool_name() {
    Some(tool) => {
        let command = s
            .actionable_tool_input()
            .map(|command| {
                if command.len() > 60 {
                    format!(" \"{}...\"", session::truncate_str(command, 60))
                } else {
                    format!(" \"{command}\"")
                }
            })
            .unwrap_or_default();
        format!(" [{}{}]", tool, command)
    }
    None => String::new(),
};
```

Preserve existing truncation limits and prompt wording.

- [ ] **Step 6: Migrate brain target identity and consumers**

Change `BrainTargetIdentity::from_session` to own the accessor results:

```rust
pending_tool_name: session.actionable_tool_name().map(str::to_owned),
pending_tool_input: session.actionable_tool_input().map(str::to_owned),
```

In production code in `src/brain/engine.rs`, make these exact mechanical substitutions:

```rust
session.pending_tool_name.as_deref()  // old
session.actionable_tool_name()        // new

session.pending_tool_input.as_deref() // old
session.actionable_tool_input()       // new
```

Apply them to deny-rule override logging, adaptive threshold lookup, low-confidence logging, conflict logging, auto-action logging, similar-decision retrieval, and `check_file_conflicts`. Do not rewrite test fixture assignments or transcript state.

- [ ] **Step 7: Run all brain tests**

Run:

```bash
HOME="$(mktemp -d)" CARGO_HOME=/home/alexander/.cargo cargo test --lib brain::context::tests
HOME="$(mktemp -d)" CARGO_HOME=/home/alexander/.cargo cargo test --lib brain::engine::tests
```

Expected: all brain context, target-expiry, rule-override, threshold, conflict, and orchestration tests pass.

- [ ] **Step 8: Finalize the task changeset**

Run:

```bash
cargo fmt --check
jj --no-pager st
jj --no-pager log -r '@|@-' --no-graph
```

Expected: the same `🐛 fix: preserve sequential approval detection` changeset now also contains `src/brain/context.rs` and `src/brain/engine.rs`.

---

### Task 4: Project Actionable Identity Through the TUI and Validate the Workspace

**Files:**
- Modify: `crates/codexctl-tui/src/app.rs:433`
- Modify: `crates/codexctl-tui/src/app.rs:1100`
- Modify: `crates/codexctl-tui/src/app.rs:2320`
- Modify: `crates/codexctl-tui/src/app.rs:2573`

**Interfaces:**
- Consumes: actionable identity accessors from Task 1 and the migrated rules/brain behavior from Tasks 2-3.
- Produces: observations, accepted/rejected decision inputs, and pending-suggestion messages containing the confirmed displayed request.

**Acceptance Criteria:**
- Passive observations contain `exec_command` plus the displayed nested command.
- Brain accept/reject decision inputs use actionable identity.
- Demo pending messages retain current behavior through accessor fallback.
- No approval-sensitive production consumer in the scoped files still reads raw wrapper identity.
- Full formatting, test, clippy, and build gates pass.

- [ ] **Step 1: Confirm the shared implementation changeset**

Run:

```bash
jj --no-pager st
jj --no-pager log -r '@' --no-graph
```

Expected: `@` remains `🐛 fix: preserve sequential approval detection` with Tasks 1-3 present. Do not run `jj new`.

- [ ] **Step 2: Add a failing TUI observation test**

Extend the test imports in `crates/codexctl-tui/src/app.rs` with `ApprovalEvidence`, `ApprovalObservation`, and `codexctl_core::terminals::Terminal`, then add:

```rust
#[test]
fn observation_projects_confirmed_wrapper_command() {
    let mut session = make_session(
        11,
        "approval-project",
        "gpt-5.5",
        SessionStatus::NeedsInput,
        0.0,
        0.0,
        true,
    );
    session.pending_tool_name = Some("exec".into());
    session.pending_tool_call_id = Some("call-1".into());
    session.pending_tool_input = Some("await tools.exec_command(args);".into());
    session.approval = ApprovalObservation::Confirmed(ApprovalEvidence {
        session_id: session.session_id.clone(),
        tty: session.tty.clone(),
        call_id: "call-1".into(),
        tool: "exec_command".into(),
        command: "install -m 664 source target".into(),
        backend: Terminal::Tmux,
        target: "main:1.0".into(),
        prompt_pattern_version: 1,
        prompt_fingerprint: 42,
    });

    let observation = observation_from(&session, "user_approve");

    assert_eq!(observation.tool.as_deref(), Some("exec_command"));
    assert_eq!(
        observation.command.as_deref(),
        Some("install -m 664 source target")
    );
}
```

- [ ] **Step 3: Run the TUI test and verify it fails**

Run:

```bash
HOME="$(mktemp -d)" CARGO_HOME=/home/alexander/.cargo cargo test -p codexctl-tui app::tests::observation_projects_confirmed_wrapper_command
```

Expected: FAIL because `observation_from` clones the raw wrapper fields.

- [ ] **Step 4: Migrate TUI projections**

Change `observation_from` to:

```rust
codexctl_core::runtime::ObservationInput {
    session_pid: session.pid,
    project: session.display_name().to_string(),
    tool: session.actionable_tool_name().map(str::to_owned),
    command: session.actionable_tool_input().map(str::to_owned),
    observed_action: action.to_string(),
}
```

In both `LogDecisionInput` constructors used by brain accept/reject, set:

```rust
tool: session.actionable_tool_name().map(str::to_owned),
command: session.actionable_tool_input().map(str::to_owned),
```

In both demo `PendingSuggestion` constructors, set:

```rust
message: s.actionable_tool_input().map(str::to_owned),
```

Do not change `merge_discovered_session`, terminal approval execution, or test-fixture assignments.

- [ ] **Step 5: Run the TUI tests**

Run:

```bash
HOME="$(mktemp -d)" CARGO_HOME=/home/alexander/.cargo cargo test -p codexctl-tui app::tests
```

Expected: all TUI app tests pass without reading or writing the user's home directory.

- [ ] **Step 6: Audit raw-field ownership**

Run:

```bash
rg -n "pending_tool_(name|input)" crates src
```

Expected remaining raw-field uses are limited to:

- transcript parsing/clearing in `monitor.rs`;
- terminal matching in `terminals/mod.rs`;
- accessor fallback and test fixture assignments in `session.rs`;
- synthetic pending-state construction in `commands.rs` and brain eval fixtures;
- test fixture assignments in rules, brain, runtime, and TUI modules.

Any production rule, brain, or TUI projection still reading raw pending identity must be migrated before continuing.

- [ ] **Step 7: Run the full quality gates**

Run:

```bash
cargo fmt
cargo fmt --check
LIVE_DECISIONS=/home/alexander/.codexctl/brain/decisions.jsonl
BEFORE_TEST_RECORDS="$(jq -s 'map(select(.project == "test")) | length' "$LIVE_DECISIONS")"
HOME="$(mktemp -d)" CARGO_HOME=/home/alexander/.cargo cargo test
AFTER_TEST_RECORDS="$(jq -s 'map(select(.project == "test")) | length' "$LIVE_DECISIONS")"
test "$BEFORE_TEST_RECORDS" = "$AFTER_TEST_RECORDS"
cargo clippy -- -D warnings
cargo build
```

Expected: every command exits 0 with no warnings from clippy, and the live decision store's exact `project == "test"` count is unchanged.

- [ ] **Step 8: Verify final jj scope**

Run:

```bash
jj --no-pager st
jj --no-pager diff --git
jj --no-pager log -r 'ancestors(@, 4)' --no-graph
```

Expected: one `🐛 fix: preserve sequential approval detection` implementation changeset follows the approved spec/plan changeset; no unrelated files, configuration, persistence, or prompt-pattern changes are present.

## Stress Test Results: Sequential Wrapper Approval Implementation Plan

### Resolved Decisions

- Task dependencies: core blocks rules and brain independently; TUI/final validation waits for both consumers.
- TDD validity: the first red test uses existing APIs and fails on raw identity mutation; accessor tests follow it; prompt-expiry remains a passing security guard.
- Accessor contract: actionable methods are projections, not authorization, and both `NotChecked` and `Unknown` fallback behavior are tested.
- Consumer coverage: the final audit scans every Rust source and classifies all remaining raw-field uses.
- Security: the sequential regression revalidates prompt B and asserts exactly one Enter, while existing mismatch tests retain zero-Enter guarantees.
- Verification isolation: root brain tests and the full suite use temporary homes, and the live `project == "test"` count is compared before and after.
- jj execution and rollback: all implementation tasks share one changeset and reuse the existing Beads graph.
- Scope: changes remain limited to the six approved source files; hooks, parser behavior, persistence, and permanent test isolation stay separate.

### Changes Made

- Rewired Beads dependencies to `core -> {rules, brain} -> TUI`.
- Reordered Task 1 so the behavioral regression fails before new accessor APIs are introduced.
- Added explicit projection-only documentation and `Unknown` fallback coverage.
- Added guarded approval of the second sequential prompt to the core regression.
- Expanded the raw-field audit from selected files to all `crates` and `src` Rust sources.
- Isolated brain/full tests from the live home and added a live-store contamination check.
- Replaced four implementation changesets with one atomic jj changeset and Beads review checkpoints.

### Deferred / Parking Lot

- Permanent brain test-state isolation remains tracked by its existing approved design and plan.
- Hook lifecycle integration and active-prompt parser hardening remain separate work.

### Confidence Assessment

- Overall: High
- Areas of concern: execution must complete the global consumer audit and preserve the temporary-home test wrapper exactly.
