//! Claude client for context extraction.
//!
//! Requests go to a proxy you operate, which injects the Anthropic API key and
//! forwards to `api.anthropic.com`. The key is never in the app: anything
//! shipped to a user's machine can be pulled out with `strings` or read off the
//! wire with a debugging proxy, so an embedded key is a public key.
//!
//! The proxy speaks the Anthropic Messages wire format verbatim, so it can be a
//! dumb passthrough that only adds the `x-api-key` header — and pointing
//! `BATON_API_BASE` straight at `https://api.anthropic.com` with a user's own
//! key needs no code change.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::context::{Context, ContextBody};

/// Overridable at build time: `BATON_API_BASE=https://... pnpm tauri build`.
/// The default is the hosted proxy.
pub fn base_url() -> &'static str {
    option_env!("BATON_API_BASE").unwrap_or("https://api.baton.app")
}

const MODEL: &str = "claude-opus-5";

/// Non-streaming keeps the client simple; 16k stays clear of HTTP timeouts.
/// Extractions are far smaller than this — the ceiling only matters for a
/// context with unusually long field values.
const MAX_TOKENS: u32 = 16_000;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("network error: {0}")]
    Http(String),
    #[error("service error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("the model declined this request{0}")]
    Refused(String),
    #[error("unexpected response shape: {0}")]
    Shape(String),
    #[error("could not parse the extracted context: {0}")]
    Json(#[from] serde_json::Error),
}

impl serde::Serialize for AiError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AiError>;

// ---------------------------------------------------------------- prompts
// Verbatim from PRD §10. Kept as three plain constants rather than a prompt
// abstraction — there is one provider and three operations.

const CREATE_PROMPT: &str = "\
Given this AI conversation, extract a durable developer context that another
AI can use to continue the work.

Do not invent information. If a field is not supported by the conversation,
leave it empty rather than guessing.

Prioritize:
- goal
- current state
- decisions
- constraints
- things already tried
- relevant files
- errors
- open issues
- next steps

Remove conversational noise.";

const UPDATE_PROMPT: &str = "\
Here is the existing context, followed by a new conversation.

Update the existing context using information from the new conversation.
Preserve valid information.
Update outdated information.
Remove information the new conversation contradicts.
Retain important decisions.
Resolve contradictions in favor of newer explicit information.
Do not invent facts.";

const HANDOFF_PROMPT: &str = "\
Transform this context into a concise prompt that allows another AI model to
continue the work without restarting.

Assume the user wants to continue from the current state.
Do not repeat unnecessary background.
Address the next model directly.
Output only the prompt text, with no preamble or commentary.";

/// JSON Schema mirroring `ContextBody`. Field names must match its serde
/// `camelCase` renaming exactly, or the response will deserialise to defaults
/// with no error — a silent, total data loss.
///
/// Strict schemas require every property in `required` plus
/// `additionalProperties: false`; nullable fields are `["string", "null"]`.
fn context_schema() -> Value {
    let strings = json!({ "type": "array", "items": { "type": "string" } });
    json!({
        "type": "object",
        "properties": {
            "description":   { "type": ["string", "null"] },
            "goal":          { "type": ["string", "null"] },
            "currentState":  { "type": ["string", "null"] },
            "decisions":     strings,
            "tried":         strings,
            "relevantFiles": strings,
            "constraints":   strings,
            "openIssues":    strings,
            "nextSteps":     strings
        },
        "required": [
            "description", "goal", "currentState", "decisions", "tried",
            "relevantFiles", "constraints", "openIssues", "nextSteps"
        ],
        "additionalProperties": false
    })
}

pub struct AiClient {
    http: reqwest::Client,
    device_id: String,
}

impl AiClient {
    pub fn new(device_id: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                // Long conversations take real time to process; the default is
                // generous but an explicit value documents the intent.
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .unwrap_or_default(),
            device_id,
        }
    }

    async fn post(&self, body: Value) -> Result<Value> {
        let res = self
            .http
            .post(format!("{}/v1/messages", base_url()))
            .header("content-type", "application/json")
            .header("anthropic-version", "2023-06-01")
            // Server-side fallback: if a safety classifier declines, the API
            // retries the same request on a fallback model within the call.
            .header("anthropic-beta", "server-side-fallback-2026-07-01")
            // Lets the proxy rate-limit without accounts. Spoofable, so treat
            // it as friction against casual abuse, not as authentication.
            .header("x-baton-device", &self.device_id)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;

        let status = res.status();
        let text = res.text().await.map_err(|e| AiError::Http(e.to_string()))?;

        if !status.is_success() {
            // Surface the API's own message when it sends one, but never echo
            // the request back — it contains the user's conversation.
            let message = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(str::to_owned))
                .unwrap_or_else(|| status.canonical_reason().unwrap_or("error").to_string());
            return Err(AiError::Api {
                status: status.as_u16(),
                message,
            });
        }

        Ok(serde_json::from_str(&text)?)
    }

    /// Pull the single text block out of a response, after checking the model
    /// did not decline. `stop_reason: "refusal"` arrives as HTTP 200, so
    /// reading `content` without this check yields a confusing empty result.
    fn text_of(res: &Value) -> Result<String> {
        if res["stop_reason"] == "refusal" {
            let detail = res["stop_details"]["category"]
                .as_str()
                .map(|c| format!(" ({c})"))
                .unwrap_or_default();
            return Err(AiError::Refused(detail));
        }

        res["content"]
            .as_array()
            .and_then(|blocks| {
                blocks
                    .iter()
                    .find(|b| b["type"] == "text")
                    .and_then(|b| b["text"].as_str())
            })
            .map(str::to_owned)
            .ok_or_else(|| AiError::Shape("no text block in response".into()))
    }

    async fn extract_body(&self, system: &str, user: String) -> Result<ContextBody> {
        let res = self
            .post(json!({
                "model": MODEL,
                "max_tokens": MAX_TOKENS,
                "system": system,
                "fallbacks": "default",
                "output_config": {
                    "effort": "medium",
                    "format": { "type": "json_schema", "schema": context_schema() }
                },
                "messages": [{ "role": "user", "content": user }]
            }))
            .await?;

        Ok(serde_json::from_str(&Self::text_of(&res)?)?)
    }

    /// PRD §10 Create — a raw conversation becomes a structured context.
    pub async fn extract(&self, conversation: &str) -> Result<ContextBody> {
        self.extract_body(CREATE_PROMPT, format!("<conversation>\n{conversation}\n</conversation>"))
            .await
    }

    /// PRD §10 Update — merge a newer conversation into an existing context.
    pub async fn update(&self, existing: &Context, conversation: &str) -> Result<ContextBody> {
        let current = serde_json::to_string_pretty(&existing.body)?;
        self.extract_body(
            UPDATE_PROMPT,
            format!(
                "<existing-context name=\"{}\">\n{current}\n</existing-context>\n\n\
                 <new-conversation>\n{conversation}\n</new-conversation>",
                existing.name
            ),
        )
        .await
    }

    /// PRD §10 Handoff — prose aimed at the next model, not at the user.
    pub async fn handoff(&self, context: &Context) -> Result<String> {
        let res = self
            .post(json!({
                "model": MODEL,
                "max_tokens": MAX_TOKENS,
                "system": HANDOFF_PROMPT,
                "fallbacks": "default",
                "output_config": { "effort": "medium" },
                "messages": [{ "role": "user", "content": context.to_markdown() }]
            }))
            .await?;

        Ok(Self::text_of(&res)?.trim().to_string())
    }
}

/// Shape of the fields we read back. Declared for documentation of the
/// contract; parsing goes through `serde_json::Value` so an added field in the
/// API response can never break extraction.
#[allow(dead_code)]
#[derive(Deserialize)]
struct MessageResponse {
    content: Vec<Value>,
    stop_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_field_names_match_the_struct() {
        // If these drift, the model returns valid JSON that deserialises to an
        // empty body with no error at all — the worst possible failure mode.
        let schema = context_schema();
        let props = schema["properties"].as_object().unwrap();
        let serialized = serde_json::to_value(ContextBody::default()).unwrap();
        let actual = serialized.as_object().unwrap();

        for key in props.keys() {
            assert!(actual.contains_key(key), "schema has unknown field `{key}`");
        }
        for key in actual.keys() {
            assert!(props.contains_key(key), "struct field `{key}` missing from schema");
        }

        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), props.len(), "strict mode requires every field");
    }

    #[test]
    fn refusal_is_an_error_not_an_empty_result() {
        let res = json!({
            "stop_reason": "refusal",
            "stop_details": { "type": "refusal", "category": "cyber" },
            "content": []
        });
        match AiClient::text_of(&res) {
            Err(AiError::Refused(d)) => assert!(d.contains("cyber")),
            other => panic!("expected refusal, got {other:?}"),
        }
    }

    #[test]
    fn text_is_read_from_the_text_block() {
        let res = json!({
            "stop_reason": "end_turn",
            "content": [
                { "type": "thinking", "thinking": "" },
                { "type": "text", "text": "{\"goal\":\"g\"}" }
            ]
        });
        assert_eq!(AiClient::text_of(&res).unwrap(), "{\"goal\":\"g\"}");
    }

    #[test]
    fn extracted_json_deserialises_into_the_body() {
        let raw = r#"{"description":null,"goal":"Replace auth","currentState":null,
            "decisions":["No NextAuth"],"tried":[],"relevantFiles":[],
            "constraints":[],"openIssues":[],"nextSteps":["Debug callback"]}"#;
        let body: ContextBody = serde_json::from_str(raw).unwrap();
        assert_eq!(body.goal.as_deref(), Some("Replace auth"));
        assert_eq!(body.decisions, vec!["No NextAuth"]);
        assert_eq!(body.next_steps, vec!["Debug callback"]);
    }
}
