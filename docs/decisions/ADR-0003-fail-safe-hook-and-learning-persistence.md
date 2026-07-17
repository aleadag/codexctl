# ADR-0003: Make Hook and Learning Persistence Fail-Safe

- Status: Accepted
- Date: 2026-07-17
- Bead: `codexctl-0cy.1.1`

## Context

Coding Brain evaluates permission requests in short-lived hook processes. The
TUI may be closed, several hooks may append concurrently, and a process may die
at any write boundary. The implementation also has two records with different
purposes: `decisions.jsonl` retains model proposals and learning evidence, while
`activity.jsonl` drives Live, Review, Scorecard, and lifecycle audit.

Those writes cannot form one filesystem transaction with the hook response
pipe. For example, Coding Brain can persist an allow decision and then fail to
write stdout, or it can write stdout and die before recording that delivery.
Calling either case “executed” would overstate the evidence and could train the
Brain on an action Codex never ran.

The same partial-publication problem exists in learning maintenance. A
distillation run produces global and per-project preference files. Replacing
them in place can expose a mixed generation after a crash. An interrupted JSONL
append can also leave a partial tail that corrupts the next otherwise valid
event.

The implementation plan and its eleven-branch stress test are recorded in the
[Coding Brain implementation plan](../../.internal/plans/2026-07-17-coding-brain-product-boundary.md).

## Decision

### Separate proposal, commitment, delivery, and execution

`decisions.jsonl` stores model proposals and learning evidence.
`activity.jsonl` is the authoritative decision-commit and lifecycle audit. Both
records use the same stable `decision_id` and `activity_id` correlation.

For a model-derived allow or deny, the hook must:

1. persist the decision proposal;
2. persist the terminal `Allowed` or `Denied` activity referencing that
   proposal;
3. write the serialized response to stdout;
4. append `Delivered` or `DeliveryFailed` best-effort.

Failure of either required append before stdout causes the model decision to
abstain. A proposal without terminal activity is non-executed and excluded from
learning projections. Deterministic code-owned denies still run before
inference and fail closed even when neither audit store is writable.

`Allowed` and `Denied` mean that Coding Brain committed the hook decision. They
do not prove Codex received it or ran the tool. A committed decision without a
delivery event projects as `DeliveryUnknown`; a failed stdout write projects as
`DeliveryFailed`. Only later lifecycle or outcome evidence may claim that the
tool executed.

### Repair append-only state and publish learning atomically

Activity append and compaction use the same exclusive lock. Before accepting a
new append, the writer inspects bytes after the final newline. It completes a
valid unterminated JSON value, or truncates an invalid fragment to the last
complete newline and records only the discarded byte count. It never copies the
raw fragment into a diagnostic. Readers continue past malformed complete lines
and report bounded offsets.

Distillation writes a complete immutable preference generation under a new
generation ID. It flushes every global and project file before atomically
replacing the watermark/current-generation pointer. Readers use only the named
generation and never write preferences on demand. A crash before the pointer
swap leaves the previous generation active; later maintenance removes abandoned
generations while retaining the current and previous published generations.

A valid tracked UUID in `.coding-brain/project.toml` is authoritative across
clones, worktrees, and forks. Coding Brain does not infer identity from paths,
names, or Git remotes. A user who wants independent learning removes the
manifest and reruns `coding-brain init`.

### Keep external I/O explicit and bounded

The user may select any model endpoint through CLI or user configuration.
Project configuration cannot redirect it. Coding Brain redacts and bounds
model-bound context, sends curl request bodies over stdin rather than argv,
disables redirects, caps response bytes, and shows stronger warnings for
plaintext non-loopback HTTP. Those warnings do not override the user's endpoint
choice.

Confirmed purge accepts only absolute non-root bases and fixed child targets.
It previews and revalidates each file type immediately before deletion, rejects
changed targets, and unlinks symlinks without following them. Project config and
identity files are never purge targets.

## Rationale

The ordering makes safety depend on evidence Coding Brain can actually
guarantee. A model action cannot leave the process until its proposal and
committed decision are durable, while deterministic denies remain effective
during storage failure. Separate delivery and outcome states avoid inventing a
transaction across JSONL files and a pipe.

Immutable preference generations apply the same principle to learning: publish
one complete snapshot with one atomic pointer instead of coordinating many
in-place renames. Tail repair keeps the JSONL format simple while ensuring that
one killed writer does not consume the next valid event.

Stable UUID authority is intentionally explicit. Automatically comparing paths
or remotes would split worktrees and clones unpredictably, while a manual
manifest reset makes the user's intent reviewable in the repository.

## Consequences

- Activity projections and operator copy must distinguish committed,
  delivered, delivery-failed, delivery-unknown, and outcome-confirmed states.
- Preference distillation must join proposal records with authoritative
  activity and exclude unpaired proposals from learning.
- Hook tests must inject failures at both audit writes, stdout, delivery append,
  JSONL tail repair, and each preference-generation publication boundary.
- Distillation keeps at least two published generations, using more state in
  exchange for safe rollback after a failed publication.
- Remote endpoints remain usable by explicit choice, but prompts and responses
  are bounded and visible transport warnings remain part of the product UI.
- Purge and project-identity reset remain explicit operator actions; normal
  startup does not migrate, rewrite, or delete legacy data.
