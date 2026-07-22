use chrono::{Duration, Utc};
use flight_tracker_api::{
    drafting::{
        DraftReviewDecision, OpenAiDraftClient, OpenAiDraftConfig, RecommendationApproval,
        deterministic_package, generate_draft_with_fallback, minimize_approved_recommendation,
        review_draft,
    },
    optimization::{RecommendationOutcome, evaluate_dataset, load_dataset},
};
use serde_json::json;

const DATASET: &str = include_str!("../../../fixtures/optimization/ft502-cases-v1.json");

#[tokio::main]
async fn main() {
    let dataset = load_dataset(DATASET).expect("versioned FT-502 fixture must be valid");
    let result = evaluate_dataset(&dataset)
        .expect("offline recommendation experiment must evaluate")
        .into_iter()
        .find(|result| result.case_id == "held-multi-01")
        .expect("held-out fixture exists");
    let candidate_id = match &result.outcome {
        RecommendationOutcome::Recommendation { candidate_id, .. } => candidate_id.clone(),
        RecommendationOutcome::Abstention { .. } => panic!("held-out fixture must recommend"),
    };
    let evidence = minimize_approved_recommendation(
        &result,
        &RecommendationApproval {
            case_id: result.case_id.clone(),
            candidate_id,
            reviewer_id: "portfolio-human-reviewer".to_owned(),
            reviewed_at: result.evaluated_at + Duration::minutes(5),
        },
    )
    .expect("review approval must match the recommendation");
    let generated_at = Utc::now();
    let package = match OpenAiDraftClient::from_env(OpenAiDraftConfig::default()) {
        Ok(client) => generate_draft_with_fallback(&client, evidence, generated_at).await,
        Err(_) => deterministic_package(evidence, generated_at),
    };
    let awaiting_review = package.clone();
    let reviewed = review_draft(
        package,
        DraftReviewDecision::Approve {
            reviewer_id: "portfolio-human-reviewer".to_owned(),
            reviewed_at: generated_at + Duration::minutes(1),
        },
    )
    .expect("explicit review should approve valid wording");
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "awaiting_review": awaiting_review,
            "after_explicit_human_approval": reviewed,
            "automatic_send_available": false
        }))
        .unwrap()
    );
}
