//! Fixed-input public demonstration of the human-reviewed drafting pipeline.
//!
//! The route accepts no caller input and can only evaluate the project-authored
//! FT-502 fixture. The generated package is cached for the process lifetime so
//! this public portfolio surface cannot become an open model proxy.

use std::{future::Future, pin::Pin, sync::LazyLock};

use axum::{Json, Router, http::header, response::IntoResponse, routing::get};
use chrono::{Duration, Utc};
use serde::Serialize;
use tokio::sync::OnceCell;

use crate::{
    drafting::{
        DraftError, DraftLanguageGenerator, DraftPackage, DraftWording, GenerationFailureCode,
        MinimizedDraftEvidence, OpenAiDraftClient, OpenAiDraftConfig, RecommendationApproval,
        generate_draft_with_fallback, minimize_approved_recommendation,
    },
    optimization::{RecommendationOutcome, evaluate_dataset, load_dataset},
};

const DATASET: &str = include_str!("../../../fixtures/optimization/ft502-cases-v1.json");
const PUBLIC_CASE_ID: &str = "held-multi-01";
const SYNTHETIC_REVIEWER: &str = "portfolio-synthetic-review";

static PUBLIC_DRAFT: LazyLock<OnceCell<Result<PublicAiDraft, PublicAiDraftError>>> =
    LazyLock::new(OnceCell::new);

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PublicAiDraft {
    pub package: DraftPackage,
    pub automatic_send_available: bool,
    pub boundary: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PublicAiDraftError {
    error: PublicAiDraftErrorBody,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PublicAiDraftErrorBody {
    code: &'static str,
    message: &'static str,
}

pub fn public_ai_draft_router() -> Router {
    Router::new().route("/api/public/ai-draft", get(public_ai_draft))
}

async fn public_ai_draft() -> impl IntoResponse {
    let result = PUBLIC_DRAFT.get_or_init(build_public_ai_draft).await;
    match result {
        Ok(draft) => ([(header::CACHE_CONTROL, "no-store")], Json(draft.clone())).into_response(),
        Err(error) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            [(header::CACHE_CONTROL, "no-store")],
            Json(error.clone()),
        )
            .into_response(),
    }
}

async fn build_public_ai_draft() -> Result<PublicAiDraft, PublicAiDraftError> {
    let evidence = fixed_public_evidence()?;
    let generated_at = Utc::now();
    let package = match OpenAiDraftClient::from_env(OpenAiDraftConfig::default()) {
        Ok(client) => generate_draft_with_fallback(&client, evidence, generated_at).await,
        Err(_) => generate_draft_with_fallback(&UnavailableGenerator, evidence, generated_at).await,
    };
    Ok(PublicAiDraft {
        package,
        automatic_send_available: false,
        boundary: "AI drafts wording only. A human must review it; this demonstration cannot approve, send, select a route, or trigger an operational action.",
    })
}

fn fixed_public_evidence() -> Result<MinimizedDraftEvidence, PublicAiDraftError> {
    let dataset = load_dataset(DATASET).map_err(|_| unavailable())?;
    let result = evaluate_dataset(&dataset)
        .map_err(|_| unavailable())?
        .into_iter()
        .find(|result| result.case_id == PUBLIC_CASE_ID)
        .ok_or_else(unavailable)?;
    let candidate_id = match &result.outcome {
        RecommendationOutcome::Recommendation { candidate_id, .. } => candidate_id.clone(),
        RecommendationOutcome::Abstention { .. } => return Err(unavailable()),
    };
    minimize_approved_recommendation(
        &result,
        &RecommendationApproval {
            case_id: result.case_id.clone(),
            candidate_id,
            reviewer_id: SYNTHETIC_REVIEWER.to_owned(),
            reviewed_at: result.evaluated_at + Duration::minutes(5),
        },
    )
    .map_err(|_| unavailable())
}

fn unavailable() -> PublicAiDraftError {
    PublicAiDraftError {
        error: PublicAiDraftErrorBody {
            code: "public_ai_draft_unavailable",
            message: "The fixed synthetic drafting demonstration is unavailable",
        },
    }
}

struct UnavailableGenerator;

impl DraftLanguageGenerator for UnavailableGenerator {
    fn generator_name(&self) -> &'static str {
        "openai_responses_api"
    }

    fn model_name(&self) -> Option<&str> {
        None
    }

    fn generate<'a>(
        &'a self,
        _evidence: &'a MinimizedDraftEvidence,
    ) -> Pin<Box<dyn Future<Output = Result<DraftWording, DraftError>> + Send + 'a>> {
        Box::pin(async { Err(DraftError::Generation(GenerationFailureCode::Unavailable)) })
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;

    #[test]
    fn fixed_evidence_is_synthetic_minimized_and_review_approved() {
        let evidence = fixed_public_evidence().unwrap();
        assert_eq!(evidence.case_id, PUBLIC_CASE_ID);
        assert_eq!(evidence.citations.len(), 4);
        assert!(
            evidence
                .citations
                .iter()
                .all(|citation| !citation.source.contains("adsb"))
        );
        let serialized = serde_json::to_string(&evidence).unwrap();
        for forbidden in ["callsign", "registration", "tenant", "prompt", "weather"] {
            assert!(!serialized.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[tokio::test]
    async fn public_route_accepts_no_input_and_never_exposes_an_action() {
        let response = public_ai_draft_router()
            .oneshot(
                Request::get("/api/public/ai-draft")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["package"]["review_status"], "awaiting_review");
        assert_eq!(value["automatic_send_available"], false);
        assert!(value.get("approve").is_none());
        assert!(value.get("send").is_none());
    }
}
