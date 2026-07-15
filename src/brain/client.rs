#![allow(dead_code)]

use std::process::Command;

use crate::config::BrainConfig;
use crate::rules::RuleAction;

/// The brain's suggestion for a session, parsed from the LLM response.
#[derive(Debug, Clone)]
pub struct BrainSuggestion {
    pub action: RuleAction,
    pub message: Option<String>,
    pub reasoning: String,
    pub confidence: f64,
    /// Epoch seconds when this suggestion was created.
    /// Used by time-to-correct analysis to measure user reaction latency.
    pub suggested_at: u64,
}

/// Call the local LLM endpoint via curl and parse the response.
pub fn infer(config: &BrainConfig, prompt: &str) -> Result<BrainSuggestion, String> {
    let is_openai = is_openai_compatible(&config.endpoint);

    let payload = if is_openai {
        // OpenAI-compatible format (llama.cpp, vLLM, LM Studio)
        serde_json::json!({
            "model": config.model,
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "response_format": {"type": "json_object"},
            "stream": false,
        })
    } else {
        // Ollama /api/generate format (default)
        serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "stream": false,
            "format": "json",
        })
    };

    let body = serde_json::to_string(&payload).map_err(|e| format!("json error: {e}"))?;
    let timeout_secs = (config.timeout_ms / 1000).max(1);

    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
            "--max-time",
            &timeout_secs.to_string(),
            &config.endpoint,
        ])
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl error (exit {}): {stderr}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if is_openai {
        parse_openai_response(&stdout)
    } else {
        parse_ollama_response(&stdout)
    }
}

/// Detect if the endpoint is OpenAI-compatible based on URL path.
fn is_openai_compatible(endpoint: &str) -> bool {
    endpoint.contains("/v1/chat") || endpoint.contains("/v1/completions")
}

/// Summarize source session output for routing to a target session.
/// Returns a compact summary that won't bloat the target's context.
pub fn summarize_for_routing(
    config: &BrainConfig,
    source_output: &str,
    source_project: &str,
    target_task: &str,
) -> Result<String, String> {
    let template = super::prompts::load(super::prompts::SUMMARIZE);
    let prompt = super::prompts::expand(
        &template,
        &[
            ("source_project", source_project),
            ("target_task", target_task),
            ("source_output", source_output),
        ],
    );

    let response = call_llm(config, &prompt)?;
    Ok(response.trim().to_string())
}

/// Make an LLM API call, auto-detecting ollama vs OpenAI format from the endpoint URL.
pub fn complete(config: &BrainConfig, prompt: &str) -> Result<String, String> {
    call_llm(config, prompt)
}

/// Make an LLM API call, auto-detecting ollama vs OpenAI format from the endpoint URL.
fn call_llm(config: &BrainConfig, prompt: &str) -> Result<String, String> {
    let is_openai = is_openai_compatible(&config.endpoint);

    let payload = if is_openai {
        serde_json::json!({
            "model": config.model,
            "messages": [{"role": "user", "content": prompt}],
            "stream": false,
        })
    } else {
        serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "stream": false,
        })
    };

    let body = serde_json::to_string(&payload).map_err(|e| format!("json error: {e}"))?;
    let timeout_secs = (config.timeout_ms / 1000).max(1);

    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
            "--max-time",
            &timeout_secs.to_string(),
            &config.endpoint,
        ])
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "curl error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("invalid response: {e}"))?;

    if is_openai {
        // OpenAI: choices[0].message.content
        Ok(json
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or(&stdout)
            .to_string())
    } else {
        // Ollama: response field
        Ok(json
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or(&stdout)
            .to_string())
    }
}

/// Parse the ollama `/api/generate` response format.
fn parse_ollama_response(response: &str) -> Result<BrainSuggestion, String> {
    let json: serde_json::Value =
        serde_json::from_str(response).map_err(|e| format!("invalid JSON response: {e}"))?;

    // Ollama wraps the generated text in a "response" field
    let generated = json
        .get("response")
        .and_then(|v| v.as_str())
        .unwrap_or(response);

    parse_suggestion_json(generated)
}

/// Parse OpenAI-compatible /v1/chat/completions response.
fn parse_openai_response(response: &str) -> Result<BrainSuggestion, String> {
    let json: serde_json::Value =
        serde_json::from_str(response).map_err(|e| format!("invalid JSON response: {e}"))?;

    // OpenAI format: choices[0].message.content
    let content = json
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or(response);

    parse_suggestion_json(content)
}

/// Parse the structured JSON that the brain LLM is expected to produce.
pub fn parse_suggestion_json(text: &str) -> Result<BrainSuggestion, String> {
    // The LLM should produce JSON like:
    // {"action": "approve", "message": null, "reasoning": "safe command", "confidence": 0.95}
    let json: serde_json::Value =
        serde_json::from_str(text.trim()).map_err(|e| format!("invalid suggestion JSON: {e}"))?;

    let action_str = json
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or("missing 'action' field")?;

    let action = if action_str == "route" {
        let target_pid = json
            .get("target_pid")
            .and_then(|v| v.as_u64())
            .ok_or("route action requires 'target_pid' field")?;
        let target_pid =
            u32::try_from(target_pid).map_err(|_| "route action 'target_pid' exceeds u32 range")?;
        RuleAction::Route { target_pid }
    } else if action_str == "spawn" {
        let prompt = json
            .get("spawn_prompt")
            .and_then(|v| v.as_str())
            .ok_or("spawn action requires 'spawn_prompt' field")?
            .to_string();
        let cwd = json
            .get("spawn_cwd")
            .and_then(|v| v.as_str())
            .unwrap_or(".")
            .to_string();
        RuleAction::Spawn { prompt, cwd }
    } else {
        RuleAction::parse(action_str).ok_or_else(|| format!("unknown action '{action_str}'"))?
    };

    let message = json
        .get("message")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let reasoning = json
        .get("reasoning")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let confidence = json
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    Ok(BrainSuggestion {
        action,
        message,
        reasoning,
        confidence: confidence.clamp(0.0, 1.0),
        suggested_at: epoch_secs(),
    })
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_approve_suggestion() {
        let json = r#"{"action": "approve", "reasoning": "safe read command", "confidence": 0.95}"#;
        let s = parse_suggestion_json(json).unwrap();
        assert_eq!(s.action, RuleAction::Approve);
        assert_eq!(s.reasoning, "safe read command");
        assert!((s.confidence - 0.95).abs() < f64::EPSILON);
        assert!(s.message.is_none());
    }

    #[test]
    fn parse_send_suggestion() {
        let json = r#"{"action": "send", "message": "continue", "reasoning": "task in progress", "confidence": 0.8}"#;
        let s = parse_suggestion_json(json).unwrap();
        assert_eq!(s.action, RuleAction::Send);
        assert_eq!(s.message.as_deref(), Some("continue"));
    }

    #[test]
    fn parse_deny_suggestion() {
        let json = r#"{"action": "deny", "reasoning": "dangerous command", "confidence": 0.99}"#;
        let s = parse_suggestion_json(json).unwrap();
        assert_eq!(s.action, RuleAction::Deny);
    }

    #[test]
    fn parse_terminate_suggestion() {
        let json = r#"{"action": "terminate", "reasoning": "over budget", "confidence": 0.7}"#;
        let s = parse_suggestion_json(json).unwrap();
        assert_eq!(s.action, RuleAction::Terminate);
    }

    #[test]
    fn parse_route_suggestion() {
        let suggestion = parse_suggestion_json(
            r#"{"action":"route","target_pid":42,"reasoning":"better owner","confidence":0.9}"#,
        )
        .unwrap();
        assert_eq!(suggestion.action, RuleAction::Route { target_pid: 42 });
    }

    #[test]
    fn parse_route_rejects_pid_overflow() {
        let json = r#"{"action":"route","target_pid":4294967296,"reasoning":"invalid"}"#;
        assert!(parse_suggestion_json(json).is_err());
    }

    #[test]
    fn parse_spawn_suggestion() {
        let suggestion = parse_suggestion_json(
            r#"{"action":"spawn","spawn_prompt":"run tests","spawn_cwd":"/work","reasoning":"parallel","confidence":0.9}"#,
        )
        .unwrap();
        assert_eq!(
            suggestion.action,
            RuleAction::Spawn {
                prompt: "run tests".into(),
                cwd: "/work".into(),
            }
        );
    }

    #[test]
    fn parse_missing_action_fails() {
        let json = r#"{"reasoning": "no action"}"#;
        assert!(parse_suggestion_json(json).is_err());
    }

    #[test]
    fn parse_unknown_action_fails() {
        let json = r#"{"action": "dance", "reasoning": "invalid"}"#;
        assert!(parse_suggestion_json(json).is_err());
    }

    #[test]
    fn delegate_suggestion_is_rejected() {
        let json = r#"{"action":"delegate","agent":"reviewer","delegate_prompt":"review"}"#;
        assert!(parse_suggestion_json(json).is_err());
    }

    #[test]
    fn parse_confidence_clamped() {
        let json = r#"{"action": "approve", "reasoning": "test", "confidence": 1.5}"#;
        let s = parse_suggestion_json(json).unwrap();
        assert!((s.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_ollama_wrapped_response() {
        let ollama_response = r#"{"model":"gemma4","response":"{\"action\":\"approve\",\"reasoning\":\"safe\",\"confidence\":0.9}","done":true}"#;
        let s = parse_ollama_response(ollama_response).unwrap();
        assert_eq!(s.action, RuleAction::Approve);
    }

    #[test]
    fn defaults_on_missing_optional_fields() {
        let json = r#"{"action": "approve"}"#;
        let s = parse_suggestion_json(json).unwrap();
        assert_eq!(s.reasoning, "");
        assert!((s.confidence - 0.5).abs() < f64::EPSILON);
        assert!(s.message.is_none());
    }

    #[test]
    fn parse_openai_wrapped_response() {
        let openai_response = r#"{"choices":[{"message":{"content":"{\"action\":\"deny\",\"reasoning\":\"dangerous\",\"confidence\":0.95}"}}]}"#;
        let s = parse_openai_response(openai_response).unwrap();
        assert_eq!(s.action, RuleAction::Deny);
        assert!((s.confidence - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn detect_openai_endpoint() {
        assert!(is_openai_compatible(
            "http://localhost:8080/v1/chat/completions"
        ));
        assert!(is_openai_compatible("http://host/v1/completions"));
        assert!(!is_openai_compatible("http://localhost:11434/api/generate"));
    }
}
