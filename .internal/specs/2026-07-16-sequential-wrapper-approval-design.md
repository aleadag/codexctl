# Sequential Wrapper Approval Detection Design

## Problem

One Codex `exec` custom tool call can run several nested `exec_command` calls. Each nested command may independently request shell permission, but the transcript emits only the outer call until that wrapper yields or completes.

codexctl currently promotes `pending_tool_name` and `pending_tool_input` from the outer transcript identity to the first confirmed terminal prompt. After that prompt disappears, no transcript event restores the outer identity. A later prompt in the same wrapper is therefore compared as though it were the first direct command. The comparison fails, the session remains `Processing`, and the brain never sees the later permission request.

The fix must detect every sequential prompt while retaining the exact terminal revalidation that prevents stale or ambiguous approvals from sending Enter.

## Goals

- Every complete, visible permission prompt inside a still-pending outer wrapper becomes `Needs Input`.
- Rules, brain context, decision logging, and TUI messages receive the exact tool and command shown by the currently confirmed prompt.
- The transcript-derived pending call identity remains stable until a transcript event changes or completes that call.
- Approval execution still re-captures the terminal and requires exact evidence equality before sending Enter.
- Direct shell calls and non-approval pending tools retain their existing behavior.

## Non-goals

- Parsing nested tool calls from JavaScript beyond the terminal prompt already visible to codexctl.
- Adding persisted session fields or migrating stored state.
- Changing prompt patterns, terminal targeting, brain policy, or hook integration.
- Fixing brain test-state isolation in the same change.

## Design

### Separate transcript identity from actionable identity

`CodexSession::pending_tool_name` and `pending_tool_input` remain the transcript identity. Monitor code is their owner: it sets them when a pending call appears and clears them when that call completes. Terminal observation must not rewrite them.

`CodexSession` gains two read-only accessors:

- `actionable_tool_name() -> Option<&str>`
- `actionable_tool_input() -> Option<&str>`

When `session.approval` is `ApprovalObservation::Confirmed(evidence)`, these accessors return `evidence.tool` and `evidence.command`. Otherwise they return the transcript-derived pending fields.

This creates one explicit boundary:

- terminal matching consumes immutable transcript identity;
- approval-sensitive consumers consume actionable identity.

No duplicate wrapper fields or restoration state are needed.

### Approval refresh

`refresh_approval_observation_with` continues to:

1. verify that the transcript describes a pending shell-capable call;
2. capture the exact terminal target;
3. match a complete visible Codex permission prompt against the pending direct call or wrapper;
4. store `Confirmed(evidence)` or a safe non-confirmed observation.

It stops assigning the evidence tool and command back into `pending_tool_name` and `pending_tool_input`.

For sequential prompts in one wrapper, each refresh therefore starts from the same outer `exec` identity. Prompt A may be confirmed, disappear, and be replaced by prompt B without waiting for a transcript event; prompt B is independently matched against the wrapper and produces new evidence.

### Consumers

Approval-sensitive projections use the actionable accessors:

- rule tool and command matching, plus rule result messages;
- brain target identity, adaptive threshold lookup, context construction, conflict checks where tool identity matters, and decision logging;
- TUI observations, brain accept/reject logs, and user-facing pending-command messages;
- runtime action inputs derived from a live session.

Terminal prompt matching and final approval revalidation continue to use the transcript identity and the stored `ApprovalEvidence` directly. Monitor parsing continues to write the pending fields directly.

Consumers that intentionally inspect the raw outer transcript call keep using the fields directly. The implementation will change only call sites whose purpose is to describe or act on the current user-visible request.

## Data Flow

1. Transcript monitor observes outer call `exec(<wrapper JavaScript>)` and stores its call ID, tool, and input.
2. Terminal capture finds prompt A, such as `cp ...`.
3. Matching uses the outer wrapper and returns evidence for `exec_command(cp ...)`.
4. Status becomes `Needs Input`; actionable accessors expose `exec_command` and `cp ...` to rules and brain.
5. Before approval, codexctl captures again and requires the same session, call ID, backend, target, prompt version, fingerprint, tool, and command.
6. Prompt A disappears. Approval becomes non-confirmed; actionable accessors fall back to the still-pending outer wrapper.
7. Terminal capture finds prompt B, such as `install ...`; matching again uses the unchanged wrapper and returns new evidence.
8. Status becomes `Needs Input`; the brain receives `install ...`.
9. The transcript eventually completes the outer call and monitor code clears the pending identity.

## Safety and Error Handling

- `Confirmed` remains the only observation that permits automatic approval.
- A missing, partial, stale, unsupported, or ambiguous prompt remains `Unknown` and cannot send Enter.
- `approve_shell_permission_with` retains its second capture and exact evidence comparison immediately before input delivery.
- A change from prompt A to prompt B invalidates any brain suggestion bound to prompt A because the target includes the approval evidence and actionable identity.
- Falling back to wrapper identity while no prompt is confirmed does not make the wrapper approvable; approval execution still requires `Confirmed` evidence.
- No command is inferred from wrapper text for brain execution. Only terminal-confirmed evidence becomes actionable.

## Testing

### Core regression

Add a terminal test using one pending wrapper and one call ID:

1. capture a complete approval prompt for `cargo test` and assert `Confirmed`;
2. assert the raw pending identity is still the original `exec` wrapper while actionable identity is `exec_command` plus `cargo test`;
3. capture a running pane with no prompt and assert a non-confirmed observation;
4. capture a complete approval prompt for `cargo clippy` without any intervening transcript update;
5. assert the second prompt is `Confirmed`, raw identity is unchanged, and actionable identity is `exec_command` plus `cargo clippy`.

This test fails under the current promotion behavior.

### Consumer coverage

- Session accessor tests cover confirmed evidence and fallback behavior.
- Rule tests prove matching uses the confirmed displayed command rather than wrapper source.
- Brain tests prove context and target identity use actionable identity and that a changed prompt expires an earlier suggestion.
- TUI/runtime projection tests prove logged observations and decisions contain the displayed command.
- Existing direct-call, incomplete-prompt, mismatched-command, stale-evidence, backend-target, fingerprint, and `request_user_input` tests remain green.

### Quality gates

Run:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build
```

## Acceptance Criteria

- Two sequential permission prompts within one outer `exec` call are both detected as `Needs Input` without an intervening transcript event.
- The brain receives and logs the exact command from each confirmed prompt.
- Raw pending transcript identity is not mutated by terminal observation.
- Stale or ambiguous prompts cannot send Enter.
- The targeted regression tests and full workspace quality gates pass.

## Stress Test Results: Sequential Wrapper Approval Detection

### Resolved Decisions

- Identity ownership: transcript monitoring exclusively owns raw pending-call identity; terminal evidence supplies only the temporary actionable identity.
- Consumer boundary: rules, brain, TUI messages, and decision projections use actionable accessors, while transcript parsing and terminal matching use raw pending fields.
- Prompt lifecycle: every refresh recomputes evidence from the unchanged wrapper and immediately drops `Confirmed` when the prompt disappears or capture fails.
- Failure recovery: incomplete, unsupported, ambiguous, or unavailable captures remain non-actionable and retry on the next refresh.
- Security: approval retains the final terminal re-capture and exact evidence equality check before Enter; brain suggestions remain bound to current evidence.
- Concurrency and scale: all state remains per-session with no prompt registry, queue, persistence, or shared synchronization.
- Alternatives: duplicate wrapper fields, identity restoration, and relying on optional hooks were rejected as less reliable or insufficient for guarded input.
- Testing: implementation starts with a failing two-prompt regression and adds consumer-boundary coverage while preserving existing safety tests.
- Rollback: the in-memory change remains atomic and requires no feature flag, migration, or cleanup.

### Changes Made

- Appended the explicit stress-test resolution record. The approved architecture and scope required no modification.

### Deferred / Parking Lot

- Hook-based lifecycle signals remain tracked separately and may complement, but do not replace, terminal-confirmed approval evidence.
- Active-prompt detection for an older permission block that remains visibly rendered is a pre-existing parser concern and is not broadened into this identity fix.
- Brain test-state isolation remains paused until this status regression is fixed.

### Confidence Assessment

- Overall: High
- Areas of concern: consumer migration must be complete; targeted tests must prove no approval-sensitive projection still reports the outer wrapper.
