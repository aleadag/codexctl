#![allow(dead_code)]

use super::adapter::*;
use codexctl_core::{discovery, monitor};

/// Codex adapter backed by Codex JSONL transcript discovery.
pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    fn family(&self) -> AgentFamily {
        AgentFamily::Codex
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            discover_sessions: true,
            monitor_state: true,
            send_input: false,
            deliver_interrupt: false,
            request_checkpoint: false,
            request_compaction: false,
            pause: false,
            resume: false,
            terminate: false,
        }
    }

    fn discover_sessions(&self) -> Vec<AgentIdentity> {
        discovery::scan_sessions()
            .into_iter()
            .map(|s| AgentIdentity {
                agent_family: "codex".into(),
                session_id: s.session_id,
                cwd: s.cwd,
                branch: None,
                pid: Some(s.pid),
            })
            .collect()
    }

    fn get_state(&self, session_id: &str) -> Option<AgentState> {
        let mut session = discovery::scan_sessions()
            .into_iter()
            .find(|s| s.session_id == session_id)?;
        monitor::update_tokens(&mut session);

        let context_pressure = if session.context_max > 0 && session.context_tokens > 0 {
            Some(session.context_tokens as f64 / session.context_max as f64)
        } else {
            None
        };
        let cost_usd = session.usage_metrics_available.then_some(session.cost_usd);

        Some(AgentState {
            status: session.status.to_string(),
            context_pressure,
            pending_tool: session.pending_tool_name,
            last_output: None,
            cost_usd,
        })
    }

    fn send_input(&self, _session_id: &str, _text: &str) -> Result<(), String> {
        Err("Codex adapter: send_input not yet implemented".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_adapter_family() {
        let adapter = CodexAdapter;
        assert_eq!(adapter.family(), AgentFamily::Codex);
    }

    #[test]
    fn codex_adapter_minimal_capabilities() {
        let adapter = CodexAdapter;
        let caps = adapter.capabilities();
        assert!(caps.discover_sessions);
        assert!(caps.monitor_state);
        assert!(!caps.send_input);
        assert!(!caps.terminate);
        assert_eq!(caps.count(), 2);
    }

    #[test]
    fn codex_adapter_ignores_history_without_live_processes() {
        let dir = tempfile::tempdir().unwrap();
        let codex_home = dir.path().join(".codex");
        let jsonl_path = codex_home
            .join("sessions")
            .join("2026")
            .join("06")
            .join("11")
            .join("rollout-2026-06-11T20-33-34-019eb6ac-6d30-7301-885d-ff4d354c0116.jsonl");
        std::fs::create_dir_all(jsonl_path.parent().unwrap()).unwrap();
        std::fs::write(
            &jsonl_path,
            include_str!("../../tests/fixtures/codex-session-meta.json"),
        )
        .unwrap();

        unsafe {
            std::env::set_var("CODEXCTL_CODEX_HOME", &codex_home);
            std::env::set_var("CODEXCTL_DISABLE_PROCESS_DISCOVERY", "1");
        }
        let adapter = CodexAdapter;
        let sessions = adapter.discover_sessions();
        unsafe {
            std::env::remove_var("CODEXCTL_CODEX_HOME");
            std::env::remove_var("CODEXCTL_DISABLE_PROCESS_DISCOVERY");
        }

        assert!(sessions.is_empty());
    }

    #[test]
    fn codex_adapter_send_input_returns_err() {
        let adapter = CodexAdapter;
        assert!(adapter.send_input("sess", "hello").is_err());
    }
}
