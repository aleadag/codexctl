# Research: Antigravity 1.1.5 Hook Allow Acceptance

> **Date:** 2026-07-24
> **Bead:** codexctl-jx1x
> **Status:** Complete

## Summary

Antigravity CLI 1.1.5 invokes a valid `PreToolUse` command hook but ignores its
documented `{"decision":"allow"}` response and proceeds to the native Bash
confirmation. Coding Brain cannot safely force provider acceptance; it should
diagnose the affected provider version, document the upstream limitation, and
keep response emission distinct from confirmed execution.

## Key Findings

### Antigravity documents `allow` as an automatic approval

> **Confidence:** high — verified against the current official hook contract.

The official contract says hooks receive JSON on stdin and return JSON on
stdout. For `PreToolUse`, `decision` is required and `"allow"` automatically
allows execution; `"deny"`, `"ask"`, and `"force_ask"` preserve stricter
provider behavior. [S1]

The installed Antigravity CLI 1.1.5 binary also contains the corresponding
response schema with the `allow`, `deny`, `ask`, `force_ask`, and
`deny_unless_prior_grant` decision values. This rules out a simple response
field-name mismatch.

### Antigravity 1.1.5 ignores a minimal valid allow response

> **Confidence:** high — reproduced repeatedly with the installed real binary.

An isolated interactive reproduction used the installed `agy` 1.1.5 binary,
the existing authenticated app-data directory, and a temporary `hooks.json`
mounted over the live hook file. The hook:

1. consumed the complete stdin payload;
2. wrote a non-sensitive marker proving execution;
3. emitted newline-terminated `{"decision":"allow"}` on stdout;
4. emitted no stderr; and
5. exited with status 0.

Antigravity loaded the one-handler file, the marker proved the handler ran, and
the CLI immediately logged `Surfacing tool confirmation: "Bash"` for the safe
`df -h` command. No `PostToolUse` followed because the native prompt remained
pending. The same prompt occurred without a trailing newline, so framing is not
the cause.

### Coding Brain's deprecated-config warning is not causal

> **Confidence:** high — the clean-stderr reproduction fails identically.

The original incident included a harmless deprecated-config warning on hook
stderr. The isolated hook emitted no stderr and still produced the same native
prompt, proving that removing the warning would not restore provider
acceptance. Protocol-mode stderr should remain bounded and non-sensitive, but
warning suppression is not a fix for this issue.

### Delivery and provider acceptance are different evidence

> **Confidence:** high — established by the accepted project ADR and current
> activity flow.

Coding Brain records `Delivered` after successfully writing hook response bytes.
The accepted persistence design explicitly says `Allowed` and `Denied` are
committed Coding Brain decisions, while only later lifecycle or outcome evidence
may claim execution. [S2]

Most Live wording already follows that rule (`response delivered` and
`outcome confirmed`). One special case still renders a delivered deny as
`blocked · command did not execute` without provider outcome evidence. The
Antigravity 1.1.5 reproduction proves that claim is too strong.

## Comparisons

| Approach | Result | Safety |
| --- | --- | --- |
| Change JSON field names or add a newline | Real-binary reproduction still prompts | No benefit |
| Suppress hook stderr | Clean-stderr reproduction still prompts | No benefit |
| Enable Antigravity always-proceed | Hides the provider defect | Unacceptable permission weakening |
| Diagnose affected version and clarify evidence wording | Accurately reports the limitation pending upstream repair | Preserves native controls |

## Codebase Context

- `src/brain/permission_hook.rs` serializes the documented Antigravity shape and
  records `Delivered` only after stdout write and flush succeed.
- `src/provider_hooks/antigravity.rs` preserves provider `deny`, `ask`,
  `force_ask`, and permission-override constraints.
- `crates/coding-brain-tui/src/ui/brain/live.rs` generally distinguishes
  response delivery from outcome confirmation, except for the delivered-deny
  special case.
- `src/doctor.rs` validates Antigravity executable presence and hook definitions
  but does not report a known provider-version contract failure.

## Recommendations

1. Add an Antigravity hook-contract Doctor advisory when managed hooks are
   current and the detected CLI version is 1.1.5.
2. Keep the existing documented response schema and all fail-safe provider
   policy handling unchanged.
3. Change the delivered-deny Live wording to `denied · response delivered`;
   claim execution or blocking only when later outcome evidence exists.
4. Document the 1.1.5 limitation and the safe interpretation of `Delivered`.
5. Retain a real-binary reproduction recipe for validating a future
   Antigravity release before removing the advisory.

## Open Questions

- Which future Antigravity version will restore the documented `PreToolUse`
  decision behavior? Until validated, versions other than the confirmed 1.1.5
  should not be declared affected or fixed.
- Whether the upstream project wants the minimal reproduction filed as a public
  issue is an external publication decision and is outside this implementation
  without explicit authorization.

## Refuted / Discarded Claims

- Discarded: stderr warnings cause Antigravity to reject otherwise valid hook
  output. A clean-stderr hook failed identically.
- Discarded: Antigravity requires newline-delimited JSON. Both framed and
  unframed valid JSON were ignored.
- Discarded: Coding Brain should bypass native permissions as a workaround.
  That would weaken the security boundary and violate the issue constraints.

## Sources

- [Antigravity Hooks](https://antigravity.google/docs/hooks) — Primary/Official
  — retrieved 2026-07-24 — stdin/stdout contract and `PreToolUse` decisions.
- [Antigravity CLI Permissions](https://antigravity.google/docs/cli/permissions)
  — Primary/Official — retrieved 2026-07-24 — native allow, ask, and deny
  behavior.
- [Antigravity CLI repository](https://github.com/google-antigravity/antigravity-cli)
  — Primary/Official — retrieved 2026-07-24 — provider release and issue
  context.
- [Coding Brain repository](https://github.com/aleadag/coding-brain) —
  Primary/Project — inspected 2026-07-24 — response serialization, activity
  evidence, Doctor, and Live wording.

[S1]: https://antigravity.google/docs/hooks
[S2]: https://github.com/aleadag/coding-brain/blob/main/docs/decisions/ADR-0003-fail-safe-hook-and-learning-persistence.md
