# Terminal support

Run `coding-brain doctor` inside the same terminal family that launches Codex. The report checks session discovery and whether Coding Brain can switch to a selected session.

## Navigation matrix

| Terminal | Switch to session | Setup |
| --- | --- | --- |
| Ghostty | Yes | macOS Automation/Accessibility permission |
| Kitty | Yes | `allow_remote_control yes` in `kitty.conf` |
| tmux | Yes | Coding Brain must reach the same tmux server |
| WezTerm | Yes | Reachable `wezterm cli` mux server |
| Warp | Yes | macOS Automation/Accessibility permission |
| iTerm2 | Yes | macOS Automation/Accessibility permission |
| Terminal.app | Yes | macOS Automation/Accessibility permission |
| GNOME Terminal | No | Launch-only backend; switch is not exposed |
| Windows Terminal from WSL | No | Launch-only bridge; remote tab control is not exposed |

Coding Brain restores the terminal before it hands control to an external attach command and re-enters the TUI after that command returns.

## Optional Agent Deck

When a selected session is managed by [Agent Deck](https://github.com/asheshgoplani/agent-deck), the TUI can attach through Agent Deck's tmux workflow. Coding Brain detects this at the time you choose "switch to session"; Agent Deck is optional, and a missing installation or cancelled attach leaves the Brain TUI usable.

Use `coding-brain doctor` for concrete setup advice when a supported terminal cannot be reached.
