# codexctl

Control plane for Codex sessions with a local-LLM brain, multi-session coordination, and terminal dashboard.

`codexctl` watches Codex JSONL transcripts under `~/.codex/sessions`, tracks session health, coordinates parallel work, and can use a local model to approve, deny, route, or terminate work according to your rules. It is local-first: decision logs, preferences, coordination state, and learned examples stay on your machine.

## Install

```bash
cargo install codexctl
cargo install --path .
```

The package builds the `codexctl` binary and the `codexctl-core` / `codexctl-tui` workspace crates.

## Get Started

```bash
codexctl init                # Onboarding wizard
codexctl doctor              # Verify install and runtime health
codexctl                     # Live dashboard for Codex sessions
codexctl --brain             # Enable local LLM supervision
```

After upgrading, run:

```bash
codexctl init --upgrade
```

This refreshes Codex hook entries and local database migrations.

## What It Does

- **Codex transcript discovery**: reads recursive `~/.codex/sessions/**/rollout-*.jsonl` session transcripts.
- **Local LLM supervision**: uses OpenAI-compatible local endpoints such as Ollama, llama.cpp, vLLM, or LM Studio.
- **Multi-session orchestration**: runs dependency-ordered tasks, coordinates handoffs, and detects file conflicts.
- **Health monitoring**: detects loops, stalls, context pressure, cost spikes, and long-running blocked work.
- **Learning from corrections**: stores approval/rejection outcomes locally and adapts confidence thresholds per project and tool.
- **Terminal integration**: supports Ghostty, Kitty, tmux, WezTerm, Warp, iTerm2, Terminal.app, Gnome Terminal, and Windows Terminal where available.

## Local LLM Brain

```bash
ollama pull gemma4:e4b
ollama serve
codexctl --brain
codexctl --brain --auto-run
```

The brain can:

- approve routine reads, searches, and test commands,
- deny risky or destructive commands,
- route summarized context between sessions,
- terminate sessions that appear stuck or unsafe,
- suggest durable rules from repeated corrections.

Prompt overrides live in:

```text
~/.codexctl/brain/prompts/
```

State is still stored under `~/.codexctl` for upgrade compatibility.

## Build And Test

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Release builds:

```bash
cargo build --release
cargo build --release --features "bus,coord,relay,hive"
```

## Architecture

This is a three-crate Cargo workspace:

```text
crates/
├── codexctl-core/    # core types, discovery, monitoring, runtime traits
└── codexctl-tui/     # terminal UI, demo fixtures, recording
src/                   # codexctl binary: brain, bus, coord, hive, relay, init
```

Dependency direction is strict:

```text
codexctl -> codexctl-tui -> codexctl-core
```

`codexctl-core` must not depend on binary-only modules.

## Codex Integration Points

- `~/.codex/sessions/**/rollout-*.jsonl` for session discovery and monitoring.
- `.codex/hooks.json` and `~/.codex/hooks.json` for hook install.
- `~/.codex/skills` and `~/.codex/plugins/*/skills` for skill discovery.
- `codex` and `codex exec` for launched or delegated work.

## Configuration

Project config:

```text
.codexctl.toml
```

Global config:

```text
~/.config/codexctl/config.toml
```

These paths remain unchanged for compatibility with existing installs.
