use crate::session::{CodexSession, SessionStatus};

use super::{LifecycleEventName, ProjectedStatus, StoreCondition};

const SHORT_LEASE_MS: u64 = 30_000;
const LONG_LEASE_MS: u64 = 10 * 60 * 1_000;
const MAX_FUTURE_SKEW_MS: u64 = 5_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscriptSemantic {
    Progress,
    Complete,
    ExplicitInput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranscriptEvidence {
    pub semantic: TranscriptSemantic,
    pub observed_at_ms: Option<u64>,
}

impl TranscriptEvidence {
    pub fn progress(observed_at_ms: Option<u64>) -> Self {
        Self {
            semantic: TranscriptSemantic::Progress,
            observed_at_ms,
        }
    }

    pub fn complete(observed_at_ms: Option<u64>) -> Self {
        Self {
            semantic: TranscriptSemantic::Complete,
            observed_at_ms,
        }
    }

    pub fn explicit_input(observed_at_ms: Option<u64>) -> Self {
        Self {
            semantic: TranscriptSemantic::ExplicitInput,
            observed_at_ms,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LifecycleEvidence {
    pub projected_status: ProjectedStatus,
    pub status_event: LifecycleEventName,
    pub status_received_at_ms: u64,
    pub latest_event: LifecycleEventName,
    pub latest_received_at_ms: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LifecycleDiagnostic {
    pub available: bool,
    pub event: Option<LifecycleEventName>,
    pub age_ms: Option<u64>,
    pub contributing: bool,
    pub ignored_reason: Option<String>,
    pub store_condition: Option<StoreCondition>,
}

pub fn contributing_status(session: &mut CodexSession, now_ms: u64) -> Option<SessionStatus> {
    let Some(evidence) = session.lifecycle_evidence else {
        session.lifecycle_diagnostic.contributing = false;
        return None;
    };
    session.lifecycle_diagnostic.event = Some(evidence.status_event);
    if evidence.status_received_at_ms > now_ms.saturating_add(MAX_FUTURE_SKEW_MS) {
        session.lifecycle_diagnostic.age_ms = None;
        session.lifecycle_diagnostic.contributing = false;
        session.lifecycle_diagnostic.ignored_reason = Some("future lifecycle timestamp".into());
        return None;
    }

    let age_ms = now_ms.saturating_sub(evidence.status_received_at_ms);
    session.lifecycle_diagnostic.age_ms = Some(age_ms);
    let lease_ms = match (evidence.projected_status, evidence.status_event) {
        (ProjectedStatus::NeedsInput, _) => LONG_LEASE_MS,
        (_, LifecycleEventName::UserPromptSubmit)
        | (_, LifecycleEventName::PermissionRequest)
        | (_, LifecycleEventName::PostToolUse) => SHORT_LEASE_MS,
        _ => LONG_LEASE_MS,
    };
    if age_ms >= lease_ms {
        session.lifecycle_diagnostic.contributing = false;
        session.lifecycle_diagnostic.ignored_reason = Some("lifecycle evidence expired".into());
        return None;
    }

    let superseded_by_hook = evidence.latest_received_at_ms > evidence.status_received_at_ms
        && matches!(
            (evidence.status_event, evidence.latest_event),
            (
                LifecycleEventName::SubagentStart,
                LifecycleEventName::SubagentStop
            ) | (
                LifecycleEventName::PreToolUse,
                LifecycleEventName::PostToolUse
            ) | (LifecycleEventName::PreToolUse, LifecycleEventName::Stop)
                | (LifecycleEventName::Stop, _)
        );
    if superseded_by_hook {
        session.lifecycle_diagnostic.contributing = false;
        session.lifecycle_diagnostic.ignored_reason = Some("superseded by lifecycle event".into());
        return None;
    }

    let invalidated_by_transcript = session.transcript_evidence.is_some_and(|transcript| {
        let Some(observed_at_ms) = transcript.observed_at_ms else {
            return false;
        };
        if observed_at_ms > now_ms.saturating_add(MAX_FUTURE_SKEW_MS)
            || observed_at_ms <= evidence.status_received_at_ms
        {
            return false;
        }
        match transcript.semantic {
            TranscriptSemantic::Complete => {
                evidence.projected_status == ProjectedStatus::Processing
            }
            TranscriptSemantic::Progress => {
                evidence.projected_status == ProjectedStatus::Idle
                    || matches!(
                        evidence.status_event,
                        LifecycleEventName::PreToolUse | LifecycleEventName::SubagentStart
                    )
            }
            TranscriptSemantic::ExplicitInput => true,
        }
    });
    if invalidated_by_transcript {
        session.lifecycle_diagnostic.contributing = false;
        session.lifecycle_diagnostic.ignored_reason = Some("superseded by transcript".into());
        return None;
    }

    let status = match evidence.projected_status {
        ProjectedStatus::Processing => SessionStatus::Processing,
        ProjectedStatus::NeedsInput => SessionStatus::NeedsInput,
        ProjectedStatus::Idle => SessionStatus::Idle,
    };
    session.lifecycle_diagnostic.contributing = true;
    session.lifecycle_diagnostic.ignored_reason = None;
    Some(status)
}

#[cfg(test)]
mod tests {
    use crate::session::{
        ApprovalEvidence, ApprovalObservation, CodexSession, RawSession, SessionStatus,
    };
    use crate::terminals::Terminal;

    use super::*;

    fn session_with_hook(
        status: ProjectedStatus,
        event: LifecycleEventName,
        received_at_ms: u64,
    ) -> CodexSession {
        let mut session = CodexSession::from_raw(RawSession {
            pid: 7,
            session_id: "session-7".into(),
            cwd: "/repo".into(),
            started_at: 0,
        });
        session.lifecycle_evidence = Some(LifecycleEvidence {
            projected_status: status,
            status_event: event,
            status_received_at_ms: received_at_ms,
            latest_event: event,
            latest_received_at_ms: received_at_ms,
        });
        session.lifecycle_diagnostic.available = true;
        session
    }

    #[test]
    fn leases_expire_exactly_at_their_boundary() {
        let cases = [
            (
                LifecycleEventName::UserPromptSubmit,
                ProjectedStatus::Processing,
                30_000,
            ),
            (
                LifecycleEventName::PermissionRequest,
                ProjectedStatus::Processing,
                30_000,
            ),
            (
                LifecycleEventName::PostToolUse,
                ProjectedStatus::Processing,
                30_000,
            ),
            (
                LifecycleEventName::PreToolUse,
                ProjectedStatus::Processing,
                600_000,
            ),
            (
                LifecycleEventName::SubagentStart,
                ProjectedStatus::Processing,
                600_000,
            ),
            (
                LifecycleEventName::PermissionRequest,
                ProjectedStatus::NeedsInput,
                600_000,
            ),
            (LifecycleEventName::Stop, ProjectedStatus::Idle, 600_000),
        ];
        for (event, status, lease) in cases {
            let mut session = session_with_hook(status, event, 1_000);
            assert!(contributing_status(&mut session, 1_000 + lease - 1).is_some());
            assert_eq!(contributing_status(&mut session, 1_000 + lease), None);
        }
    }

    #[test]
    fn strictly_newer_transcript_semantics_invalidate_only_conflicting_hook_status() {
        let mut stopped = session_with_hook(ProjectedStatus::Idle, LifecycleEventName::Stop, 1_000);
        stopped.transcript_evidence = Some(TranscriptEvidence::progress(Some(2_000)));
        assert_eq!(contributing_status(&mut stopped, 3_000), None);

        let mut processing = session_with_hook(
            ProjectedStatus::Processing,
            LifecycleEventName::UserPromptSubmit,
            1_000,
        );
        processing.transcript_evidence = Some(TranscriptEvidence::complete(Some(2_000)));
        assert_eq!(contributing_status(&mut processing, 3_000), None);

        let mut matching_stop =
            session_with_hook(ProjectedStatus::Idle, LifecycleEventName::Stop, 1_000);
        matching_stop.transcript_evidence = Some(TranscriptEvidence::complete(Some(2_000)));
        assert_eq!(
            contributing_status(&mut matching_stop, 3_000),
            Some(SessionStatus::Idle)
        );
    }

    #[test]
    fn equal_missing_and_future_transcript_timestamps_do_not_invalidate() {
        for observed_at_ms in [Some(500), Some(1_000), None, Some(10_001)] {
            let mut session =
                session_with_hook(ProjectedStatus::Idle, LifecycleEventName::Stop, 1_000);
            session.transcript_evidence = Some(TranscriptEvidence::progress(observed_at_ms));
            assert_eq!(
                contributing_status(&mut session, 5_000),
                Some(SessionStatus::Idle)
            );
        }
    }

    #[test]
    fn future_hook_timestamp_does_not_contribute() {
        let mut session = session_with_hook(
            ProjectedStatus::Processing,
            LifecycleEventName::PreToolUse,
            10_001,
        );
        assert_eq!(contributing_status(&mut session, 5_000), None);
        assert!(!session.lifecycle_diagnostic.contributing);
    }

    #[test]
    fn reconciliation_does_not_mutate_actionable_fields() {
        let mut session = session_with_hook(
            ProjectedStatus::Processing,
            LifecycleEventName::PreToolUse,
            1_000,
        );
        session.pending_tool_name = Some("exec_command".into());
        session.pending_tool_call_id = Some("call-7".into());
        session.pending_tool_input = Some("cargo test".into());
        session.pending_file_path = Some("src/main.rs".into());
        session.approval = ApprovalObservation::Confirmed(ApprovalEvidence {
            session_id: "session-7".into(),
            tty: "pts/7".into(),
            call_id: "call-7".into(),
            tool: "exec_command".into(),
            command: "cargo test".into(),
            backend: Terminal::Tmux,
            target: "main:1.0".into(),
            prompt_pattern_version: 1,
            prompt_fingerprint: 42,
        });
        let actionable = (
            session.pending_tool_name.clone(),
            session.pending_tool_call_id.clone(),
            session.pending_tool_input.clone(),
            session.pending_file_path.clone(),
            session.approval.clone(),
        );

        assert_eq!(
            contributing_status(&mut session, 2_000),
            Some(SessionStatus::Processing)
        );
        assert_eq!(
            (
                session.pending_tool_name,
                session.pending_tool_call_id,
                session.pending_tool_input,
                session.pending_file_path,
                session.approval,
            ),
            actionable
        );
    }
}
