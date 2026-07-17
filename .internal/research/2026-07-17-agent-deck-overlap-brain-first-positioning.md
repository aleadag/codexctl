# Research: Agent Deck overlap and BRAIN-first codexctl positioning

> **Date:** 2026-07-17
> **Bead:** codexctl-a1i
> **Status:** Complete

## Summary

Agent Deck already owns most of the broad “mission control” surface and now overlaps the proposed scheduler/runner-babysitter direction as well. codexctl should return to a BRAIN-first boundary: keep the thin observation and action substrate the BRAIN needs, but stop investing in generic session management or durable runner orchestration.

The recommended product is a local, auditable decision layer for Codex, not a second fleet manager and not yet a standalone BRAIN service.

## Key Findings

### Agent Deck covers the commodity session-management surface

> **Confidence:** high — the claim is supported by the official v1.9.73 README and independently re-fetched for citation soundness.

Agent Deck describes itself as mission control and combines session status and switching, groups and search, forks, Git worktrees, cost tracking, and conductor-based fleet management in one product [S1]. Its official docs also cover lifecycle operations, MCP and skills management, Docker isolation, web access, remote instances, and multi-tool support including Codex [S1].

This makes a feature-parity roadmap for codexctl expensive and weakly differentiated. The overlapping codexctl surface includes live discovery, filtering/search, launch/resume, kill, terminal input, recording, compaction, budget alerts, and terminal switching (`crates/codexctl-tui/src/app.rs:702-856`, `1744-1865`, `2232-2457`, `2581-2684`).

### Agent Deck also overlaps scheduler and babysitter work

> **Confidence:** high — the conductor claim is supported by official versioned documentation and independently re-fetched for citation soundness.

An Agent Deck conductor is a persistent Claude or Codex session that supervises other sessions, applies policy to routine responses, escalates uncertainty, and accumulates patterns in `LEARNINGS.md` [S2]. Agent Deck also ships parent-linked child launches, completion delivery, heartbeats, status-transition notifications, worktree isolation, and a watchdog for restarting critical sessions and nudging stuck children [S2][S3].

That substantially overlaps the open P1 `codexctl-dk3` direction: eligibility, claiming, workspace setup, process lifecycle, heartbeat, timeout, retry, recovery, verification, and reporting. Agent Deck does not prove every `dk3` guarantee, but it has already occupied the user-facing runner-supervision category.

### codexctl's strongest asset is its structured decision-feedback loop

> **Confidence:** high — verified directly against the current checkout, public README, implementation, and tests.

codexctl combines deterministic rule precedence with local-model judgment, operator corrections, optional guarded execution, and persistent decision/outcome/review state [S4]. The implementation goes materially beyond a conductor prompt:

- native `PermissionRequest` allow/deny with safe fallthrough and persisted audit state (`src/brain/permission_hook.rs:21-321`);
- outcome-aware decision records and periodic global/project preference distillation (`src/brain/decisions.rs:42-241`, `553-605`);
- adaptive thresholds and outcome-weighted preferences (`src/brain/preferences.rs:140-156`, `566-817`);
- ranked few-shot retrieval and canonical examples (`src/brain/retrieval.rs:9-150`, `src/brain/review.rs:28-90`);
- calibration, risk-tier accuracy, false approvals, counterfactuals, and a scorecard (`src/brain/metrics.rs:1811-2185`, `2273-2423`);
- transcript autopsy and focused Brain Review (`src/brain/autopsy.rs:272-875`, `src/brain/review.rs:1-90`).

Agent Deck has meaningful intelligence through conductor policy, `LEARNINGS.md`, transcript self-improvement, and deterministic operational safeguards [S2][S5]. The narrower differentiation is therefore not “uses an LLM”; it is auditable hook-level adjudication, calibrated outcomes, counterfactual review, and operator-specific learning.

### BRAIN-first does not mean deleting all session infrastructure

> **Confidence:** high — the dependency boundary is explicit in the approved architecture and current crate wiring.

The BRAIN still requires sensing and actuation: Codex hooks, transcript discovery, normalized session state, health signals, deterministic rules, and small terminal/action adapters. The current architecture keeps BRAIN code in the root binary while core owns discovery/runtime contracts and the TUI owns the dashboard [S6]. A literal standalone-service extraction would require a stable event/action protocol and ownership changes; there is no second concrete consumer yet.

The smallest coherent scope is therefore a BRAIN-first companion with a thin diagnostic/review cockpit. Keep discovery and immediate actions as infrastructure; stop expanding them as a general session manager.

## Comparisons

| Criterion | Full manager + BRAIN | BRAIN-first companion | Headless BRAIN service |
|-----------|----------------------|-----------------------|------------------------|
| Differentiation | Low to medium; competes with Agent Deck breadth | Highest near-term; concentrates on decision quality and learning | Potentially high, but hypothetical |
| User value | One-tool convenience | Works with Agent Deck, Kitty, tmux, or plain Codex | Best for integrators, weakest direct UX |
| Scope cost | High and continuously expanding | Bounded: observe, decide, learn, review, act | High protocol, daemon, auth, and migration cost |
| Migration risk | Low now, high opportunity cost | Medium; roadmap reversal must be explicit | High; current BRAIN is binary-coupled |
| Recommendation | Reject | Choose | Defer until a second consumer exists |

## Disagreements

The planning record contains two incompatible approved directions:

- `.internal/specs/2026-07-14-brain-only-architecture-design.md` makes the local BRAIN the product focus and removes durable coordination.
- open P1 `codexctl-dk3` says the user later superseded brain-only with a scheduler/runner babysitter.

The current Agent Deck evidence changes the competitive context for the later decision: both generic mission control and much of runner babysitting now have a strong adjacent implementation. If the user confirms BRAIN-first, `codexctl-dk3` must be explicitly superseded or frozen; it should not remain open as a contradictory P1.

## Codebase Context

The current shipped tree is already close to the recommended boundary:

- `README.md:3-71` describes a local-brain companion, immediate best-effort actions, learning/review, and external durable coordination.
- `docs/contributing.md:37-41` directs new work toward the dashboard/BRAIN/rules/learning/mailbox/terminal integration and keeps durable queues and worker coordination external.
- `.internal/specs/2026-07-14-brain-only-architecture-design.md:20-38` defines the exact retained and removed product scope.
- `src/main.rs:180-180` and `297-297` expose headless and single-decision seams.
- The dashboard is not yet thin: `crates/codexctl-tui/src/app.rs` still owns a broad manager UX and interleaves rules, BRAIN inference, and mailbox delivery.

Some BRAIN paths also need maturation before claiming a complete learning loop: briefing is printed rather than injected (`src/brain/briefing.rs:3-9`, `src/commands.rs:333-352`), outcome recording is not installed as the current `PostToolUse` hook (`src/init/hooks.rs:43-48`, `845-864`), and context-saturation restart has no production caller found outside its definition/tests (`src/brain/engine.rs:623-632`, `1669-1731`).

## Recommendations

1. Adopt this product promise: **codexctl is the local, auditable decision layer for Codex; it evaluates pending actions and session health, learns from corrections, and safely delivers recommendations or opt-in immediate actions.**
2. Freeze generic manager expansion: no groups, global conversation search, worktree/fork manager, MCP/skills manager, web fleet UI, remote instances, or feature parity with Agent Deck.
3. Freeze scheduler/runner work in `codexctl-dk3` if the user confirms this direction. Keep Beads as the durable ledger and leave runner orchestration to Agent Deck or a separate future product.
4. Invest in permission/lifecycle decisions, deterministic safety gates, outcome attribution, calibration, counterfactual review, canonical examples, per-project preferences, evals, and focused Brain Review UX.
5. Keep the existing dashboard temporarily as a compatibility and diagnostic surface, but narrow new UI work to explaining BRAIN inputs, decisions, confidence, outcomes, and corrections.
6. Keep the architecture headless-compatible through CLI/JSON/hook seams. Do not extract a service until a second real consumer establishes the protocol requirements.
7. Avoid a hard Agent Deck dependency. Codex hooks and BRAIN review should continue to work with Agent Deck, Kitty, tmux, or plain terminal sessions.

## Recommended Beads

No new implementation bead should be created before the product fork is confirmed. On confirmation, update or supersede `codexctl-dk3` with the evidence in this research bead rather than creating a duplicate roadmap task.

## Open Questions

- Is the actual north star better permission/judgment quality, or unattended overnight execution? These imply different products and should not remain blended.
- Should the existing session dashboard remain as a thin BRAIN cockpit indefinitely, or be deprecated after equivalent JSON/CLI integration is proven?
- Which first BRAIN quality metric should drive the roadmap: false-approve rate, calibrated accuracy, correction reuse, or time-to-unblock?

## Refuted / Discarded Claims

- **“Agent Deck is only a dashboard.”** Discarded: official docs show conductors, child linkage, heartbeat, watchdog, worktrees, notifications, and remote channels [S1][S2][S3].
- **“Agent Deck already duplicates the complete codexctl BRAIN.”** Discarded: Agent Deck has policy/learning files and self-improvement, but released evidence does not show the same structured permission adjudication, decision/outcome calibration, or counterfactual review loop [S2][S5].
- **“BRAIN-first means extracting a daemon immediately.”** Discarded: the current BRAIN is coupled to root configuration, rules, session types, runtime adapters, and TUI ownership; extraction should follow a proven second consumer [S6].

## Verification Notes

- Layer 1: all external URLs below resolved successfully on 2026-07-17.
- Layer 2: the load-bearing Agent Deck session-manager and conductor claims were independently re-fetched and returned `SUPPORTED` with high confidence.
- Local codexctl claims were verified directly against the current checkout and cited implementation/test paths. No recommendation relies on an unverified external claim.

## Sources

- [S1: Agent Deck README v1.9.73](https://github.com/asheshgoplani/agent-deck/blob/v1.9.73/README.md) — Primary/Official — 2026-06-21 release — mission-control, session, worktree, cost, and conductor surface.
- [S2: Agent Deck conductor docs v1.9.73](https://github.com/asheshgoplani/agent-deck/blob/v1.9.73/docs/conductor/README.md) — Primary/Official — 2026-06-21 release — conductor process, policy, learning, state, heartbeat, and escalation.
- [S3: Agent Deck watchdog docs v1.9.73](https://github.com/asheshgoplani/agent-deck/blob/v1.9.73/documentation/WATCHDOG.md) — Primary/Official — 2026-06-21 release — restart and stuck-child babysitting.
- [S4: codexctl README](https://github.com/aleadag/codexctl/blob/main/README.md) — Primary/Official — checked 2026-07-17 — product promise, actions, learning/review, and external coordination boundary.
- [S5: Agent Deck self-improvement reference v1.9.73](https://github.com/asheshgoplani/agent-deck/blob/v1.9.73/skills/agent-deck/references/self-improvement.md) — Primary/Official — 2026-06-21 release — transcript analysis and improvement workflow.
- [S6: codexctl contributing architecture](https://github.com/aleadag/codexctl/blob/main/docs/contributing.md) — Primary/Official — checked 2026-07-17 — crate ownership and binary-only BRAIN boundary.
- [S7: Agent Deck v1.9.73 release](https://github.com/asheshgoplani/agent-deck/releases/tag/v1.9.73) — Primary/Official — published 2026-06-21 — version baseline; main was newer at commit `350a640` on 2026-07-12.
