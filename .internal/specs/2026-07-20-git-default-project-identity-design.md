# Git-Default Project Identity Design

## Context

Coding Brain currently requires `.coding-brain/project.toml` before a project
has durable identity. Without that manifest, `ProjectIdentity::load` hashes the
canonical working-directory path into a machine-local temporary ID, so another
clone or worktree becomes a different project. Doctor then asks the user to run
`coding-brain init` in each project.

Git already provides a useful default boundary. Clones that refer to the same
canonical remote should share Brain activity and future project memory without
requiring a tracked identity file. A manifest is still useful when users need
an explicit identity that does not follow the remote.

This design replaces the manifest-only durable identity requirement in the
project-identity section of the Brain-primary TUI design. It does not change
the project display-label fallback.

## Decision

Resolve project identity in this order:

1. Resolve the Git top-level directory for the supplied working directory.
2. If `<git-root>/.coding-brain/project.toml` exists and is valid, use its UUID.
3. Otherwise, read `origin` and derive a stable UUID from its canonical remote.
4. If the directory is not in a Git worktree, Git cannot be executed, or
   `origin` is absent or unusable, retain the current path-derived temporary ID.

Outside a Git worktree, manifest lookup and temporary identity continue to use
the supplied working directory. `coding-brain init` remains available to create
an explicit manifest override, but it is no longer required for a durable
identity when a usable Git remote exists.

`ProjectId::Stable` continues to represent both manifest and Git-derived IDs.
The serialized enum shape and activity schema do not change.

## Canonical Remote Fingerprint

Read the configured fetch URL with `git remote get-url origin`. Normalize
common URL and SCP-style forms so, for example,
`git@github.com:owner/repository.git` and
`https://github.com/owner/repository.git` produce the same canonical value:

- remove the transport scheme and user information;
- accept only Git's network schemes: `https`, `http`, `ssh`, `git`, `ftp`, and
  `ftps`;
- lowercase the host while preserving repository-path case;
- retain an explicit port;
- remove query strings and fragments, which may contain transient credentials;
- remove leading and trailing path separators and a final `.git` suffix;
- represent the result as `host[:port]/path`;
- reject local and `file` remotes because their paths are machine-specific.

Repository-path case and percent encoding remain significant. Coding Brain does
not resolve DNS aliases, SSH host aliases, or hosting-provider redirects, which
would make identity depend on machine configuration or network access.
Bracketed IPv6 is supported in standard URL form; ambiguous SCP-style bracketed
IPv6 and unknown schemes fall back to temporary identity.

Reject a normalized network value with no host or repository path. The raw
remote URL must never be persisted or logged because it may contain
credentials.

Generate the project ID as UUIDv5 in a fixed Coding Brain namespace from the
versioned input `git-remote:v1:<canonical-remote>`. The version marker prevents
a later normalization revision from silently reusing the current namespace.
Using the same UUID-shaped value as manifests keeps downstream persistence and
serialization unchanged.

## Data Flow

1. Permission and lifecycle hooks call `ProjectIdentity::load` with the session
   working directory as they do today.
2. Project resolution obtains the Git root once, then uses that root for the
   manifest, remote, and temporary-path fallback.
3. A valid manifest returns immediately and does not inspect the remote.
4. A missing manifest allows canonical remote discovery and UUIDv5 derivation.
5. Activity producers persist the resulting `ProjectId` without knowing which
   durable source produced it.
6. Doctor uses the same resolver, so a Git-derived stable identity passes the
   project-identity check.

The Git discovery and normalization stay in `coding-brain-core::project` rather
than reusing UI-oriented Git context code. This keeps identity behavior shared
by every producer and avoids adding a binary-crate dependency to core.

Resolution performs two read-only local Git commands and never contacts a
remote. The hook process is short-lived, so it does not add a process-local
cache; the existing hook deadline remains the bound for pathological local Git
or filesystem stalls. Further performance tuning is deferred until measurement
shows these reads are material.

## Compatibility and Failure Behavior

A valid tracked manifest remains authoritative, preserving every existing
explicit project UUID. A malformed or unsupported manifest remains an error;
Coding Brain must not hide it by falling back to the remote.

Git lookup failures are non-fatal and fall back to temporary identity. This
includes a missing executable, a non-worktree directory, an absent `origin`, a
local or `file` remote, an invalid URL, and an unsuccessful Git command. The
fallback remains explicitly non-durable, so Doctor can still recommend
`coding-brain init` in those cases.

Equivalent SSH and HTTPS forms preserve identity. Changing `origin` to a
different canonical repository intentionally changes identity unless a
manifest pins it. Existing activity recorded under a temporary path ID remains
historical; new activity uses the Git-derived stable ID. The resolver does not
rewrite append-only activity or guess that old and new IDs are equivalent.

The canonical remote is a user-declared identity, not proof of repository
ownership. If a hosting slug is deleted, transferred, or recreated, the same
remote intentionally reuses the prior project identity unless a manifest
overrides it. Project identity must not become an authorization boundary by
itself; deterministic safety controls cannot trust a remote-derived ID as
evidence of ownership.

## Non-goals

- Discover an identity from arbitrary remotes when `origin` is absent.
- Treat a machine-local remote path as durable cross-clone identity.
- Contact a Git hosting service or resolve server-side repository IDs.
- Detect hosting-slug deletion, transfer, or reuse without an explicit manifest.
- Treat root commits, forks, or shared object history as project identity.
- Migrate or rewrite activities stored under a temporary ID.
- Remove explicit project manifests or the `coding-brain init` workflow.
- Change project display labels.
- Add caching or dedicated subprocess-timeout machinery without measured need.

## Testing

- A repository root and its subdirectories resolve to the same identity.
- Separate clones with equivalent HTTPS, SSH, and SCP-style `origin` URLs
  resolve to the same stable UUID.
- Credential-bearing and credential-free forms produce the same UUID, and the
  persisted ID contains no credential or remote-path text.
- Host case, credentials, trailing separators, and `.git` do not affect the
  fingerprint; query strings and fragments are ignored; repository-path case,
  percent encoding, and explicit ports remain significant.
- Known network schemes and URL-form bracketed IPv6 normalize predictably;
  unknown schemes and SCP-style bracketed IPv6 retain temporary identity.
- Local and `file` remotes retain temporary identity.
- A valid manifest overrides the remote and preserves its UUID.
- A malformed manifest returns the existing manifest error.
- A non-Git directory, missing `origin`, invalid remote, or failed Git command
  produces a temporary ID without failing hook evaluation.
- Doctor accepts manifest-backed and Git-backed stable identities and advises
  `coding-brain init` only for temporary identity.
- Existing project, hook, activity, and Doctor tests continue to pass.
