//! Coding Brain's terminal UI.
//!
//! Owns the Brain application state, rendering, and terminal suspension used
//! to switch to a live Codex session. Shared contracts live in `codexctl-core`.

#![allow(unknown_lints)]
#![allow(
    clippy::collapsible_if,
    clippy::manual_is_multiple_of,
    clippy::io_other_error,
    clippy::too_many_arguments
)]

pub mod brain_app;
pub mod terminal_suspend;
pub mod ui;
