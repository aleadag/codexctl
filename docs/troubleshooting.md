# Troubleshooting

## No sessions found

- Run `codexctl init` (or `codexctl --init` for hook-only install) if you haven't already — this wires up the Codex hooks
- Ensure Codex is running (`codex` in another terminal)
- Check that `~/.codex/sessions/` contains `.json` files
- Run `codexctl --log /tmp/codexctl.log` and check the log

## Tab switching doesn't work

Run `codexctl doctor` first — it checks everything (PATH, hooks, plugin install, brain endpoint, bus, session discovery, terminal) and tells you the exact command to fix anything broken. For terminal-specific diagnostics, the legacy `codexctl --doctor` flag still works.

- **GNOME Terminal**: Launch support is available; use tmux or Kitty if you need remote switching or input automation
- **Windows Terminal on WSL**: Launch support is available when `cmd.exe /c wt.exe` works; use tmux or Kitty inside WSL for switching and input automation
- **Ghostty**: Should work out of the box
- **Kitty**: Add `allow_remote_control yes` to `~/.config/kitty/kitty.conf`
- **Warp/iTerm2/Terminal.app**: Grant Automation/Accessibility permission in System Settings > Privacy & Security
- **tmux**: Must be running inside a tmux session

## Cost shows $0.00

codexctl reads token usage from JSONL logs. If the session just started, wait for the first response to complete. Check that `~/.codex/projects/` contains `.jsonl` files.

## High CPU usage from codexctl itself

Increase the poll interval: `codexctl --interval 3000` (default is 2000ms).

## Brain not responding

- Check the brain endpoint is running: `curl http://localhost:11434/api/tags`
- Check brain gate mode: `codexctl --mode status` (if `off`, the brain is disabled)
- Check the brain model is loaded: `ollama list`
- Run `codexctl doctor` for the full install + runtime checklist

## Plugin hook not firing

- Verify the plugin is installed and enabled in Codex
- Check that `codexctl` is on your PATH: `which codexctl`
- Test the brain query manually: `codexctl --brain --brain-query --tool Bash --tool-input "echo hi"`
- Check brain gate mode: `codexctl --mode status`

## FAQ

**Does codexctl modify Codex or its files?**
Only `codexctl init` (the Plugin phase), the legacy `--init`/`--uninstall` flags, and `init --remove`/`init --purge` write to `.codex/hooks.json` (to add/remove hooks). Everything else is read-only. The only other writes are to codexctl's own state under `~/.codexctl/` (bus DB, brain decisions, hive knowledge, etc. — wipe with `codexctl init --purge`).

**Does it need an API key?**
No. It reads local files on disk. No network access required (unless you configure webhooks).

**Does it work with Codex in VS Code / JetBrains?**
It monitors any Codex process, regardless of how it was launched. Terminal-specific features (tab switching, input) require a supported terminal.

**Can I use it with a single session?**
Yes, but the value increases with concurrency. If you run one session, you already know where it is.

**What about Windows?**
Native Windows is not supported yet. WSL plus Windows Terminal can now launch new Codex tabs through `codexctl --new` or `n`, and WSL plus `tmux` remains the recommended setup when you also want switch/input/approve automation.

For other issues, run with `--log` and [open an issue](https://github.com/aleadag/codexctl/issues/new) with the log attached.
