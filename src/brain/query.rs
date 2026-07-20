#![allow(dead_code)]

use serde_json::Value;

use coding_brain_core::brain_activity::redact_activity_text;

use super::client::{self, BrainSuggestion};
use super::decisions::{
    DecisionType, adaptive_threshold, format_few_shot_examples, format_preference_summary,
    load_preferences_for_project, retrieve_similar,
};
use super::diff_digest::DiffDigest;
use crate::config::BrainConfig;

const MAX_DYNAMIC_PROMPT_BYTES: usize = 48 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct BrainDecisionRequest {
    pub project: String,
    pub tool_name: String,
    pub tool_input: String,
    pub diff_digest: Option<DiffDigest>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BrainDecision {
    pub action: String,
    pub message: Option<String>,
    pub reasoning: String,
    pub confidence: f64,
    pub source: &'static str,
    pub threshold: Option<f64>,
    pub below_threshold: Option<bool>,
    pub diff_digest: Option<Value>,
}

pub(crate) fn evaluate(
    request: &BrainDecisionRequest,
    brain_config: &BrainConfig,
    gate_mode: &str,
) -> BrainDecision {
    evaluate_with(request, brain_config, gate_mode, client::infer)
}

pub(crate) fn evaluate_with<F>(
    request: &BrainDecisionRequest,
    brain_config: &BrainConfig,
    gate_mode: &str,
    infer: F,
) -> BrainDecision
where
    F: FnOnce(&BrainConfig, &str) -> Result<BrainSuggestion, String>,
{
    if gate_mode == "off" {
        return BrainDecision {
            action: "abstain".into(),
            message: None,
            reasoning: "Brain gate mode is off".into(),
            confidence: 0.0,
            source: "gate",
            threshold: None,
            below_threshold: None,
            diff_digest: None,
        };
    }

    let tool_display = if request.tool_input.is_empty() {
        request.tool_name.clone()
    } else {
        format!("{}: {}", request.tool_name, request.tool_input)
    };
    let session_summary = format!(
        "Project: {} | Status: Needs Input | Pending tool: {} | Command: {}",
        request.project, request.tool_name, request.tool_input
    );
    let diff_section = request
        .diff_digest
        .as_ref()
        .map(|digest| format!("\n\n## Proposed change\n{}", digest.format_for_prompt()))
        .unwrap_or_default();
    let pref_section = load_preferences_for_project(&request.project)
        .map(|prefs| {
            format!(
                "\n\n## Learned Preferences\n{}",
                format_preference_summary(&prefs)
            )
        })
        .unwrap_or_default();
    let similar = retrieve_similar(
        Some(&request.tool_name),
        &request.project,
        brain_config.few_shot_count.min(5),
        Some(DecisionType::Session),
    );
    let few_shot_section = if similar.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n## Past Decisions\n{}",
            format_few_shot_examples(&similar)
        )
    };
    let dynamic_context = redact_activity_text(&format!(
        "## Session\n{session_summary}{diff_section}{pref_section}{few_shot_section}\n\
         \n## Decision subject\nThe session wants to run [{tool_display}]."
    ));
    let dynamic_context = truncate_utf8(&dynamic_context, MAX_DYNAMIC_PROMPT_BYTES);
    let prompt = format!(
        "You are a session supervisor deciding whether to approve or deny a tool call.\n\n\
         {dynamic_context}\n\n\
         Weigh the proposed change against the learned preferences and past \
         decisions. Be more cautious when sensitive paths or risky tokens are \
         present. Respond with JSON: {{\"action\": \"approve\"|\"deny\", \
         \"message\": \"...\", \"reasoning\": \"...\", \"confidence\": 0.0-1.0}}"
    );

    match infer(brain_config, &prompt) {
        Ok(suggestion) => {
            let threshold = adaptive_threshold(Some(&request.tool_name)).unwrap_or(0.6);
            BrainDecision {
                action: suggestion.action.label().into(),
                message: suggestion.message,
                reasoning: suggestion.reasoning,
                confidence: suggestion.confidence,
                source: "brain",
                threshold: Some(threshold),
                below_threshold: Some(suggestion.confidence < threshold),
                diff_digest: request.diff_digest.as_ref().map(DiffDigest::to_log_json),
            }
        }
        Err(error) => BrainDecision {
            action: "abstain".into(),
            message: None,
            reasoning: format!("Brain query failed: {error}"),
            confidence: 0.0,
            source: "error",
            threshold: None,
            below_threshold: None,
            diff_digest: None,
        },
    }
}

fn truncate_utf8(value: &str, limit: usize) -> &str {
    if value.len() <= limit {
        return value;
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::client::BrainSuggestion;
    use crate::config::BrainConfig;
    use crate::rules::RuleAction;

    fn request() -> BrainDecisionRequest {
        BrainDecisionRequest {
            project: "codexctl".into(),
            tool_name: "Bash".into(),
            tool_input: "cargo test".into(),
            diff_digest: None,
        }
    }

    fn suggestion(action: RuleAction, confidence: f64) -> BrainSuggestion {
        BrainSuggestion {
            action,
            message: Some("brain message".into()),
            reasoning: "brain reasoning".into(),
            confidence,
            suggested_at: 0,
        }
    }

    #[test]
    fn confident_approve_uses_brain_result() {
        let decision = evaluate_with(&request(), &BrainConfig::default(), "on", |_, _| {
            Ok(suggestion(RuleAction::Approve, 0.9))
        });

        assert_eq!(decision.action, "approve");
        assert_eq!(decision.message.as_deref(), Some("brain message"));
        assert_eq!(decision.reasoning, "brain reasoning");
        assert_eq!(decision.confidence, 0.9);
        assert_eq!(decision.source, "brain");
        assert_eq!(decision.threshold, Some(0.6));
        assert_eq!(decision.below_threshold, Some(false));
    }

    #[test]
    fn confident_deny_uses_brain_result() {
        let decision = evaluate_with(&request(), &BrainConfig::default(), "on", |_, _| {
            Ok(suggestion(RuleAction::Deny, 0.8))
        });

        assert_eq!(decision.action, "deny");
        assert_eq!(decision.source, "brain");
        assert_eq!(decision.below_threshold, Some(false));
    }

    #[test]
    fn low_confidence_is_marked_below_threshold() {
        let decision = evaluate_with(&request(), &BrainConfig::default(), "on", |_, _| {
            Ok(suggestion(RuleAction::Approve, 0.59))
        });

        assert_eq!(decision.action, "approve");
        assert_eq!(decision.threshold, Some(0.6));
        assert_eq!(decision.below_threshold, Some(true));
    }

    #[test]
    fn inference_failure_abstains() {
        let decision = evaluate_with(&request(), &BrainConfig::default(), "on", |_, _| {
            Err("endpoint unavailable".into())
        });

        assert_eq!(decision.action, "abstain");
        assert_eq!(
            decision.reasoning,
            "Brain query failed: endpoint unavailable"
        );
        assert_eq!(decision.confidence, 0.0);
        assert_eq!(decision.source, "error");
        assert_eq!(decision.threshold, None);
        assert_eq!(decision.below_threshold, None);
    }

    #[test]
    fn model_context_is_redacted_and_bounded() {
        let mut request = request();
        request.tool_input = format!("TOKEN=private-value {}", "x".repeat(80 * 1024));

        let decision = evaluate_with(&request, &BrainConfig::default(), "on", |_, prompt| {
            assert!(!prompt.contains("private-value"));
            assert!(
                prompt.len() <= 50 * 1024,
                "prompt was {} bytes",
                prompt.len()
            );
            Ok(suggestion(RuleAction::Approve, 0.9))
        });

        assert_eq!(decision.action, "approve");
    }

    #[test]
    fn gate_off_abstains_without_inference() {
        let decision = evaluate_with(&request(), &BrainConfig::default(), "off", |_, _| {
            panic!("gate-off evaluation must not call inference")
        });

        assert_eq!(decision.action, "abstain");
        assert_eq!(decision.reasoning, "Brain gate mode is off");
        assert_eq!(decision.confidence, 0.0);
        assert_eq!(decision.source, "gate");
    }
}
