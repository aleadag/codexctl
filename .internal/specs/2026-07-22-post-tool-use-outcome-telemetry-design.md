# PostToolUse Outcome Telemetry Design

> Date: 2026-07-22
> Issue: `codexctl-i0y`
> Research: `.internal/research/2026-07-22-codex-post-tool-use-outcome-path.md`
> Status: Approved design

## Context

Current Codex sends `tool_use_id` on `PreToolUse` and `PostToolUse`, but its documented `PermissionRequest` payload omits that field. Coding Brain copies the optional permission-side ID into Decision activity and later requires exact `(session_id, turn_id, tool_use_id)` equality to attach a PostToolUse Outcome. Production Decision rows therefore have no tool ID and never match, while the existing integration test supplies an ID that current Codex does not send.

The failure is silent. A PostToolUse event that cannot satisfy the exact predicate produces neither an Outcome nor a lifecycle observation, so Doctor can report the hook configuration as healthy while Live repeats `execution not confirmed` on ordinary delivered rows.

## Goals

- Attribute current Codex Bash and unified-exec PostToolUse events when there is one unambiguous Decision.
- Preserve exact stable-ID attribution whenever both sides provide it.
- Make PostToolUse receipt and zero-coverage failures observable.
- Keep missing confirmation visible only when it changes an operator decision.
- Preserve distinct succeeded, failed, and cancelled Outcome rendering.
- Represent opaque PostToolUse completion without claiming command success.
- Avoid storing new raw command or tool-response data.

## Non-goals

- Do not invent a general correlation scheme for non-Bash tools that lack a permission-side ID.
- Do not infer exit status from arbitrary string-shaped unified-exec `tool_response` values.
- Do not change permission decisions, hook trust, or tool execution behavior.
- Do not migrate historical activity rows or backfill historical Outcomes.
- Do not refactor the legacy pending-outcome spool, which is not the active managed hook path.

## Design

### PostToolUse recording

`run_with_activity` will treat PostToolUse as both lifecycle evidence and a possible Decision outcome:

1. Parse and record the lifecycle event as today.
2. Under the activity store's existing exclusive lock, read the activity log once for both observation and correlation work.
3. Append a lightweight `ActivityKind::Lifecycle` observation with `tool: "PostToolUse"` and the hook identity first.
4. Append a Decision Outcome only when the locked snapshot lacks equivalent evidence; correlation failure or ambiguity must not suppress the observation while the store remains writable.
5. Compact the activity store as today.

The lifecycle observation contains IDs and project/session evidence already permitted by the activity schema. It will not persist `tool_input`, `tool_response`, raw command output, command hashes, or fingerprints. Lifecycle rows remain excluded from Live's Decision projection.

### Outcome correlation

Correlation uses two ordered strategies.

#### 1. Exact stable-ID match

The primary strategy remains exact equality on:

- `session_id`
- `turn_id`
- `tool_use_id`

The candidate must be a Decision whose first terminal state is Allowed and which has a `decision_id`. Delivered evidence is not required because PostToolUse can legitimately confirm execution after delivery remained Unknown. Denied, Abstained, and Error decisions are never eligible. If the activity already has the same completed Outcome evidence, the repeated PostToolUse is a no-op; otherwise the Decision remains eligible for a new Outcome. This preserves current behavior for any producer that supplies IDs on both events and makes repeated PostToolUse delivery idempotent.

#### 2. Unique Bash command fallback

If no exact match exists, fallback is allowed only when:

- `tool_name` is `Bash`;
- `tool_input.command` is present and non-empty;
- the command is transformed with the same bounded redaction used by PermissionRequest activity;
- normalization neither redacts nor truncates the command, because either transformation is lossy and cannot prove command equality;
- the PostToolUse `tool_use_id` identifies exactly one preceding PreToolUse observation in the same session and turn;
- the candidate Decision follows that PreToolUse and precedes the next same-turn PreToolUse observation;
- within that anchored interval, a Decision whose first terminal state is Allowed has the same session, turn, tool name, and normalized redacted command;
- that Decision has a `decision_id`; and
- exactly one candidate qualifies.

One candidate produces the Outcome, or is a no-op when that activity already has equivalent Outcome evidence. The locked snapshot/check/append operation makes this idempotent across concurrent hook processes. A missing or non-unique PreToolUse anchor, multiple matching Decisions in the anchored interval, or overlapping/interleaved PreToolUse intervals is ambiguous: Coding Brain emits one metadata-only diagnostic and writes no Outcome. Zero candidates remain non-actionable when the anchored interval contains no Decision, because most tool calls do not pass through PermissionRequest. If the interval contains a Decision but attribution still fails, Coding Brain records an attribution diagnostic instead of silently discarding the evidence.

No fallback will match only by time, latest row, session/turn, missing IDs, or command text outside the anchored interval. Those choices can attach parallel, repeated, or redaction-colliding calls to the wrong Decision.

### Outcome classification

`ActivityOutcome` gains a neutral `Completed` variant. An opaque string-shaped unified-exec `tool_response` proves only that PostToolUse arrived, so it maps to Completed rather than Succeeded. Structured responses continue to map explicit success, failure, and cancellation evidence to Succeeded, Failed, and Cancelled respectively; absent or unrecognized structured status also maps to Completed.

Completed is confirmation of tool completion, not a positive success signal. Downstream projections must not count it as success, use it to supersede a failed result, or feed it into success-based learning. The current unified-exec regression fixture uses the documented string-shaped response and asserts Completed.

### Activity schema compatibility

The activity schema advances from v1 to v2 for `ActivityOutcome::Completed` and the new lifecycle evidence contract:

- readers accept both v1 and v2 rows in one log;
- all newly appended rows use v2, including Outcomes correlated to v1 Decisions;
- v1 rows retain their original schema version during compaction and are never rewritten as v2;
- compaction preserves the schema version of every retained row;
- no historical migration or backfill is performed; and
- downgrade to a pre-v2 binary is unsupported after any v2 row has been written, because existing v1 compaction can discard unrecognized rows.

Mixed-version reads and compaction in the upgraded binary are part of the compatibility test suite. Operators who must preserve rollback capability need to back up the activity log before upgrading.

### Doctor telemetry check

Doctor gains one `outcome telemetry` check backed by the activity store. It examines bounded recent samples rather than allowing one historical success to hide a current regression. Retries are deduplicated by unique invocation key `(session_id, turn_id, tool_use_id)` before thresholds are evaluated:

- Consider the latest 100 unique tool invocation keys represented by PreToolUse and PostToolUse lifecycle observations.
- Select those keys by each invocation's newest lifecycle evidence, then collect Pre/Post flags only for the selected keys so event ordering cannot hide an older matching PreToolUse.
- Report Skipped when fewer than 10 unique PreToolUse invocations exist.
- Report Advisory when at least 10 unique PreToolUse invocations exist but no PostToolUse observation exists in that window.
- Separately inspect the latest 20 unique eligible allowed-and-delivered Decisions, ranked by delivery timestamp rather than a later Outcome timestamp.
- Report Skipped for attribution coverage until at least 5 eligible Decisions exist.
- Report Advisory when PostToolUse evidence exists but none of those eligible Decisions has an Outcome.
- Otherwise report Pass with compact observed counts.

The check intentionally detects zero coverage, not a success ratio. Exact 9/10 and 4/5 boundary behavior is covered by tests.

Activity-store read failures are Advisory with the existing ownership/permissions guidance. The check is diagnostic and does not change Doctor's exit code.

The first advisory explains that configuration is present but runtime PostToolUse evidence is absent and suggests upgrading/restarting Codex, reviewing `/hooks`, and running completed local tools. The second explains that PostToolUse is arriving but Decision attribution is unavailable, which distinguishes hook delivery from Coding Brain correlation.

### Live wording

`activity_status` keeps Outcome text as the highest-priority status:

- `allowed · outcome confirmed: succeeded`
- `allowed · outcome confirmed: failed`
- `allowed · outcome confirmed: cancelled`
- `allowed · outcome confirmed: completed`

Without an Outcome:

- Delivered becomes `allowed · response delivered`.
- Failed delivery retains `delivery failed · execution not confirmed`.
- Unknown delivery retains `delivery unknown · execution not confirmed`.
- Delivered denials remain `blocked · command did not execute`.

This removes universal boilerplate without implying that response delivery proves execution.

## Error Handling and Safety

- Exact IDs always take precedence over content-based correlation.
- Ambiguity fails closed for attribution: no guessed Outcome is written.
- Raw `tool_input.command` exists only transiently in hook memory. The fallback compares the same representation already stored by PermissionRequest only when normalization is lossless; it does not persist raw commands, raw PostToolUse responses, hashes, or fingerprints.
- Commands whose normalized comparison was redacted or truncated are ineligible for content fallback; they require exact stable IDs.
- Hook identifiers are bounded before comparison with their persisted normalized forms.
- PostToolUse observations exclude tool input and response data.
- Untrusted hook IDs and labels pass through existing `ActivityEvent::normalized` bounds before persistence, and diagnostics contain metadata only.
- An unmatched PostToolUse for a tool without a Decision is normal and does not create attention noise.
- Correlation changes occur after tool execution and cannot weaken permission enforcement.

### Scale and downgrade boundary

Each PostToolUse performs one shared activity-log read under the existing exclusive lock, then uses bounded reverse scans for the exact-ID and anchored fallback searches. Observation is written before optional Outcome evidence, and compaction runs only after the atomic operation releases its lock. Lock/read/write failures emit bounded diagnostics and preserve fail-open hook protocol behavior; they cannot guarantee durable observation. No index or timing-sensitive performance contract is added. A large-log fixture verifies that correlation remains correct near the tail.

There is no feature flag and no destructive migration. The upgraded compactor preserves retained rows at their original schema version. Downgrade after v2 writes is explicitly unsupported: the current v1 reader treats v2 rows as malformed, and its compactor can omit them when rewriting the log.

## Testing

### Lifecycle hook

- Preserve the exact-ID unit test.
- Add exact and fallback negative tests proving Denied, Abstained, and Error decisions never receive Outcomes.
- Add a unique Bash fallback test where the Decision has no `tool_use_id` and PostToolUse has one.
- Require the fallback to locate the matching PreToolUse anchor and remain inside its interval.
- Assert PostToolUse creates both lifecycle evidence and the matched Outcome.
- Add interleaved parallel PreToolUse and repeated-identical-command tests; assert ambiguity produces a diagnostic and no Outcome.
- Add redaction-colliding command tests; assert no Outcome is guessed.
- Add an idempotency test for duplicate PostToolUse delivery.
- Add truncation-colliding long-command, oversized-identifier, and concurrent duplicate-delivery tests; assert two concurrent observations but exactly one Outcome.
- Exercise a current unified-exec payload with `tool_name: "Bash"`, `tool_input.command`, `tool_use_id`, and string `tool_response`.
- Keep a no-Decision tool call non-diagnostic while retaining its PostToolUse observation.
- Use a large activity log to verify the reverse scan selects only the anchored tail candidate; do not assert wall-clock timing.

### End-to-end hook activity

- Stop adding `tool_use_id` to the PermissionRequest payload helper.
- Run PermissionRequest through delivery, then the current unified-exec PostToolUse fixture.
- Assert the projected activity has a neutral Completed confirmation and the original Decision identity.

### Doctor

- Insufficient activity is Skipped.
- Sustained PreToolUse with zero PostToolUse is Advisory.
- PostToolUse with eligible delivered Decisions but zero Outcomes is Advisory.
- At least one attributed Outcome produces Pass.
- Verify the 9/10 PreToolUse and 4/5 Decision threshold boundaries.
- Verify retry deduplication and recent-window expiry.
- Verify Post-before-Pre reverse traversal and delayed Outcomes cannot reorder either recent window.
- Activity-store read failure remains non-fatal and actionable.

### TUI

- Delivered activity without Outcome omits `execution not confirmed`.
- Unknown and failed delivery retain it.
- Completed, succeeded, failed, and cancelled Outcomes render distinctly.

### Schema compatibility

- Read v1-only, v2-only, and mixed v1/v2 logs.
- Preserve each retained row's schema version through compaction.
- Assert compaction never upgrades v1 rows implicitly.
- Verify opaque responses map to Completed and structured responses map to succeeded, failed, and cancelled.

## Documentation Impact

The user-visible changes are self-explanatory in Live and Doctor output. No configuration or command syntax changes. Release documentation should mention the corrected outcome telemetry and new Doctor advisory if the repository's release process requires it.

## Success Criteria

- A current Codex unified-exec PostToolUse payload can attribute a delivered Bash Decision whose PermissionRequest omitted `tool_use_id`.
- Fallback attribution is constrained to the unique matching PreToolUse interval.
- Ambiguous payloads never attach an Outcome.
- Doctor distinguishes absent PostToolUse evidence from failed outcome attribution.
- Ordinary delivered Live rows no longer repeat `execution not confirmed`.
- Opaque unified-exec completion is neutral; existing structured Outcome states remain distinct.
- Mixed v1/v2 activity logs remain readable and compact without rewriting v1 rows.
- Permission behavior remains unchanged and no raw hook payload content is newly persisted.

## Stress Test Results

### Resolved Decisions

1. Fallback correlation is anchored to a unique PreToolUse ID and bounded by the next same-turn PreToolUse; ambiguity produces no Outcome.
2. PostToolUse observation and Outcome attribution are independent, and Doctor counts unique invocation keys.
3. Doctor uses bounded zero-coverage thresholds: 10 of 100 tool invocations and 5 of 20 eligible delivered Decisions.
4. Opaque responses map to neutral Completed; only structured evidence maps to succeeded, failed, or cancelled.
5. Activity schema v2 writes coexist with v1 reads, and compaction preserves row versions.
6. PostToolUse uses one shared log read and bounded reverse scans, with no new index.
7. Upgrade uses no feature flag or destructive migration; the upgraded compactor preserves v1 rows.
8. Adversarial coverage includes parallel/repeated/redaction-colliding commands, duplicate Post events, exact Doctor boundaries, retries, recent-window expiry, schema mixing, and every Outcome class.
9. Correlation keeps raw payload data transient, persists no new secrets or fingerprints, and fails closed without changing permission enforcement.
10. Downgrade after v2 writes is explicitly unsupported because existing v1 compaction can discard unrecognized rows.

### Changes Made

- Replaced command-only fallback with a PreToolUse-anchored interval algorithm.
- Added neutral Completed semantics and explicit downstream constraints.
- Made lifecycle observation independent from attribution success.
- Defined deduplicated Doctor windows and exact minimum-sample behavior.
- Added schema v2 compatibility, compaction, scale, downgrade-boundary, and adversarial-test requirements.
- Strengthened privacy requirements for hook inputs and diagnostics.
- Replaced the infeasible safe-downgrade promise with an explicit pre-upgrade backup boundary.

### Deferred / Parking Lot

- General correlation for non-Bash tools without permission-side stable IDs.
- Inferring exit status from opaque unified-exec response strings.

### Confidence

High. Self-review verified the existing v1 downgrade hazard and the design now states that boundary explicitly.
