# Benign Lifecycle Outcomes Design

## Context

`PostToolUse` is emitted for every completed tool call, while
`PermissionRequest` runs only when Codex asks the Brain for a permission
decision. `append_outcome` currently treats every `PostToolUse` without a
matching terminal decision as an orphan diagnostic. Ordinary auto-approved
tools therefore appear in Live as errors such as `Bash` and
`collaborationwait_agent`, even though their lifecycle identities are valid.

## Decision

Classify outcome joins from the activity already recorded for the exact
`session_id`, `turn_id`, and `tool_use_id`:

- If no Decision activity exists for the identity, return successfully without
  appending an outcome or diagnostic. The tool did not involve the Brain, so
  there is no Brain outcome to attribute.
- If a matching terminal Decision with a decision ID exists, append its outcome
  as today.
- If Decision activity exists for the identity but no attributable terminal
  decision exists, append the existing orphan diagnostic. This remains a real
  incomplete or inconsistent Brain lifecycle.

The existing generic lifecycle observation stays in the audit log and remains
excluded from activity snapshots. Matching remains independent of tool name,
so the rule covers Bash, collaboration tools, and future tool types equally.

## Safety and Scope

The change does not alter permission evaluation, hook installation, lifecycle
status projection, activity schemas, or persisted paths. It suppresses only a
false-positive diagnostic when the exact lifecycle identity has no Decision
activity. It does not guess an outcome or attach one to a different decision.

## Testing

- A `PostToolUse` with only a matching lifecycle observation produces no
  diagnostic and no stderr output.
- A matching terminal Decision still receives its succeeded or failed outcome.
- Matching nonterminal Decision activity still produces an orphan diagnostic.
- The focused lifecycle-hook tests and full repository quality gates pass.
