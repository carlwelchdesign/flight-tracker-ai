//! Human-reviewed wording for approved offline recommendation evidence.
//!
//! Source facts, generated language, and review state are distinct contracts.
//! This module deliberately has no delivery, messaging, persistence, HTTP, or
//! operational-action capability.

mod openai;

use std::{future::Future, pin::Pin};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::optimization::{ExperimentResult, RecommendationOutcome};

pub use openai::{OpenAiDraftClient, OpenAiDraftConfig};

pub const DRAFT_POLICY_VERSION: u32 = 1;
pub const DETERMINISTIC_TEMPLATE_VERSION: u32 = 1;

const REQUIRED_FACT_IDS: [&str; 4] = [
    "candidate",
    "closest_approach",
    "added_distance",
    "source_boundary",
];
const UNSAFE_PHRASES: [&str; 9] = [
    "approved route",
    "cleared route",
    "dispatch release",
    "guaranteed",
    "operationally safe",
    "safe to fly",
    "should fly",
    "must reroute",
    "clearance granted",
];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RecommendationApproval {
    pub case_id: String,
    pub candidate_id: String,
    pub reviewer_id: String,
    pub reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MinimizedDraftEvidence {
    pub case_id: String,
    pub candidate_id: String,
    pub evaluated_at: DateTime<Utc>,
    pub dataset_version: String,
    pub rule_version: u32,
    pub closest_approach: DisplayFact,
    pub added_distance: DisplayFact,
    pub added_distance_percent: DisplayFact,
    pub citations: Vec<DraftCitation>,
    pub boundary: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DisplayFact {
    pub value: f64,
    pub unit: &'static str,
    pub display: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DraftCitation {
    pub id: &'static str,
    pub label: String,
    pub source: &'static str,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DraftWording {
    pub headline: String,
    pub body: String,
    pub caveat: String,
    pub fact_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DraftPackage {
    pub facts: MinimizedDraftEvidence,
    pub generated_wording: DraftWording,
    pub generation: GenerationEvidence,
    pub review_status: ReviewStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GenerationEvidence {
    pub generator: String,
    pub model: Option<String>,
    pub policy_version: u32,
    pub generated_at: DateTime<Utc>,
    pub fallback_reason: Option<GenerationFailureCode>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    AwaitingReview,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReviewedDraft {
    pub package: DraftPackage,
    pub review: ReviewEvidence,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReviewEvidence {
    pub reviewer_id: String,
    pub reviewed_at: DateTime<Utc>,
    pub decision: ReviewDecisionKind,
    pub edited: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecisionKind {
    Approve,
    Reject,
}

pub enum DraftReviewDecision {
    Approve {
        reviewer_id: String,
        reviewed_at: DateTime<Utc>,
    },
    EditAndApprove {
        reviewer_id: String,
        reviewed_at: DateTime<Utc>,
        wording: DraftWording,
    },
    Reject {
        reviewer_id: String,
        reviewed_at: DateTime<Utc>,
        note: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DraftFindingCode {
    Omission,
    FabricatedDetail,
    UnitChange,
    UnsafePhrasing,
    UnknownFactReference,
    UnboundedOutput,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DraftFinding {
    pub code: DraftFindingCode,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationFailureCode {
    Unavailable,
    Timeout,
    RateLimited,
    Refused,
    InvalidResponse,
    FailedValidation,
}

#[derive(Debug, Error)]
pub enum DraftError {
    #[error("recommendation evidence has not been explicitly approved for drafting")]
    ApprovalMismatch,
    #[error("recommendation evidence is not eligible for drafting")]
    NotARecommendation,
    #[error("reviewer identity is required")]
    MissingReviewer,
    #[error("draft has already received a review decision")]
    AlreadyReviewed,
    #[error("review timestamp cannot precede draft generation")]
    InvalidReviewTime,
    #[error("rejection note is required and must not exceed 500 characters")]
    InvalidRejectionNote,
    #[error("draft failed validation")]
    FailedValidation(Vec<DraftFinding>),
    #[error("draft generator failed: {0:?}")]
    Generation(GenerationFailureCode),
}

pub trait DraftLanguageGenerator: Sync {
    fn generator_name(&self) -> &'static str;
    fn model_name(&self) -> Option<&str>;
    fn generate<'a>(
        &'a self,
        evidence: &'a MinimizedDraftEvidence,
    ) -> Pin<Box<dyn Future<Output = Result<DraftWording, DraftError>> + Send + 'a>>;
}

pub fn minimize_approved_recommendation(
    result: &ExperimentResult,
    approval: &RecommendationApproval,
) -> Result<MinimizedDraftEvidence, DraftError> {
    let RecommendationOutcome::Recommendation {
        candidate_id,
        expected_effect,
        ..
    } = &result.outcome
    else {
        return Err(DraftError::NotARecommendation);
    };
    if approval.case_id != result.case_id
        || approval.candidate_id != *candidate_id
        || approval.reviewer_id.trim().is_empty()
        || approval.reviewed_at < result.evaluated_at
    {
        return Err(DraftError::ApprovalMismatch);
    }

    let closest = display_fact(expected_effect.closest_approach_nm, "NM");
    let added = display_fact(expected_effect.added_distance_nm, "NM");
    let added_percent = display_fact(expected_effect.added_distance_percent, "%");
    Ok(MinimizedDraftEvidence {
        case_id: result.case_id.clone(),
        candidate_id: candidate_id.clone(),
        evaluated_at: result.evaluated_at,
        dataset_version: result.dataset_version.clone(),
        rule_version: result.rule_version,
        closest_approach: closest,
        added_distance: added,
        added_distance_percent: added_percent,
        citations: vec![
            DraftCitation {
                id: "candidate",
                label: format!("Pre-authored candidate {candidate_id}"),
                source: "project-authored synthetic replay fixture",
                observed_at: result.evaluated_at,
            },
            DraftCitation {
                id: "closest_approach",
                label: format!(
                    "Fixed-margin geometry: {}",
                    display_fact(expected_effect.closest_approach_nm, "NM").display
                ),
                source: "deterministic Rust route-hazard rule",
                observed_at: result.evaluated_at,
            },
            DraftCitation {
                id: "added_distance",
                label: format!(
                    "Geometric proxy: {} ({})",
                    display_fact(expected_effect.added_distance_nm, "NM").display,
                    display_fact(expected_effect.added_distance_percent, "%").display
                ),
                source: "deterministic Rust candidate comparison",
                observed_at: result.evaluated_at,
            },
            DraftCitation {
                id: "source_boundary",
                label: format!(
                    "Dataset {} · rule version {} · human-approved input {}",
                    result.dataset_version, result.rule_version, approval.reviewed_at
                ),
                source: "FT-502 offline experiment",
                observed_at: result.evaluated_at,
            },
        ],
        boundary: "Synthetic offline review evidence only; not a route, clearance, dispatch release, or operational recommendation.",
    })
}

pub async fn generate_draft_with_fallback<G: DraftLanguageGenerator>(
    generator: &G,
    evidence: MinimizedDraftEvidence,
    generated_at: DateTime<Utc>,
) -> DraftPackage {
    let generated = generator.generate(&evidence).await;
    let (wording, fallback_reason, generator_name, model) = match generated {
        Ok(wording) => match validate_draft(&evidence, &wording) {
            findings if findings.is_empty() => (
                wording,
                None,
                generator.generator_name().to_owned(),
                generator.model_name().map(str::to_owned),
            ),
            _ => (
                deterministic_wording(&evidence),
                Some(GenerationFailureCode::FailedValidation),
                "deterministic_template".to_owned(),
                None,
            ),
        },
        Err(DraftError::Generation(code)) => (
            deterministic_wording(&evidence),
            Some(code),
            "deterministic_template".to_owned(),
            None,
        ),
        Err(_) => (
            deterministic_wording(&evidence),
            Some(GenerationFailureCode::InvalidResponse),
            "deterministic_template".to_owned(),
            None,
        ),
    };
    DraftPackage {
        facts: evidence,
        generated_wording: wording,
        generation: GenerationEvidence {
            generator: generator_name,
            model,
            policy_version: DRAFT_POLICY_VERSION,
            generated_at,
            fallback_reason,
        },
        review_status: ReviewStatus::AwaitingReview,
    }
}

pub fn deterministic_package(
    evidence: MinimizedDraftEvidence,
    generated_at: DateTime<Utc>,
) -> DraftPackage {
    DraftPackage {
        generated_wording: deterministic_wording(&evidence),
        facts: evidence,
        generation: GenerationEvidence {
            generator: "deterministic_template".to_owned(),
            model: None,
            policy_version: DETERMINISTIC_TEMPLATE_VERSION,
            generated_at,
            fallback_reason: None,
        },
        review_status: ReviewStatus::AwaitingReview,
    }
}

pub fn review_draft(
    mut package: DraftPackage,
    decision: DraftReviewDecision,
) -> Result<ReviewedDraft, DraftError> {
    if package.review_status != ReviewStatus::AwaitingReview {
        return Err(DraftError::AlreadyReviewed);
    }
    let (reviewer_id, reviewed_at, kind, edited, note) = match decision {
        DraftReviewDecision::Approve {
            reviewer_id,
            reviewed_at,
        } => (
            reviewer_id,
            reviewed_at,
            ReviewDecisionKind::Approve,
            false,
            None,
        ),
        DraftReviewDecision::EditAndApprove {
            reviewer_id,
            reviewed_at,
            wording,
        } => {
            let findings = validate_draft(&package.facts, &wording);
            if !findings.is_empty() {
                return Err(DraftError::FailedValidation(findings));
            }
            package.generated_wording = wording;
            (
                reviewer_id,
                reviewed_at,
                ReviewDecisionKind::Approve,
                true,
                None,
            )
        }
        DraftReviewDecision::Reject {
            reviewer_id,
            reviewed_at,
            note,
        } => {
            let trimmed = note.trim();
            if trimmed.is_empty() || trimmed.chars().count() > 500 {
                return Err(DraftError::InvalidRejectionNote);
            }
            (
                reviewer_id,
                reviewed_at,
                ReviewDecisionKind::Reject,
                false,
                Some(trimmed.to_owned()),
            )
        }
    };
    if reviewer_id.trim().is_empty() {
        return Err(DraftError::MissingReviewer);
    }
    if reviewed_at < package.generation.generated_at {
        return Err(DraftError::InvalidReviewTime);
    }
    package.review_status = match kind {
        ReviewDecisionKind::Approve => ReviewStatus::Approved,
        ReviewDecisionKind::Reject => ReviewStatus::Rejected,
    };
    Ok(ReviewedDraft {
        package,
        review: ReviewEvidence {
            reviewer_id: reviewer_id.trim().to_owned(),
            reviewed_at,
            decision: kind,
            edited,
            note,
        },
    })
}

pub fn validate_draft(
    evidence: &MinimizedDraftEvidence,
    wording: &DraftWording,
) -> Vec<DraftFinding> {
    let mut findings = Vec::new();
    let combined = format!("{} {} {}", wording.headline, wording.body, wording.caveat);
    let generated_claims = format!("{} {}", wording.headline, wording.body);
    if wording.headline.chars().count() > 120
        || wording.body.chars().count() > 600
        || wording.caveat.chars().count() > 300
    {
        findings.push(finding(
            DraftFindingCode::UnboundedOutput,
            "draft exceeds the bounded review surface",
        ));
    }
    for (id, token) in [
        ("candidate", evidence.candidate_id.as_str()),
        (
            "closest_approach",
            evidence.closest_approach.display.as_str(),
        ),
        ("added_distance", evidence.added_distance.display.as_str()),
        (
            "added_distance_percent",
            evidence.added_distance_percent.display.as_str(),
        ),
    ] {
        if !combined.contains(token) {
            findings.push(finding(
                DraftFindingCode::Omission,
                &format!("required fact {id} is not visible in generated wording"),
            ));
        }
    }
    for required in REQUIRED_FACT_IDS {
        if !wording.fact_ids.iter().any(|id| id == required) {
            findings.push(finding(
                DraftFindingCode::Omission,
                &format!("required fact reference {required} is missing"),
            ));
        }
    }
    for fact_id in &wording.fact_ids {
        if !REQUIRED_FACT_IDS.contains(&fact_id.as_str()) {
            findings.push(finding(
                DraftFindingCode::UnknownFactReference,
                &format!("unknown fact reference {fact_id}"),
            ));
        }
    }
    let lower = generated_claims.to_ascii_lowercase();
    if [" km", "kilometer", " statute mile", " meters", " metres"]
        .iter()
        .any(|unit| lower.contains(unit))
    {
        findings.push(finding(
            DraftFindingCode::UnitChange,
            "wording introduced a unit outside the approved evidence",
        ));
    }
    if UNSAFE_PHRASES.iter().any(|phrase| lower.contains(phrase)) {
        findings.push(finding(
            DraftFindingCode::UnsafePhrasing,
            "wording implies operational authority or safety",
        ));
    }
    for numeric in numeric_tokens(&combined) {
        let permitted = [
            evidence.candidate_id.as_str(),
            evidence.closest_approach.display.as_str(),
            evidence.added_distance.display.as_str(),
            evidence.added_distance_percent.display.as_str(),
            evidence.case_id.as_str(),
        ]
        .iter()
        .any(|allowed| allowed.contains(&numeric));
        if !permitted {
            findings.push(finding(
                DraftFindingCode::FabricatedDetail,
                &format!("unapproved numeric detail {numeric}"),
            ));
        }
    }
    findings.sort_by_key(|finding| finding.code);
    findings.dedup_by(|left, right| left.code == right.code && left.detail == right.detail);
    findings
}

fn deterministic_wording(evidence: &MinimizedDraftEvidence) -> DraftWording {
    DraftWording {
        headline: "Candidate ready for human review".to_owned(),
        body: format!(
            "Review candidate {}. The deterministic fixture rule measured a {} closest approach and a geometric added-distance proxy of {} ({}).",
            evidence.candidate_id,
            evidence.closest_approach.display,
            evidence.added_distance.display,
            evidence.added_distance_percent.display
        ),
        caveat: evidence.boundary.to_owned(),
        fact_ids: REQUIRED_FACT_IDS.iter().map(ToString::to_string).collect(),
    }
}

fn display_fact(value: f64, unit: &'static str) -> DisplayFact {
    DisplayFact {
        value,
        unit,
        display: if unit == "%" {
            format!("{value:.1}%")
        } else {
            format!("{value:.1} {unit}")
        },
    }
}

fn numeric_tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| {
            !character.is_ascii_digit() && character != '.' && character != '-'
        })
        .filter(|token| token.chars().any(|character| character.is_ascii_digit()))
        .map(str::to_owned)
        .collect()
}

fn finding(code: DraftFindingCode, detail: &str) -> DraftFinding {
    DraftFinding {
        code,
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::optimization::{evaluate_dataset, load_dataset};

    const OPTIMIZATION_DATASET: &str =
        include_str!("../../../../fixtures/optimization/ft502-cases-v1.json");
    const EVALS: &str = include_str!("../../../../fixtures/drafting/ft503-evals-v1.json");

    #[derive(Deserialize)]
    struct EvalDataset {
        schema_version: u32,
        cases: Vec<EvalCase>,
    }

    #[derive(Deserialize)]
    struct EvalCase {
        id: String,
        wording: DraftWording,
        expected_findings: Vec<DraftFindingCode>,
    }

    fn evidence() -> MinimizedDraftEvidence {
        let dataset = load_dataset(OPTIMIZATION_DATASET).unwrap();
        let result = evaluate_dataset(&dataset)
            .unwrap()
            .into_iter()
            .find(|result| result.case_id == "held-multi-01")
            .unwrap();
        let candidate_id = match &result.outcome {
            RecommendationOutcome::Recommendation { candidate_id, .. } => candidate_id.clone(),
            RecommendationOutcome::Abstention { .. } => panic!("fixture must recommend"),
        };
        minimize_approved_recommendation(
            &result,
            &RecommendationApproval {
                case_id: result.case_id.clone(),
                candidate_id,
                reviewer_id: "reviewer-1".to_owned(),
                reviewed_at: result.evaluated_at + chrono::Duration::minutes(5),
            },
        )
        .unwrap()
    }

    #[test]
    fn minimized_evidence_excludes_provider_raw_and_operational_fields() {
        let serialized = serde_json::to_string(&evidence()).unwrap();
        for forbidden in [
            "raw_payload",
            "provider_record_id",
            "registration",
            "callsign",
            "send",
            "dispatch_action",
        ] {
            assert!(!serialized.contains(forbidden));
        }
    }

    #[test]
    fn unapproved_or_abstained_results_cannot_enter_drafting() {
        let dataset = load_dataset(OPTIMIZATION_DATASET).unwrap();
        let results = evaluate_dataset(&dataset).unwrap();
        let abstention = results
            .iter()
            .find(|result| matches!(result.outcome, RecommendationOutcome::Abstention { .. }))
            .unwrap();
        let approval = RecommendationApproval {
            case_id: abstention.case_id.clone(),
            candidate_id: "none".to_owned(),
            reviewer_id: "reviewer-1".to_owned(),
            reviewed_at: abstention.evaluated_at + chrono::Duration::minutes(1),
        };
        assert!(matches!(
            minimize_approved_recommendation(abstention, &approval),
            Err(DraftError::NotARecommendation)
        ));

        let recommendation = results
            .iter()
            .find(|result| matches!(result.outcome, RecommendationOutcome::Recommendation { .. }))
            .unwrap();
        assert!(matches!(
            minimize_approved_recommendation(recommendation, &approval),
            Err(DraftError::ApprovalMismatch)
        ));
    }

    #[test]
    fn versioned_eval_set_detects_omissions_fabrication_units_and_unsafe_language() {
        let evals: EvalDataset = serde_json::from_str(EVALS).unwrap();
        assert_eq!(evals.schema_version, 1);
        assert!(evals.cases.len() >= 6);
        let evidence = evidence();
        for case in evals.cases {
            let actual: BTreeSet<_> = validate_draft(&evidence, &case.wording)
                .into_iter()
                .map(|finding| finding.code)
                .collect();
            let expected: BTreeSet<_> = case.expected_findings.into_iter().collect();
            assert_eq!(actual, expected, "{}", case.id);
        }
    }

    struct FailingGenerator(GenerationFailureCode);

    impl DraftLanguageGenerator for FailingGenerator {
        fn generator_name(&self) -> &'static str {
            "test_failure"
        }

        fn model_name(&self) -> Option<&str> {
            Some("test-model")
        }

        fn generate<'a>(
            &'a self,
            _evidence: &'a MinimizedDraftEvidence,
        ) -> Pin<Box<dyn Future<Output = Result<DraftWording, DraftError>> + Send + 'a>> {
            Box::pin(async move { Err(DraftError::Generation(self.0)) })
        }
    }

    struct FixedGenerator(DraftWording);

    impl DraftLanguageGenerator for FixedGenerator {
        fn generator_name(&self) -> &'static str {
            "test_generator"
        }

        fn model_name(&self) -> Option<&str> {
            Some("test-model")
        }

        fn generate<'a>(
            &'a self,
            _evidence: &'a MinimizedDraftEvidence,
        ) -> Pin<Box<dyn Future<Output = Result<DraftWording, DraftError>> + Send + 'a>> {
            Box::pin(async move { Ok(self.0.clone()) })
        }
    }

    #[tokio::test]
    async fn generator_failure_degrades_to_valid_deterministic_wording() {
        let evidence = evidence();
        let generated_at = evidence.evaluated_at + chrono::Duration::minutes(10);
        let package = generate_draft_with_fallback(
            &FailingGenerator(GenerationFailureCode::Unavailable),
            evidence,
            generated_at,
        )
        .await;
        assert_eq!(package.generation.generator, "deterministic_template");
        assert_eq!(
            package.generation.fallback_reason,
            Some(GenerationFailureCode::Unavailable)
        );
        assert!(validate_draft(&package.facts, &package.generated_wording).is_empty());
        assert_eq!(package.review_status, ReviewStatus::AwaitingReview);
    }

    #[tokio::test]
    async fn grounded_model_wording_remains_distinct_and_awaits_review() {
        let evidence = evidence();
        let wording = deterministic_wording(&evidence);
        let generated_at = evidence.evaluated_at + chrono::Duration::minutes(10);
        let package =
            generate_draft_with_fallback(&FixedGenerator(wording.clone()), evidence, generated_at)
                .await;

        assert_eq!(package.generated_wording, wording);
        assert_eq!(package.generation.generator, "test_generator");
        assert_eq!(package.generation.model.as_deref(), Some("test-model"));
        assert_eq!(package.generation.fallback_reason, None);
        assert_eq!(package.review_status, ReviewStatus::AwaitingReview);
    }

    #[tokio::test]
    async fn invalid_model_wording_fails_closed_to_the_template() {
        let evidence = evidence();
        let mut unsafe_wording = deterministic_wording(&evidence);
        unsafe_wording.body.push_str(" This is an approved route.");
        let generated_at = evidence.evaluated_at + chrono::Duration::minutes(10);
        let package =
            generate_draft_with_fallback(&FixedGenerator(unsafe_wording), evidence, generated_at)
                .await;

        assert_eq!(package.generation.generator, "deterministic_template");
        assert_eq!(
            package.generation.fallback_reason,
            Some(GenerationFailureCode::FailedValidation)
        );
        assert!(validate_draft(&package.facts, &package.generated_wording).is_empty());
    }

    #[test]
    fn approval_and_edit_are_explicit_and_invalid_edits_fail_closed() {
        let evidence = evidence();
        let package = deterministic_package(evidence.clone(), evidence.evaluated_at);
        let approved = review_draft(
            package,
            DraftReviewDecision::Approve {
                reviewer_id: "human-reviewer".to_owned(),
                reviewed_at: evidence.evaluated_at + chrono::Duration::minutes(1),
            },
        )
        .unwrap();
        assert_eq!(approved.package.review_status, ReviewStatus::Approved);
        assert!(!approved.review.edited);

        let package = deterministic_package(evidence.clone(), evidence.evaluated_at);
        let mut unsafe_edit = package.generated_wording.clone();
        unsafe_edit
            .body
            .push_str(" This is an approved route and safe to fly.");
        assert!(matches!(
            review_draft(
                package,
                DraftReviewDecision::EditAndApprove {
                    reviewer_id: "human-reviewer".to_owned(),
                    reviewed_at: evidence.evaluated_at + chrono::Duration::minutes(2),
                    wording: unsafe_edit,
                }
            ),
            Err(DraftError::FailedValidation(_))
        ));
    }

    #[test]
    fn review_must_be_pending_and_cannot_predate_generation() {
        let evidence = evidence();
        let generated_at = evidence.evaluated_at + chrono::Duration::minutes(10);
        let package = deterministic_package(evidence, generated_at);

        assert!(matches!(
            review_draft(
                package.clone(),
                DraftReviewDecision::Approve {
                    reviewer_id: "human-reviewer".to_owned(),
                    reviewed_at: generated_at - chrono::Duration::seconds(1),
                }
            ),
            Err(DraftError::InvalidReviewTime)
        ));

        let mut reviewed_package = package;
        reviewed_package.review_status = ReviewStatus::Approved;
        assert!(matches!(
            review_draft(
                reviewed_package,
                DraftReviewDecision::Approve {
                    reviewer_id: "human-reviewer".to_owned(),
                    reviewed_at: generated_at,
                }
            ),
            Err(DraftError::AlreadyReviewed)
        ));
    }

    #[test]
    fn serialized_contract_separates_facts_wording_citations_and_review_state() {
        let evidence = evidence();
        let package = deterministic_package(evidence.clone(), evidence.evaluated_at);
        let value = serde_json::to_value(package).unwrap();
        assert!(value.get("facts").is_some());
        assert!(value["facts"].get("citations").is_some());
        assert!(value.get("generated_wording").is_some());
        assert_eq!(value["review_status"], "awaiting_review");
        assert!(value.get("send").is_none());
    }
}
