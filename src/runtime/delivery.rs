//! Bind local brain mailbox delivery to the runtime contract.

use codexctl_core::discovery;
use codexctl_core::runtime::{BrainDelivery, SessionSnapshot};
use codexctl_core::session::CodexSession;

pub struct LiveBrainDelivery;

impl BrainDelivery for LiveBrainDelivery {
    fn deliver_mailbox(&self, snapshots: &[SessionSnapshot]) -> Vec<(u32, String)> {
        let live = resolve_live(snapshots);
        crate::brain::mailbox::deliver_pending(&live)
    }
}

fn resolve_live(snapshots: &[SessionSnapshot]) -> Vec<CodexSession> {
    let mut live = discovery::scan_sessions();
    discovery::resolve_jsonl_paths(&mut live);
    let mut by_id: std::collections::HashMap<String, CodexSession> = live
        .into_iter()
        .map(|session| (session.session_id.clone(), session))
        .collect();
    snapshots
        .iter()
        .filter_map(|snapshot| by_id.remove(snapshot.session_id.as_str()))
        .collect()
}
