# Research: Dream Project Memory Storage

> **Date:** 2026-07-17
> **Bead:** `codexctl-0cy.11`
> **Status:** Complete

## Summary

Dream should keep provenance-rich memories in a Coding Brain-owned structured
store, then generate a project Markdown view automatically. Beads should be an
optional publication adapter for a small set of active memories, not the only
copy or a runtime dependency.

## Key Findings

### Beads is useful as an injection target, not a reflective-memory ledger

> **Confidence:** high — verified against the pinned Beads implementation and
> its official README.

`bd remember` derives or accepts a key, namespaces it as a memory key, and
writes the insight through the generic configuration store. Reusing a key
updates its value in place [S1]. The memory record therefore has no first-class
fields for source activity IDs, evidence, confidence, model, scope, or
supersession; Coding Brain would have to encode all of that into one string.

Beads deliberately injects stored project memories through `bd prime`, which
makes it a good delivery mechanism for a bounded set of current preferences
[S4]. It is a poor fit for an automatically growing reflection history.

The sync behavior reinforces that distinction. When the same memory key is
edited on two clones, Beads resolves the config conflict with the remote value
and reports that the local edit was superseded [S2]. That is sensible for a
convergent projection, but it must not be the only surviving copy of Dream's
evidence and history.

### Markdown is the strongest portable review surface

> **Confidence:** high — GitHub documents source and rendered review views for
> Markdown in commits and pull requests.

A project Markdown file is readable without Coding Brain or Beads and can be
reviewed through ordinary repository diffs. GitHub provides both source and
rendered views for prose documents in commits and pull requests [S3]. This fits
the user's preference for automatic promotion: Dream can update the file
without blocking on approval, while the resulting change remains visible and
reversible through version control.

A single hand-maintained Markdown file should not be the canonical database,
however. Automatic reflection needs stable IDs, evidence references,
confidence, scope, model identity, and append-only supersession or retraction.
Those fields already fit Coding Brain's structured persistence pattern better
than repeated in-place prose edits.

### Agent discovery and Brain ownership are separate concerns

> **Confidence:** high for discovery, medium for cross-agent consumption — only
> `AGENTS.md` discovery is standardized for Codex.

Codex discovers `AGENTS.md` before work [S5]. An arbitrary Coding Brain memory
file is not part of that documented discovery chain. Coding Brain can always
load its own projection; other agents need a one-time project instruction that
points them to it, or an adapter such as Beads that injects the current summary.

The current repository already follows this separation:

- structured global and per-project preferences live under
  `~/.codexctl/brain/preferences/`;
- prompt construction loads those preferences directly;
- `garden` projects high-confidence preferences into a managed `AGENTS.md`
  block only when explicitly applied.

Dream can extend that pipeline without turning Beads or a tracked repository
file into the internal learning database.

## Comparisons

| Criterion | Beads canonical | Markdown canonical | Brain store + projections |
|-----------|-----------------|--------------------|---------------------------|
| Works without optional tools | No | Yes | Yes |
| Provenance and supersession | Encoded inside one value | Possible but awkward in one file | Native structured fields |
| Automatic agent injection | Strong for Beads-enabled agents | Requires discovery instruction | Brain-native; adapters optional |
| Human review | `bd`/Dolt tooling | Excellent repository diff | Generated Markdown provides it |
| Concurrent automatic writes | Store-dependent; same-key remote wins | One file can conflict | Append records, regenerate view |
| Existing code fit | New hard dependency | Replaces current JSON persistence | Extends current persistence |
| Recommended role | Optional bounded projection | Portable generated projection | Canonical source of truth |

## Disagreements

One research path recommended one append-only Markdown file per memory as the
canonical store. That layout improves Git merge isolation, but it would make
every automatic reflection a project working-tree mutation and duplicate the
structured persistence Coding Brain already owns. The better fit here is an
append-only structured ledger under Brain state plus one deterministic Markdown
projection; projects can choose to track that projection.

## Codebase Context

- `src/brain/decisions.rs:257-283` fixes production Brain state under
  `~/.codexctl/brain` and appends resolved decisions to `decisions.jsonl`.
- `src/brain/preferences.rs:109-156` defines structured learned patterns,
  confidence, sample counts, conditions, accuracy, and temporal patterns.
- `src/brain/pref_store.rs:15-64` writes global and per-project preference JSON.
- `src/brain/query.rs:77-85` loads project preferences and retrieved examples
  into Brain inference.
- `src/brain/garden.rs:3-29` describes Markdown promotion from private
  preferences, and `src/brain/garden.rs:202-244` gates mutation behind apply.
- `src/brain/garden.rs:210-220` already requires at least 20 samples and 90%
  confidence before proposing an `AGENTS.md` addition.

Project identity is currently based on a sanitized display string. Dream needs
a stable repository identity before project memory can be trusted across
renames or two repositories with the same display name.

## Recommendations

1. Add an append-only Brain-owned Dream ledger under Coding Brain's XDG state
   root, for example
   `$XDG_STATE_HOME/coding-brain/memory/<project-id>.jsonl`, falling back to
   `~/.local/state/coding-brain/memory/<project-id>.jsonl`.
2. Give every record a stable ID, kind, project and path scope, confidence,
   evidence references, model identity, creation time, and an operation such as
   `assert`, `supersede`, or `retract`.
3. Let Dream activate high-confidence records automatically. Explicit project
   instructions and deterministic rules always take precedence; lower-confidence
   reflections remain inactive evidence.
4. Generate `.coding-brain/MEMORY.md` as the concise current projection. Do not
   auto-commit it. A project can track it, ignore it, or add a one-time
   `AGENTS.md` instruction telling agents to read it.
5. When explicitly enabled and Beads is available and writable, publish a
   bounded set of current memories using stable `bd remember --key` keys.
   Treat those entries as rebuildable views. Adapter failure never blocks Dream,
   and Coding Brain never runs `bd dolt push` or `bd dolt pull` automatically.
6. Replace the current Garden implementation with, or later route it through,
   the same projection logic rather than maintaining two independent promotion
   systems.

## Recommended Beads

Do not create an implementation bead yet. Dream is a future product direction;
the current Brain-primary TUI migration should only preserve the extension seam
and document this storage decision.

## Open Questions

- Should `.coding-brain/MEMORY.md` be tracked by default or generated into an
  ignored project-local directory?
- What stable project identity should survive checkout path and display-name
  changes?
- Which confidence and evidence thresholds promote a reflection from inactive
  evidence to active memory?

## Refuted / Discarded Claims

- A pointer in `AGENTS.md` guarantees that Codex will load an arbitrary memory
  file. The official source guarantees `AGENTS.md` discovery, but not automatic
  traversal of files it mentions. Treat the pointer as an instruction, not a
  file-include mechanism.
- Beads memory schema alone proves how `bd remember` stores entries. The schema
  only proves the generic config table shape; `memory.go` was needed to verify
  that memories use that store.

Independent citation checks supported [S1], [S2], and [S3]. The first phrasing
of the two discarded claims above was inconclusive and was narrowed rather than
kept. [S4] and [S5] were checked directly against their official sources.

## Sources

- [Beads memory command](https://github.com/gastownhall/beads/blob/44e278e5311291874e2b9f0baeb4a475a076d5a2/cmd/bd/memory.go) — Primary/Official — pinned 2026-07-17 — key/value writes and same-key updates.
- [Beads merge settlement](https://github.com/gastownhall/beads/blob/44e278e5311291874e2b9f0baeb4a475a076d5a2/internal/storage/versioncontrolops/mergesettle.go) — Primary/Official — pinned 2026-07-17 — remote-wins memory conflict behavior.
- [GitHub: Working with non-code files](https://docs.github.com/en/enterprise-server%403.20/repositories/working-with-files/using-files/working-with-non-code-files) — Primary/Official — accessed 2026-07-17 — source and rendered Markdown review.
- [Beads README](https://github.com/gastownhall/beads/blob/44e278e5311291874e2b9f0baeb4a475a076d5a2/README.md) — Primary/Official — pinned 2026-07-17 — `bd prime` memory injection, Dolt sync, and contributor routing.
- [OpenAI: Custom instructions with AGENTS.md](https://developers.openai.com/codex/guides/agents-md) — Primary/Official — accessed 2026-07-17 — documented Codex instruction discovery.
