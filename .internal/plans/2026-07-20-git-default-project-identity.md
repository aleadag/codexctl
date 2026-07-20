# Git-Default Project Identity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use beads-superpowers:subagent-driven-development (recommended) or beads-superpowers:executing-plans to implement this plan task-by-task. Each Task becomes a bead (`bd create -t task --parent <epic-id>`). Steps within tasks use checkbox (`- [ ]`) syntax for human readability.

**Goal:** Give Git repositories a durable Coding Brain project identity derived from their canonical `origin` remote, while preserving manifest overrides and temporary fallback behavior.

**Architecture:** Keep identity resolution in `coding-brain-core::project`. Resolve the Git root, prefer its manifest, otherwise normalize `origin` and generate a namespaced UUIDv5; Doctor consumes the same resolver and changes only its user-facing status text.

**Tech Stack:** Rust 2024, `std::process::Command`, `uuid` v4/v5 features, TOML, Cargo tests, Jujutsu.

## Global Constraints

- A valid `<git-root>/.coding-brain/project.toml` remains authoritative.
- Equivalent HTTPS, SSH, and SCP-style `origin` URLs produce the same UUID.
- Local and `file` remotes remain temporary because their paths are machine-specific.
- Raw remote URLs are never persisted or logged.
- Missing or unusable Git metadata falls back to `ProjectId::Temporary` without failing hook evaluation.
- Malformed manifests remain errors and never fall back to Git.
- Existing temporary activity is not migrated or rewritten.
- A reused hosting slug intentionally reuses identity unless a manifest overrides it.
- Remote-derived identity is not proof of ownership and must not become an authorization boundary.
- Git resolution uses only two read-only local commands and never contacts a remote; cache and timeout tuning is deferred pending measurement.
- Keep the serialized `ProjectId` shape and activity schema unchanged.
- Use `bd -C /home/alexander/.beads-planning` for implementation tracking.
- Use jj, not raw Git, for repository history operations. Do not commit or push without user authorization.

---

## File Map

- `crates/coding-brain-core/Cargo.toml`: enable deterministic UUIDv5 generation.
- `Cargo.lock`: record the UUID feature dependency resolution if Cargo changes it.
- `crates/coding-brain-core/src/project.rs`: own Git-root discovery, remote normalization, UUID derivation, manifest precedence, fallback behavior, and focused tests.
- `src/doctor.rs`: report Git-derived identities as stable and retain `init` guidance only for temporary identities.

### Task 1: Canonical Git Remote Fingerprint

**Files:**

- Modify: `crates/coding-brain-core/Cargo.toml:28`
- Modify: `Cargo.lock`
- Modify: `crates/coding-brain-core/src/project.rs:1`
- Test: `crates/coding-brain-core/src/project.rs:190`

**Interfaces:**

- Consumes: a Git remote string.
- Produces: `canonical_remote(remote: &str) -> Option<String>` and `git_remote_project_id(git_root: &Path) -> Option<ProjectId>` for Task 2.

**Acceptance Criteria:**

- Common HTTPS, SSH URL, and SCP-style forms normalize to `host[:port]/path`.
- Scheme and credentials do not affect identity; host case is folded, while path case and explicit ports remain significant.
- Query strings and fragments are removed; percent encoding and host aliases remain literal.
- Only `https`, `http`, `ssh`, `git`, `ftp`, and `ftps` URL schemes are accepted; ambiguous SCP-style bracketed IPv6 is rejected.
- Local and `file` remotes are rejected as durable identity sources.
- Empty or structurally invalid remotes are rejected.
- UUIDv5 input is versioned and raw remote text is neither persisted nor logged.

Before Step 1, if history mutation is authorized, describe the current empty jj
changeset:

```bash
jj desc -m "✨ feat: derive project IDs from canonical Git remotes"
```

- [ ] **Step 1: Enable UUIDv5 support**

Change the core dependency to:

```toml
uuid = { version = "1", features = ["v4", "v5"] }
```

Run:

```bash
cargo check -p coding-brain-core
```

Expected: Cargo resolves UUIDv5 support and `coding-brain-core` checks successfully.

- [ ] **Step 2: Write failing normalization tests**

Add table-driven unit tests in `project.rs`:

```rust
#[test]
fn canonicalizes_equivalent_network_remotes() {
    for remote in [
        "https://github.com/Owner/Repo.git",
        "https://user:secret@github.com/Owner/Repo.git",
        "https://github.com/Owner/Repo.git?token=secret#clone",
        "ssh://git@github.com/Owner/Repo.git",
        "git@GITHUB.COM:Owner/Repo.git",
    ] {
        assert_eq!(
            canonical_remote(remote).as_deref(),
            Some("github.com/Owner/Repo")
        );
    }
}

#[test]
fn normalization_preserves_ports_and_path_case() {
    assert_eq!(
        canonical_remote("ssh://git@example.com:2222/Owner/Repo.git").as_deref(),
        Some("example.com:2222/Owner/Repo")
    );
    assert_ne!(
        canonical_remote("https://example.com/Owner/Repo"),
        canonical_remote("https://example.com/owner/repo")
    );
}

#[test]
fn rejects_machine_local_remotes() {
    assert_eq!(canonical_remote("upstream.git"), None);
    assert_eq!(canonical_remote("file:///srv/upstream.git"), None);
}

#[test]
fn rejects_empty_or_incomplete_network_remotes() {
    for remote in [
        "",
        "https://",
        "https://github.com",
        "git@github.com:",
        "custom://github.com/Owner/Repo.git",
        "git@[2001:db8::1]:Owner/Repo.git",
    ] {
        assert_eq!(canonical_remote(remote), None);
    }
}

#[test]
fn accepts_bracketed_ipv6_in_url_form() {
    assert_eq!(
        canonical_remote("ssh://git@[2001:DB8::1]:2222/Owner/Repo.git").as_deref(),
        Some("[2001:db8::1]:2222/Owner/Repo")
    );
}
```

- [ ] **Step 3: Run the focused tests and confirm failure**

Run:

```bash
cargo test -p coding-brain-core project::tests::canonicalizes_equivalent_network_remotes
```

Expected: compilation fails because `canonical_remote` does not exist.

- [ ] **Step 4: Implement normalization and UUIDv5 derivation**

Add private helpers in `project.rs`. Keep them in this module because only project identity consumes them:

```rust
const GIT_REMOTE_NAMESPACE: uuid::Uuid =
    uuid::Uuid::from_u128(0x2c54e35b_775d_4bc5_83df_40d4d2fde58e);

fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn normalize_repo_path(path: &str) -> Option<&str> {
    let path = path.split(['?', '#']).next()?;
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    (!path.is_empty()).then_some(path)
}

fn canonical_network_remote(authority: &str, path: &str) -> Option<String> {
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, value)| value);
    if host_port.is_empty() {
        return None;
    }
    let path = normalize_repo_path(path)?;
    Some(format!("{}/{path}", host_port.to_ascii_lowercase()))
}

fn canonical_remote(remote: &str) -> Option<String> {
    let remote = remote.trim();
    if remote.is_empty() {
        return None;
    }
    if remote.starts_with("file://") {
        return None;
    }
    if let Some((scheme, rest)) = remote.split_once("://") {
        let scheme = scheme.to_ascii_lowercase();
        if !matches!(
            scheme.as_str(),
            "https" | "http" | "ssh" | "git" | "ftp" | "ftps"
        ) {
            return None;
        }
        let (authority, path) = rest.split_once('/')?;
        return canonical_network_remote(authority, path);
    }
    if remote.starts_with('[') || remote.contains("@[") {
        return None;
    }
    if let Some((authority, path)) = remote.split_once(':') {
        if !authority.contains('/') {
            return canonical_network_remote(authority, path);
        }
    }
    None
}

fn git_remote_project_id(git_root: &Path) -> Option<ProjectId> {
    let remote = git_output(git_root, &["remote", "get-url", "origin"])?;
    let canonical = canonical_remote(&remote)?;
    let fingerprint = format!("git-remote:v1:{canonical}");
    Some(ProjectId::Stable(
        uuid::Uuid::new_v5(&GIT_REMOTE_NAMESPACE, fingerprint.as_bytes()).to_string(),
    ))
}
```

Do not add diagnostics containing `remote`; it may contain a credential.

- [ ] **Step 5: Run all project-module tests**

Run:

```bash
cargo test -p coding-brain-core project::tests
```

Expected: all project tests pass.

- [ ] **Step 6: Verify the task checkpoint**

After the task, verify with:

```bash
jj --no-pager st
jj --no-pager diff --git
```

Expected: only the UUID feature, lockfile, normalization helpers, and focused tests are changed. Do not push.

### Task 2: Git-Root Identity Resolution and Manifest Precedence

**Files:**

- Modify: `crates/coding-brain-core/src/project.rs:59`
- Test: `crates/coding-brain-core/src/project.rs:190`

**Interfaces:**

- Consumes: `git_output`, `git_remote_project_id`, `manifest_path`, and `temporary_id` from Task 1 and the existing `CodingBrainPaths` API.
- Produces: unchanged public signatures for `ProjectIdentity::load` and `ProjectManifest::create`; both operate on the resolved Git root when available.

**Acceptance Criteria:**

- Root and nested working directories resolve to one project identity.
- A valid root manifest overrides the Git remote.
- A malformed root manifest returns `ProjectError` without remote fallback.
- Repositories with equivalent remotes share a stable ID.
- Missing Git, missing `origin`, or a local/file `origin` returns a temporary ID rooted at the Git top-level when one is available.
- Manifest creation writes at the Git root and preserves concurrent creation behavior.

Before Step 1, if separate changesets are authorized, start this task with:

```bash
jj new -m "✨ feat: resolve project identity from the Git root"
```

- [ ] **Step 1: Add Git fixture helpers and failing resolver tests**

Add these test helpers and cases to `project.rs`:

```rust
fn run_git(cwd: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn init_git_repository(root: &Path, remote: Option<&str>) {
    fs::create_dir_all(root).unwrap();
    run_git(root, &["init", "--quiet"]);
    if let Some(remote) = remote {
        run_git(root, &["remote", "add", "origin", remote]);
    }
}

#[test]
fn git_root_and_subdirectory_share_remote_identity() {
    let root = tempfile::tempdir().unwrap();
    init_git_repository(root.path(), Some("git@github.com:owner/repo.git"));
    let nested = root.path().join("src/nested");
    fs::create_dir_all(&nested).unwrap();
    let paths = fixture_paths(root.path());
    let root_id = ProjectIdentity::load(root.path(), &paths).unwrap();
    let nested_id = ProjectIdentity::load(&nested, &paths).unwrap();
    assert!(root_id.is_durable());
    assert_eq!(root_id, nested_id);
}

#[test]
fn equivalent_clone_remotes_share_identity() {
    let fixture = tempfile::tempdir().unwrap();
    let first = fixture.path().join("first");
    let second = fixture.path().join("second");
    init_git_repository(
        &first,
        Some("https://user:secret@github.com/Owner/Repo.git?token=hidden"),
    );
    init_git_repository(&second, Some("git@github.com:Owner/Repo.git"));
    let paths = fixture_paths(fixture.path());
    let first_id = ProjectIdentity::load(&first, &paths).unwrap();
    let second_id = ProjectIdentity::load(&second, &paths).unwrap();
    assert_eq!(first_id, second_id);
    let ProjectId::Stable(value) = first_id.id() else {
        panic!("remote identity must be stable");
    };
    assert!(uuid::Uuid::parse_str(value).is_ok());
    assert!(!value.contains("secret"));
    assert!(!value.contains("hidden"));
    assert!(!value.contains("Owner"));
}

#[test]
fn root_manifest_overrides_remote_from_nested_directory() {
    let root = tempfile::tempdir().unwrap();
    init_git_repository(root.path(), Some("https://example.com/owner/repo.git"));
    let paths = fixture_paths(root.path());
    let explicit = ProjectManifest::create(root.path(), &paths).unwrap();
    let nested = root.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    assert_eq!(ProjectIdentity::load(&nested, &paths).unwrap(), explicit);
}

#[test]
fn manifest_creation_from_nested_directory_writes_at_git_root() {
    let root = tempfile::tempdir().unwrap();
    init_git_repository(root.path(), Some("https://example.com/owner/repo.git"));
    let nested = root.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    let paths = fixture_paths(root.path());
    ProjectManifest::create(&nested, &paths).unwrap();
    assert!(root.path().join(".coding-brain/project.toml").is_file());
    assert!(!nested.join(".coding-brain/project.toml").exists());
}

#[test]
fn repository_without_origin_uses_one_temporary_root_identity() {
    let root = tempfile::tempdir().unwrap();
    init_git_repository(root.path(), None);
    let nested = root.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    let paths = fixture_paths(root.path());
    let root_id = ProjectIdentity::load(root.path(), &paths).unwrap();
    let nested_id = ProjectIdentity::load(&nested, &paths).unwrap();
    assert!(!root_id.is_durable());
    assert_eq!(root_id, nested_id);
}

#[test]
fn malformed_root_manifest_does_not_fall_back_to_origin() {
    let root = tempfile::tempdir().unwrap();
    init_git_repository(root.path(), Some("https://example.com/owner/repo.git"));
    fs::create_dir_all(root.path().join(".coding-brain")).unwrap();
    fs::write(
        root.path().join(".coding-brain/project.toml"),
        "schema_version = 2\nproject_id = \"not-a-uuid\"\n",
    )
    .unwrap();
    let nested = root.path().join("nested");
    fs::create_dir_all(&nested).unwrap();
    assert!(ProjectIdentity::load(&nested, &fixture_paths(root.path())).is_err());
}
```

- [ ] **Step 2: Run the focused tests and confirm failure**

Run:

```bash
cargo test -p coding-brain-core project::tests::git_root_and_subdirectory_share_remote_identity
```

Expected: the nested directory receives a different temporary ID or fails to find the root manifest.

- [ ] **Step 3: Implement Git-root resolution and precedence**

Add the root helper:

```rust
fn git_root(cwd: &Path) -> Option<PathBuf> {
    let root = git_output(cwd, &["rev-parse", "--show-toplevel"])?;
    let root = PathBuf::from(root);
    if root.is_absolute() {
        Some(root)
    } else {
        fs::canonicalize(cwd.join(root)).ok()
    }
}

fn project_root(cwd: &Path) -> PathBuf {
    git_root(cwd).unwrap_or_else(|| cwd.to_path_buf())
}
```

Replace `ProjectIdentity::load` with the following precedence:

```rust
pub fn load(cwd: &Path, paths: &CodingBrainPaths) -> Result<Self, ProjectError> {
    let root = project_root(cwd);
    let manifest_path = manifest_path(&root, paths);
    match fs::read_to_string(&manifest_path) {
        Ok(contents) => ProjectManifest::parse(&contents),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(id) = git_remote_project_id(&root) {
                return Ok(Self { id });
            }
            let canonical = fs::canonicalize(&root)?;
            Ok(Self {
                id: ProjectId::Temporary(temporary_id(&canonical)),
            })
        }
        Err(error) => Err(error.into()),
    }
}
```

In `ProjectManifest::create`, compute `let root = project_root(cwd);`, then use
`paths.project_dir(&root)`, `manifest_path(&root, paths)`, and
`ProjectIdentity::load(&root, paths)`. Keep the existing atomic
`persist_noclobber`, permissions, and directory sync logic unchanged.

- [ ] **Step 4: Run project and hook-facing tests**

Run:

```bash
cargo test -p coding-brain-core project::tests
cargo test lifecycle_hook
cargo test permission_hook
```

Expected: all commands pass; manifest concurrency and hook evaluation remain intact.

- [ ] **Step 5: Verify the task checkpoint**

Verify:

```bash
jj --no-pager st
jj --no-pager diff --git
```

Expected: the diff is limited to identity resolution and its tests. Do not push.

### Task 3: Doctor Messaging and Full Verification

**Files:**

- Modify: `src/doctor.rs:385`
- Test: `src/doctor.rs:385`

**Interfaces:**

- Consumes: unchanged `ProjectIdentity::load`, `ProjectIdentity::is_durable`, `CodingBrainPaths`, and `Path`.
- Produces: `check_project_identity_at(cwd: &Path, paths: &CodingBrainPaths) -> Check` for deterministic tests; `check_project_identity()` remains the CLI entry point.

**Acceptance Criteria:**

- Doctor passes for manifest-backed and Git-backed stable identities.
- The pass message does not claim every stable identity came from a manifest UUID.
- Doctor recommends `coding-brain init` only for temporary identity.
- Malformed manifest guidance remains advisory and actionable.
- Formatting, full tests, Clippy, and build all pass.

Before Step 1, if separate changesets are authorized, start this task with:

```bash
jj new -m "🩺 fix: clarify automatic project identity diagnostics"
```

- [ ] **Step 1: Extract a deterministic Doctor helper and write failing tests**

Keep environment resolution in `check_project_identity`, then delegate:

```rust
fn check_project_identity_at(
    cwd: &Path,
    paths: &coding_brain_core::paths::CodingBrainPaths,
) -> Check {
    match coding_brain_core::project::ProjectIdentity::load(cwd, paths) {
        Ok(identity) if identity.is_durable() => Check {
            name: "project identity".into(),
            status: CheckStatus::Pass,
            message: "stable project identity loaded".into(),
            fix_hint: None,
        },
        Ok(_) => Check {
            name: "project identity".into(),
            status: CheckStatus::Advisory,
            message: "no manifest or usable Git origin; memory is temporary".into(),
            fix_hint: Some(
                "Run `coding-brain init` to create an explicit identity override. Removing .coding-brain/project.toml before rerunning init deliberately creates a new identity."
                    .into(),
            ),
        },
        Err(error) => Check {
            name: "project identity".into(),
            status: CheckStatus::Advisory,
            message: format!("project manifest is malformed: {error}"),
            fix_hint: Some(
                "Fix .coding-brain/project.toml, or remove it before `coding-brain init` to deliberately create a new identity."
                    .into(),
            ),
        },
    }
}
```

Change the top-level path import and add local test helpers so Doctor tests do
not mutate the process working directory:

```rust
use std::path::{Path, PathBuf};

fn fixture_paths(home: &Path) -> coding_brain_core::paths::CodingBrainPaths {
    coding_brain_core::paths::CodingBrainPaths::resolve(
        &coding_brain_core::paths::PathEnvironment::new(
            None,
            None,
            Some(home.to_path_buf()),
        ),
    )
    .unwrap()
}

fn run_git(cwd: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}
```

Add the Doctor cases:

```rust
#[test]
fn project_identity_passes_for_git_origin_without_manifest() {
    let root = tempfile::tempdir().unwrap();
    run_git(root.path(), &["init", "--quiet"]);
    run_git(
        root.path(),
        &["remote", "add", "origin", "https://github.com/owner/repo.git"],
    );
    let paths = fixture_paths(root.path());
    let check = check_project_identity_at(root.path(), &paths);
    assert_eq!(check.status, CheckStatus::Pass);
    assert_eq!(check.message, "stable project identity loaded");
    assert_eq!(check.fix_hint, None);
}

#[test]
fn project_identity_advises_init_without_manifest_or_origin() {
    let root = tempfile::tempdir().unwrap();
    let paths = fixture_paths(root.path());
    let check = check_project_identity_at(root.path(), &paths);
    assert_eq!(check.status, CheckStatus::Advisory);
    assert!(check.message.contains("memory is temporary"));
    assert!(check.fix_hint.unwrap().contains("coding-brain init"));
}
```

- [ ] **Step 2: Run the focused Doctor tests and confirm failure**

Run:

```bash
cargo test project_identity_passes_for_git_origin_without_manifest
```

Expected: compilation fails because `check_project_identity_at` and the test fixtures do not exist.

- [ ] **Step 3: Implement the Doctor helper and neutral stable message**

Move the current `ProjectIdentity::load` match into
`check_project_identity_at`. Have `check_project_identity()` resolve paths and
the current directory, then return `check_project_identity_at(&cwd, &paths)`.
Use the exact messages in Step 1 so Git-backed identity is not described as a
manifest UUID.

- [ ] **Step 4: Run focused and full quality gates**

Run:

```bash
cargo test project_identity_
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo build
```

Expected: every command exits successfully with no formatting diff or Clippy warnings.

- [ ] **Step 5: Review the final jj diff**

Before handoff, always run:

```bash
jj --no-pager st
jj --no-pager diff --git
jj --no-pager log -r '@|@-' --no-graph
```

Expected: the final stack contains only the approved design, implementation
plan, project identity implementation, focused tests, and Doctor wording. Do
not commit, rebase, or push unless the user explicitly authorizes it.

## Spec Coverage Map

- Manifest precedence and malformed-manifest behavior: Task 2.
- Git-root and nested-directory consistency: Task 2.
- Bounded HTTPS, SSH, and SCP normalization plus local/file rejection: Task 1.
- Versioned UUIDv5 and credential containment: Task 1.
- Temporary fallback and no activity migration: Task 2; no persistence rewrite is added.
- Doctor behavior: Task 3.
- Unchanged activity schema and downstream `ProjectId` interface: Tasks 1 and 2 preserve the enum and public signatures.
- Full regression verification: Task 3.

## Stress Test Results: Git-Default Project Identity Plan

### Resolved Decisions

- Only network remotes create durable automatic identity; local and `file`
  remotes retain the path-derived temporary fallback.
- Git-root manifests are the sole explicit override inside a Git repository,
  so session subdirectories cannot fragment one repository's identity.
- Canonicalization removes userinfo, query strings, fragments, trailing
  separators, and `.git`, while preserving ports, path case, and percent
  encoding.
- DNS aliases, SSH aliases, and hosting-provider redirects remain literal and
  require no network or machine-specific resolution.
- Hosting-slug reuse intentionally reuses identity unless a manifest overrides
  it; a project ID is not proof of repository ownership.
- The initial implementation performs two read-only local Git calls per hook
  and relies on the existing hook deadline for pathological stalls.
- Temporary activity is not rewritten. Restoring a prior canonical `origin`
  restores its deterministic UUID, and a manifest can pin future identity.
- UUIDv5 remains suitable for a non-authorization identifier; tests prove
  credential-bearing remotes persist only an opaque UUID.
- Parser tests and temporary real-Git repositories jointly cover pure behavior
  and Git integration without modifying global Git configuration.
- The canonical Git-root path hash remains a temporary fallback, never a
  durable or raw-path identifier.
- Network URL schemes are allowlisted. URL-form bracketed IPv6 is supported;
  ambiguous SCP-style bracketed IPv6 and unknown schemes are rejected.

### Changes Made

- Changed local and `file` remotes from stable UUID sources to temporary
  fallback cases.
- Added query and fragment removal, a network-scheme allowlist, and explicit
  bracketed-IPv6 behavior.
- Added credential-containment assertions at the persisted `ProjectId` layer.
- Documented hosting-slug reuse, the non-authorization boundary, and the
  measured-performance prerequisite for caching or custom timeouts.

### Deferred / Parking Lot

- Measure Git discovery latency in real permission-hook workloads before
  adding caching or subprocess-timeout infrastructure.
- Add provider-specific canonicalization only if concrete same-repository
  aliases cause user-visible fragmentation.

### Confidence Assessment

- Overall: High
- Areas of concern: intentional hosting-slug reuse and provider-neutral path
  semantics can still join or split identity in rare cases; manifests remain
  the explicit escape hatch.
