# Antigravity 1.1.5 Hook Contract Mitigation Design

> **Date:** 2026-07-24
> **Issue:** codexctl-0a29
> **Brainstorming:** codexctl-x2fy
> **Research:** `.internal/research/2026-07-24-antigravity-1-1-5-hook-allow-acceptance.md`

## Problem

Antigravity CLI 1.1.5 invokes Coding Brain's `PreToolUse` handler but ignores a
valid, clean `{"decision":"allow"}` response and surfaces its native Bash
confirmation. Coding Brain cannot repair provider-side acceptance without
bypassing Antigravity's permission boundary.

The current response schema and fail-safe parsing are correct. However, Doctor
does not identify the affected provider version, and Live has one delivered-deny
special case that claims the command did not execute before outcome evidence
exists.

## Chosen Approach

Implement an evidence-preserving local mitigation:

1. Add an Antigravity hook-contract Doctor row. When current managed
   Antigravity hooks and `agy` version 1.1.5 are both detected, report an
   advisory that this version may ignore `PreToolUse` decisions and retain the
   native prompt. Other detected versions remain non-affected unless separately
   validated; an unavailable or unparseable version produces no speculative
   compatibility claim.
2. Render successful stdout writes as `response emitted` for both allow and
   deny. Remove the delivered-deny claim
   `blocked · command did not execute`. Confirmed `PostToolUse` outcome remains
   the only execution claim.
3. Add troubleshooting guidance with the confirmed 1.1.5 behavior, safe
   interpretation of response delivery, and upgrade/revalidation advice.
4. Preserve the existing Antigravity response JSON, provider policy parsing,
   lifecycle correlation, and native permissions.

No automatic terminal input is added. No public upstream issue is filed in this
change.

## Alternatives Rejected

### Automatic terminal key fallback

Rejected because terminal input can race with prompt changes and approve the
wrong request. A future implementation would need exact provider/session/step/
command correlation, immediate pane recapture, guarded input, and subsequent
`PostToolUse` confirmation. Those requirements are materially larger than this
provider-version mitigation.

### Return `ask` for every Antigravity approval

Rejected because it disables a documented capability for versions that may work
and does not improve the already-native prompt behavior of 1.1.5. Coding Brain
should keep recording its committed decision while describing stdout success as
response delivery, not provider acceptance.

### Change response fields or suppress stderr

Rejected by the real-binary reproduction: the documented response shape,
newline framing, exit status zero, and clean stderr still produce the native
prompt.

## Components

### Doctor

The provider setup path remains responsible for hook-definition integrity.
A separate compatibility check keeps version-specific provider behavior from
being conflated with installation correctness.

The version probe runs `agy --version` with a short timeout and bounded stdout
and stderr. Only a successful exit with a strict simple semantic version token
reaches the pure compatibility classifier. Probe failure, timeout, oversized
output, non-UTF-8 output, or malformed output skips the compatibility claim
without affecting existing setup checks. The affected predicate is exact
version `1.1.5`; future versions are not called fixed until the real-binary
reproduction validates them. Doctor runs at most one uncached probe per
invocation, and only after it confirms current managed Antigravity hooks, so a
provider upgrade is visible on the next run without adding work to unrelated
setups.

### Live

`ActivityState::{Allowed,Denied}` continues to mean Coding Brain committed a
decision. `DeliveryState::Delivered` continues to mean response bytes were
written successfully and is rendered as `response emitted`. Only
`ActivityOutcome` adds `outcome confirmed` copy. Removing the delivered-deny
special case makes deny use the same evidence rule as allow.

### Troubleshooting

The native-prompt section explains that `agy` 1.1.5 is a confirmed upstream
contract failure, not evidence that Coding Brain failed to evaluate or write its
response. It directs users to keep the native prompt and upgrade/revalidate
rather than enabling always-proceed or terminal injection.

## Error Handling and Security

- Missing, failed, non-UTF-8, or malformed version output cannot disable hooks,
  permissions, or Doctor's existing setup checks.
- The compatibility advisory never changes hook output or provider policy.
- No terminal keystrokes are sent automatically.
- `deny`, `ask`, `force_ask`, permission overrides, malformed inputs, and hook
  failures retain their current fail-safe behavior.
- Tests and documentation contain no captured hook payloads, conversation IDs,
  account identity, or model content.

## Testing

1. Add failing Doctor unit tests for the bounded version probe, exact affected
   version, other versions, malformed output, and absent/current hook
   conditions.
2. Update Live regressions so delivered allow and deny report response emission
   and do not claim provider acceptance or blocked execution.
3. Retain existing Antigravity permission-contract tests unchanged to prove no
   policy regression.
4. Run focused Doctor, TUI, and hook activity tests, followed by workspace test,
   clippy, and formatting gates.

## Success Criteria

- `coding-brain doctor` visibly identifies current managed Antigravity hooks
  running against confirmed-affected `agy` 1.1.5.
- Live never infers provider acceptance or non-execution from a successful
  stdout write alone.
- Troubleshooting explains the upstream limitation and safe operator action.
- No automatic terminal input or native permission weakening is introduced.
- Focused and full workspace quality gates pass.

## Stress Test Results: Antigravity 1.1.5 Hook Contract Mitigation

### Resolved Decisions

- Version probing is bounded by time and output size; failure cannot block
  Doctor or alter existing setup results.
- Only exact version 1.1.5 is classified as confirmed affected. Other versions
  remain unverified until the real-binary reproduction is rerun.
- The compatibility advisory appears only when the affected executable and
  current managed Antigravity hooks are both present.
- Successful stdout writes are rendered as `response emitted` for allow and
  deny; only later outcome evidence claims execution.
- The native prompt remains authoritative when the provider ignores a
  decision. Coding Brain sends no automatic terminal input.
- CI covers the bounded probe, advisory classifier, protocol JSON, and evidence
  wording. The authenticated interactive provider reproduction remains a
  documented manual validation rather than a fake acceptance test.
- The exact 1.1.5 advisory does not expire automatically. It tells users to
  upgrade and revalidate without declaring an untested later version fixed.
- Doctor performs one conditional uncached probe per invocation, preserving
  fresh version evidence with bounded runtime cost.

### Changes Made

- Added explicit timeout and output bounds to the version-probe design.
- Expanded the Live wording correction from delivered deny only to every
  successfully emitted allow or deny response.

### Deferred / Parking Lot

- Public upstream issue filing requires separate authorization.
- Automatic terminal fallback remains out of scope unless a future design can
  prove exact prompt correlation, immediate revalidation, and outcome
  confirmation without weakening permissions.

### Confidence Assessment

- Overall: High
- Areas of concern: Antigravity's first fixed version is unknown and must be
  validated against the real interactive provider before documentation changes.
