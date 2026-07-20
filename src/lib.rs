#![allow(unknown_lints)]
#![allow(
    clippy::collapsible_if,
    clippy::manual_is_multiple_of,
    clippy::io_other_error
)]

// ---- Foundational modules now living in coding-brain-core (epic #279, PRs for
// #273 + #276 + the hooks/launch/skills move below).
//
// Re-exported under their original names so existing `crate::session::*`
// (etc.) paths keep resolving without rewriting 50+ import sites. Once #275
// extracts the TUI into its own crate it will depend on coding-brain-core
// directly and these aliases can disappear.
pub use coding_brain_core::{
    discovery, health, helpers, history, hooks, logger, models, monitor, process, rules, session,
    skills, terminals, theme, transcript,
};
pub use coding_brain_tui::{brain_app, ui};
pub mod config;

pub mod brain;
pub mod doctor;
pub mod init;
mod lifecycle_hook;
pub mod runtime;
