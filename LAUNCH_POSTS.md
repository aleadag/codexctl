# Launch Posts

## Where To Launch

Start here:

- GitHub release notes
- GitHub Discussions
- `r/CodexCode`
- DEV
- X

Then use these for a stronger feature drop:

- Show HN
- Lobsters

Reason:

- GitHub, Reddit, DEV, and X are the best fit for the current audience and project size
- Show HN and Lobsters work better when the release has a sharp technical hook and fresh demo

## Launch Order

### Day 1

- Publish the release
- Update the README and landing page
- Open a GitHub Discussion
- Post to `r/CodexCode`
- Post to X

### Day 2

- Publish the DEV post
- Reply to relevant Codex threads with the demo GIF

### Next feature release

- Post to Show HN
- Post to Lobsters

## GitHub Discussion

Title:

`codexctl`: orchestrate a swarm of Codex agents with a local-LLM brain that learns from you

Body:

I kept losing track of which Codex session was blocked, waiting for approval, or quietly burning budget, so I built `codexctl`.

It is a local dashboard for supervising multiple Codex sessions from one terminal.

Useful if you run several sessions at once and want to:

- see every session at once
- approve prompts without tab hunting
- enforce spend budgets
- jump to the right terminal quickly
- run dependency-ordered task graphs

Quick start:

```bash
brew install aleadag/tap/codexctl
codexctl --demo
```

Repo:

- https://github.com/aleadag/codexctl

## Reddit

Subreddit:

- `r/CodexCode`

Title:

I built a terminal dashboard for supervising multiple Codex sessions

Body:

Running multiple Codex tabs and losing track of which one is blocked, waiting for approval, or quietly burning money got old fast.

So I built `codexctl`, a local dashboard for supervising Codex from one terminal.

It is useful if you run several sessions at once and want to:

- see every session at once
- approve prompts without tab hunting
- set spend budgets and auto-kill over-budget runs
- jump to the right terminal quickly
- record demo GIFs from the dashboard

Quick start:

```bash
brew install aleadag/tap/codexctl
codexctl --demo
```

Repo:

- https://github.com/aleadag/codexctl

If this matches how you use Codex, I’d like to know what breaks first for you once you have 3 or more sessions open.

## DEV

Title:

Orchestrate a swarm of Codex agents with a local-LLM brain that learns from you

Body:

When I started running several Codex sessions in parallel, the operational failure mode was obvious:

- one tab was blocked on a permission prompt
- another was chewing through budget
- another looked alive but was actually stalled

Codex is strong at execution. It is not built to supervise five terminals at once.

So I built `codexctl`, a local operator layer for Codex. It gives me one dashboard to:

- see every session
- approve prompts without tab hunting
- control spend with budgets and auto-kill
- jump to the right terminal
- coordinate multi-session task graphs

Quick start:

```bash
brew install aleadag/tap/codexctl
codexctl --demo
```

Repo:

- https://github.com/aleadag/codexctl

## X

Post:

I got tired of tab-hunting 5 Codex sessions, so I built `codexctl`.

It shows which agent is blocked, waiting for approval, over budget, or stalled, and lets me intervene from one terminal dashboard.

```bash
brew install aleadag/tap/codexctl
codexctl --demo
```

Repo:
https://github.com/aleadag/codexctl

Attach:

- `docs/assets/github-social-preview.png`
- or a short GIF from `docs/assets/codexctl-demo-hero.gif`

## Show HN

Title:

Show HN: codexctl – orchestrate a swarm of Codex agents with a local-LLM brain that learns from you

Body:

If you run several Codex sessions at once, `codexctl` shows which one is blocked, waiting for approval, over budget, or stalled, and lets you intervene from one terminal dashboard.

It is local-only, zero-config, and currently supports macOS and Linux terminals including Ghostty, tmux, Kitty, Warp, iTerm2, and GNOME Terminal.

Quick start:

```bash
brew install aleadag/tap/codexctl
codexctl --demo
```

Repo:

- https://github.com/aleadag/codexctl

## Lobsters

Title:

codexctl: a terminal control plane for Codex sessions

Summary:

`codexctl` is a Rust CLI for supervising multiple Codex sessions from one terminal. It tracks session state, surfaces blocked prompts and spend, supports terminal switching and input for supported terminals, and can orchestrate dependency-ordered task graphs.

Best attached artifact:

- the demo GIF
- or a short architecture comment describing how local session discovery works
