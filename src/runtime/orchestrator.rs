//! Bind `Orchestrator` to the binary's mailbox + coord stores.
//!
//! Each `deliver_*` method resolves `SessionSnapshot` inputs to the live
//! `CodexSession` values the brain mailbox and coord interrupt bus actually
//! need, via a fresh discovery scan per call.

use codexctl_core::discovery;
use codexctl_core::runtime::{Orchestrator, SessionSnapshot};
use codexctl_core::session::CodexSession;

use crate::brain;

pub struct LiveOrchestrator;

impl Orchestrator for LiveOrchestrator {
    fn deliver_mailbox(&self, snapshots: &[SessionSnapshot]) -> Vec<(u32, String)> {
        let live = resolve_live(snapshots);
        brain::mailbox::deliver_pending(&live)
    }

    fn deliver_interrupts(&self, snapshots: &[SessionSnapshot]) -> Vec<(String, String)> {
        #[cfg(feature = "coord")]
        {
            let Ok(conn) = crate::coord::store::open() else {
                return Vec::new();
            };
            let live = resolve_live(snapshots);
            crate::coord::interrupt_bus::deliver_pending(&conn, &live)
        }
        #[cfg(not(feature = "coord"))]
        {
            let _ = snapshots;
            Vec::new()
        }
    }

    fn expire_stale(&self) {
        #[cfg(feature = "coord")]
        {
            if let Ok(conn) = crate::coord::store::open() {
                let _ = crate::coord::store::expire_stale_leases(&conn);
                let _ = crate::coord::store::expire_stale_interrupts(&conn);
            }
        }
    }
}

/// Re-fetch the live `CodexSession` set and intersect with the snapshots
/// the caller passed. Sessions that exited between the snapshot and the
/// call are silently dropped — the orchestration layer is best-effort.
fn resolve_live(snapshots: &[SessionSnapshot]) -> Vec<CodexSession> {
    let mut live = discovery::scan_sessions();
    discovery::resolve_jsonl_paths(&mut live);
    let mut by_id: std::collections::HashMap<String, CodexSession> = live
        .into_iter()
        .map(|s| (s.session_id.clone(), s))
        .collect();
    snapshots
        .iter()
        .filter_map(|snap| by_id.remove(snap.session_id.as_str()))
        .collect()
}
