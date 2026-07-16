# Lifecycle Hook Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use beads-superpowers:subagent-driven-development (recommended) or beads-superpowers:executing-plans to implement this plan task-by-task. Each Task becomes a bead (`bd create -t task --parent <epic-id>`). Steps within tasks use checkbox (`- [ ]`) syntax for human readability.

**Goal:** Remove codexctl's invalid and ineffective `PostToolUse` and `Stop` snapshot handlers while preserving permission automation and every independently owned hook.

**Architecture:** Imperative init remains an exact-match upgrader: it recognizes legacy codexctl snapshot commands only so it can remove them, then installs the single current `PermissionRequest` definition. The Home Manager module stops generating lifecycle snapshot entries and continues merging only the immutable absolute-path permission handler; unrelated hooks, including `codex-jj-stop-hook`, remain owned by their defining modules.

**Tech Stack:** Rust, Serde JSON, Cargo tests, Nix/Home Manager module evaluation, Markdown documentation, Jujutsu.

## Global Constraints

- Bead: `codexctl-9am`; approved design: `.internal/specs/2026-07-16-lifecycle-hook-cleanup-design.md`.
- `codex-jj-stop-hook` is Home Manager-owned and must remain structurally unchanged.
- A handler is codexctl-owned only when its executable is bare `codexctl` or an absolute path ending in `/codexctl` and its arguments match an exact managed form.
- Keep recognizing bare and absolute legacy `--json` commands, with or without the exact `2>/dev/null || true` suffix, for migration and uninit only.
- Do not add a replacement lifecycle adapter, event store, or background communication path; that remains `codexctl-rqm`.
- Do not change `PermissionRequest` output, trust behavior, discovery, or terminal fallback blocking.
- Do not rename `.codexctl` state or configuration paths.
- Use emoji conventional jj descriptions and include the Bead ID; do not push.

---

### Task 1: Remove Imperative Lifecycle Snapshot Hooks

**Bead:** `codexctl-9am.2.1`

**Files:**
- Modify: `src/init/hooks.rs:4-42`
- Modify: `src/init/hooks.rs:341-446`
- Modify: `src/init/hooks.rs:576-805`
- Modify: `src/init/hooks.rs:888-1000`

**Interfaces:**
- Consumes: existing `is_codexctl_program`, `is_exact_codexctl_command`, `filter_managed_hooks`, and `remove_codexctl_hooks` ownership rules.
- Produces: `build_hooks_value()` containing only the current `PermissionRequest` definition; `merge_hooks(&mut Value)` that removes legacy managed handlers from every event before appending current definitions.

**Acceptance Criteria:**
- Fresh imperative init generates only the codexctl `PermissionRequest` hook.
- Init removes supported legacy codexctl `PostToolUse` and `Stop` snapshot handlers rather than replacing them.
- Mixed matcher groups retain unrelated commands and an independent Stop hook unchanged.
- Bare and absolute legacy forms remain removable; relative and lookalike executables remain user-owned.
- Permission hook migration, discovery, and idempotence remain unchanged.

- [ ] **Step 1: Start the task changeset before editing**

```bash
jj new -m "🐛 fix: remove invalid imperative lifecycle hooks (codexctl-9am)"
jj --no-pager st
```

Expected: an empty working-copy changeset with the exact description above.

- [ ] **Step 2: Rewrite the installer expectations first**

In `src/init/hooks.rs`, change `test_build_hooks_value` and `test_merge_hooks_empty` so they require only `PermissionRequest`:

```rust
#[test]
fn test_build_hooks_value() {
    let hooks = build_hooks_value();
    let obj = hooks.as_object().unwrap();

    assert_eq!(obj.len(), 1);
    assert!(obj.contains_key("PermissionRequest"));
    assert!(!obj.contains_key("PostToolUse"));
    assert!(!obj.contains_key("Stop"));
}

#[test]
fn test_merge_hooks_empty() {
    let mut settings = serde_json::json!({});
    merge_hooks(&mut settings);

    let hooks = settings["hooks"].as_object().unwrap();
    assert_eq!(hooks.len(), 1);
    assert!(hooks.contains_key("PermissionRequest"));
    assert!(!hooks.contains_key("PostToolUse"));
    assert!(!hooks.contains_key("Stop"));
}
```

- [ ] **Step 3: Add the migration and ownership regression before implementation**

Replace `merge_replaces_absolute_managed_hooks_without_duplicates` with this focused regression:

```rust
#[test]
fn merge_removes_legacy_lifecycle_hooks_and_preserves_external_stop() {
    let external_stop = serde_json::json!({
        "type": "command",
        "command": "/nix/store/test-codex-jj-stop-hook",
    });
    let mut settings = serde_json::json!({
        "hooks": {
            "PermissionRequest": [{
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": "/nix/store/test-codexctl/bin/codexctl --permission-hook",
                    "timeout": 30,
                    "statusMessage": "Brain reviewing permission…"
                }]
            }],
            "PostToolUse": [{
                "matcher": "*",
                "hooks": [{
                    "type": "command",
                    "command": "/nix/store/test-codexctl/bin/codexctl --json 2>/dev/null || true",
                    "timeout": 5
                }]
            }],
            "Stop": [{
                "hooks": [
                    external_stop.clone(),
                    {
                        "type": "command",
                        "command": "codexctl --json",
                        "timeout": 5
                    }
                ]
            }]
        }
    });

    merge_hooks(&mut settings);
    let once = settings.clone();
    merge_hooks(&mut settings);

    assert_eq!(settings, once);
    assert!(settings["hooks"].get("PostToolUse").is_none());
    assert_eq!(
        settings["hooks"]["Stop"][0]["hooks"],
        serde_json::json!([external_stop])
    );
    assert_eq!(
        settings["hooks"]["PermissionRequest"][0]["hooks"][0]["command"],
        "codexctl --permission-hook"
    );
}
```

Change `merge_replaces_absolute_snapshot_without_shell_suffix_idempotently` to assert that the `PostToolUse` event is absent after `merge_hooks` instead of expecting a replacement snapshot command.

- [ ] **Step 4: Run the focused tests and verify RED**

```bash
cargo test --bin codexctl init::hooks::tests::test_build_hooks_value -- --exact
cargo test --bin codexctl init::hooks::tests::merge_removes_legacy_lifecycle_hooks_and_preserves_external_stop -- --exact
```

Expected: the first test fails because `build_hooks_value` still contains three events; the second fails because merge still appends codexctl `PostToolUse` and `Stop` handlers.

- [ ] **Step 5: Remove the current lifecycle specs and make migration global**

In `src/init/hooks.rs`, leave `HOOKS` with only this entry:

```rust
const HOOKS: &[HookSpec] = &[HookSpec {
    event: "PermissionRequest",
    matcher: "Bash",
    command: "codexctl --permission-hook",
    timeout: 30,
    status_message: Some("Brain reviewing permission…"),
}];
```

Replace the module comment with:

```rust
/// The current hook codexctl installs into Codex hooks.json.
///
/// PermissionRequest is the only active integration. Legacy PostToolUse and
/// Stop snapshot commands remain recognized below solely for exact cleanup
/// during init and uninit.
```

At the beginning of `merge_hooks`, remove every managed definition before appending current hooks:

```rust
fn merge_hooks(existing: &mut serde_json::Value) {
    remove_codexctl_hooks(existing);
    let new_hooks = build_hooks_value();

    let hooks_obj = existing
        .as_object_mut()
        .expect("settings must be an object")
        .entry("hooks")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let (Some(target), Some(source)) = (hooks_obj.as_object_mut(), new_hooks.as_object()) {
        for (event, new_matchers) in source {
            let event_arr = target
                .entry(event)
                .or_insert_with(|| serde_json::Value::Array(Vec::new()));
            if let (Some(arr), Some(new_arr)) =
                (event_arr.as_array_mut(), new_matchers.as_array())
            {
                for new_matcher in new_arr {
                    arr.push(new_matcher.clone());
                }
            }
        }
    }
}
```

Keep `is_managed_snapshot_command` and the `PostToolUse | Stop` branch of `is_managed_command`; they now exist only to migrate and uninstall exact legacy commands.

- [ ] **Step 6: Update success text and affected existing assertions**

Change `print_success` to list only:

```rust
println!("Hooks installed:");
println!("  PermissionRequest (Bash) — lets the brain allow or deny requests");
```

Update existing tests to reflect one current installed handler:

- `test_merge_hooks_preserves_existing`: preserve the existing Write matcher and append only Bash; assert no codexctl `PostToolUse` or `Stop` events.
- `test_remove_codexctl_hooks_all`: expect one removal after constructing fresh current settings.
- `test_remove_then_no_hooks_key`: continue expecting the `hooks` key to disappear after removing the one current handler.
- `merge_replaces_absolute_snapshot_without_shell_suffix_idempotently`: expect the legacy event to be deleted.
- Preserve `test_has_codexctl_hooks_present`, legacy PermissionRequest migration, relative-path preservation, and lookalike tests because they prove migration ownership.

- [ ] **Step 7: Run the complete imperative hook suite and verify GREEN**

```bash
cargo test --bin codexctl init::hooks::tests
cargo test --bin codexctl init::state::tests
cargo test --bin codexctl doctor::tests
cargo fmt --all --check
```

Expected: every focused Rust test passes and formatting is clean.

- [ ] **Step 8: Review the task changeset**

```bash
jj --no-pager diff --git
jj --no-pager st
```

Expected: only `src/init/hooks.rs` changes; the working-copy description contains `codexctl-9am`.

---

### Task 2: Remove Declarative Lifecycle Hooks and Update Documentation

**Bead:** `codexctl-9am.2.2`

**Files:**
- Modify: `nix/home-manager.nix:18-116`
- Modify: `nix/tests/home-manager-module.nix:175-216`
- Modify: `docs/configuration.md:35-43`

**Interfaces:**
- Consumes: Home Manager's `programs.codex.hooks` merge interface and the immutable `lib.getExe cfg.package` permission command.
- Produces: `programs.codexctl.codexHooks.enable` contributing only a `PermissionRequest` matcher and trust notice.

**Acceptance Criteria:**
- The Home Manager module generates no codexctl `PostToolUse` or `Stop` entry.
- The independently supplied Stop hook remains the only Stop entry and remains unchanged.
- The permission handler still uses the selected package's absolute Nix-store executable, timeout, matcher, and status message.
- Module option text and configuration documentation describe permission-hook integration rather than lifecycle hooks.
- Focused Nix evaluation and all repository quality gates pass.

- [ ] **Step 1: Start the task changeset before editing**

```bash
jj new -m "🐛 fix: remove invalid declarative lifecycle hooks (codexctl-9am)"
jj --no-pager st
```

Expected: an empty working-copy changeset on top of Task 1.

- [ ] **Step 2: Change the Home Manager assertions first**

In `nix/tests/home-manager-module.nix`, remove `postToolUse`, `postToolUseHandler`, `generatedStop`, and `generatedStopHandler`. Keep:

```nix
stopHooks = cfg.programs.codex.hooks.Stop;
```

Replace the generated lifecycle assertions with:

```nix
assert builtins.length dualAliasConfigured.config.programs.codex.hooks.PermissionRequest == 1;
assert !(dualAliasConfigured.config.programs.codex.hooks ? PostToolUse);
assert !(dualAliasConfigured.config.programs.codex.hooks ? Stop);
assert permission.matcher == "Bash";
assert permissionHandler.type == "command";
assert permissionHandler.command == "${expectedExe} --permission-hook";
assert permissionHandler.timeout == 30;
assert permissionHandler.statusMessage == "Brain reviewing permission…";
assert stopHooks == [ existingStop ];
```

Keep the package-only, unsupported-hook-option, disabled-Codex, trust-notice, config-file, and no-systemd-service assertions.

- [ ] **Step 3: Run the focused Nix check and verify RED**

```bash
nix build .#checks.x86_64-linux.home-manager-module
```

Expected: evaluation fails because the module still contributes `PostToolUse` and an additional `Stop` entry.

- [ ] **Step 4: Remove declarative lifecycle definitions**

In `nix/home-manager.nix`:

- delete `refreshCommand`;
- change `codexHooks.enable.description` to `"Whether to merge the codexctl permission hook into programs.codex.hooks."`; and
- leave `programs.codex.hooks` with only this definition:

```nix
programs.codex.hooks.PermissionRequest = lib.mkAfter [
  {
    matcher = "Bash";
    hooks = [
      {
        type = "command";
        command = "${executable} --permission-hook";
        timeout = 30;
        statusMessage = "Brain reviewing permission…";
      }
    ];
  }
];
```

Keep the existing package assertion, `programs.codex.enable` assertion, and trust activation. The trust notice remains valid because the immutable permission command still changes when the package changes.

- [ ] **Step 5: Correct the public Home Manager documentation**

Replace the lifecycle-hook paragraph in `docs/configuration.md` with:

```markdown
The module installs its selected package, writes the settings as TOML, and merges a `PermissionRequest` handler into `programs.codex.hooks`. The handler calls the selected codexctl package by its immutable Nix store path rather than relying on `PATH`. Hooks configured by other Home Manager modules remain independent and are preserved.
```

Keep the Nix-store secret warning and `/hooks` trust guidance unchanged.

- [ ] **Step 6: Run focused declarative validation and verify GREEN**

```bash
nix fmt -- --check .
nix build .#checks.x86_64-linux.home-manager-module
```

Expected: formatting and the Home Manager evaluation pass; the test proves `stopHooks == [ existingStop ]`.

- [ ] **Step 7: Run the full repository quality gates**

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
nix flake check
```

Expected: every command exits zero with no warnings promoted to errors.

- [ ] **Step 8: Verify scope and live generated configuration**

```bash
rg -n 'codexctl --json|PostToolUse|notifies codexctl' src/init/hooks.rs nix/home-manager.nix nix/tests/home-manager-module.nix docs/configuration.md
jj --no-pager diff --git
jj --no-pager st
```

Expected: the scan finds legacy `--json` references only in exact cleanup and migration tests; it finds no generated lifecycle definition or obsolete notification text. The diff contains only the approved Rust, Nix, test, documentation, spec, and plan surfaces, and nothing is pushed.
