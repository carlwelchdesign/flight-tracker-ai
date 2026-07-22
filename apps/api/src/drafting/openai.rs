use std::{env, time::Duration};

use reqwest::{Client, StatusCode};
use serde_json::{Value, json};

use super::{
    DraftError, DraftLanguageGenerator, DraftWording, GenerationFailureCode, MinimizedDraftEvidence,
};

const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const DEFAULT_MODEL: &str = "gpt-5.6-luna";
const MAX_RESPONSE_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone)]
pub struct OpenAiDraftConfig {
    pub model: String,
    pub timeout: Duration,
}

impl Default for OpenAiDraftConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_MODEL.to_owned(),
            timeout: Duration::from_secs(20),
        }
    }
}

pub struct OpenAiDraftClient {
    client: Client,
    api_key: String,
    endpoint: String,
    config: OpenAiDraftConfig,
}

impl OpenAiDraftClient {
    pub fn from_env(config: OpenAiDraftConfig) -> Result<Self, DraftError> {
        let api_key = env::var("OPENAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or(DraftError::Generation(GenerationFailureCode::Unavailable))?;
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|_| DraftError::Generation(GenerationFailureCode::Unavailable))?;
        Ok(Self {
            client,
            api_key,
            endpoint: DEFAULT_ENDPOINT.to_owned(),
            config,
        })
    }

    #[cfg(test)]
    fn with_endpoint(api_key: &str, endpoint: String, config: OpenAiDraftConfig) -> Self {
        Self {
            client: Client::builder().timeout(config.timeout).build().unwrap(),
            api_key: api_key.to_owned(),
            endpoint,
            config,
        }
    }

    async fn request(&self, evidence: &MinimizedDraftEvidence) -> Result<DraftWording, DraftError> {
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&request_body(&self.config.model, evidence))
            .send()
            .await
            .map_err(map_transport_error)?;
        let status = response.status();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES)
        {
            return Err(DraftError::Generation(
                GenerationFailureCode::InvalidResponse,
            ));
        }
        let bytes = response.bytes().await.map_err(map_transport_error)?;
        if bytes.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(DraftError::Generation(
                GenerationFailureCode::InvalidResponse,
            ));
        }
        if !status.is_success() {
            return Err(DraftError::Generation(classify_error_response(
                status, &bytes,
            )));
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| DraftError::Generation(GenerationFailureCode::InvalidResponse))?;
        parse_response(value)
    }
}

fn classify_error_response(status: StatusCode, body: &[u8]) -> GenerationFailureCode {
    if status != StatusCode::TOO_MANY_REQUESTS {
        return GenerationFailureCode::Unavailable;
    }
    let code = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| value.pointer("/error/code")?.as_str().map(str::to_owned));
    if code.as_deref() == Some("insufficient_quota") {
        GenerationFailureCode::QuotaExhausted
    } else {
        GenerationFailureCode::RateLimited
    }
}

impl DraftLanguageGenerator for OpenAiDraftClient {
    fn generator_name(&self) -> &'static str {
        "openai_responses_api"
    }

    fn model_name(&self) -> Option<&str> {
        Some(&self.config.model)
    }

    fn generate<'a>(
        &'a self,
        evidence: &'a MinimizedDraftEvidence,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<DraftWording, DraftError>> + Send + 'a>,
    > {
        Box::pin(self.request(evidence))
    }
}

fn request_body(model: &str, evidence: &MinimizedDraftEvidence) -> Value {
    json!({
        "model": model,
        "store": false,
        "max_output_tokens": 300,
        "reasoning": { "effort": "low" },
        "instructions": "Draft concise wording for a human reviewer. Use only the supplied synthetic facts. Copy every supplied display value exactly. Do not infer airports, aircraft, timing, fuel, cost, safety, clearance, dispatch, or operational action. Include every required fact id. The caveat must preserve the non-operational boundary.",
        "input": serde_json::to_string(evidence).expect("minimized evidence is serializable"),
        "text": {
            "format": {
                "type": "json_schema",
                "name": "flight_review_draft",
                "strict": true,
                "schema": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "headline": { "type": "string", "maxLength": 120 },
                        "body": { "type": "string", "maxLength": 600 },
                        "caveat": { "type": "string", "maxLength": 300 },
                        "fact_ids": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["candidate", "closest_approach", "added_distance", "source_boundary"]
                            }
                        }
                    },
                    "required": ["headline", "body", "caveat", "fact_ids"]
                }
            }
        }
    })
}

fn parse_response(value: Value) -> Result<DraftWording, DraftError> {
    let content = value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("content").and_then(Value::as_array))
        .flatten()
        .find(|item| item.get("type").and_then(Value::as_str) == Some("output_text"));
    let Some(content) = content else {
        let refused = value
            .get("output")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| item.get("content").and_then(Value::as_array))
            .flatten()
            .any(|item| item.get("type").and_then(Value::as_str) == Some("refusal"));
        return Err(DraftError::Generation(if refused {
            GenerationFailureCode::Refused
        } else {
            GenerationFailureCode::InvalidResponse
        }));
    };
    let text = content
        .get("text")
        .and_then(Value::as_str)
        .ok_or(DraftError::Generation(
            GenerationFailureCode::InvalidResponse,
        ))?;
    serde_json::from_str(text)
        .map_err(|_| DraftError::Generation(GenerationFailureCode::InvalidResponse))
}

fn map_transport_error(error: reqwest::Error) -> DraftError {
    DraftError::Generation(if error.is_timeout() {
        GenerationFailureCode::Timeout
    } else {
        GenerationFailureCode::Unavailable
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_non_persistent_bounded_and_contains_only_minimized_evidence() {
        let evidence = MinimizedDraftEvidence {
            case_id: "case-1".to_owned(),
            candidate_id: "north_clear".to_owned(),
            evaluated_at: "2026-07-20T17:00:00Z".parse().unwrap(),
            dataset_version: "ft502-cases-v1".to_owned(),
            rule_version: 1,
            closest_approach: super::super::display_fact(30.0, "NM"),
            added_distance: super::super::display_fact(5.0, "NM"),
            added_distance_percent: super::super::display_fact(2.0, "%"),
            citations: Vec::new(),
            boundary: "synthetic only",
        };
        let body = request_body("test-model", &evidence);
        assert_eq!(body["store"], false);
        assert_eq!(body["max_output_tokens"], 300);
        assert_eq!(body["text"]["format"]["strict"], true);
        let serialized = body.to_string();
        for forbidden in [
            "callsign",
            "registration",
            "raw_payload",
            "provider_record_id",
        ] {
            assert!(!serialized.contains(forbidden));
        }
    }

    #[test]
    fn response_parser_classifies_refusal_and_invalid_shape() {
        let refusal = json!({
            "output": [{"content": [{"type": "refusal", "refusal": "no"}]}]
        });
        assert!(matches!(
            parse_response(refusal),
            Err(DraftError::Generation(GenerationFailureCode::Refused))
        ));
        assert!(matches!(
            parse_response(json!({"output": []})),
            Err(DraftError::Generation(
                GenerationFailureCode::InvalidResponse
            ))
        ));
    }

    #[test]
    fn quota_exhaustion_is_distinct_from_transient_rate_limiting() {
        let exhausted = json!({"error": {"code": "insufficient_quota"}}).to_string();
        let limited = json!({"error": {"code": "rate_limit_exceeded"}}).to_string();
        assert_eq!(
            classify_error_response(StatusCode::TOO_MANY_REQUESTS, exhausted.as_bytes()),
            GenerationFailureCode::QuotaExhausted
        );
        assert_eq!(
            classify_error_response(StatusCode::TOO_MANY_REQUESTS, limited.as_bytes()),
            GenerationFailureCode::RateLimited
        );
        assert_eq!(
            classify_error_response(StatusCode::FORBIDDEN, b"{}"),
            GenerationFailureCode::Unavailable
        );
    }

    #[test]
    fn test_client_constructs_without_a_debug_surface_for_the_key() {
        let client = OpenAiDraftClient::with_endpoint(
            "secret-test-key",
            "http://127.0.0.1:9/v1/responses".to_owned(),
            OpenAiDraftConfig::default(),
        );
        assert_eq!(client.model_name(), Some(DEFAULT_MODEL));
    }
}
