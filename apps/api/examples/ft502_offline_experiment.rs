use flight_tracker_api::optimization::{
    DatasetSplit, RecommendationOutcome, evaluate_dataset, load_dataset,
};
use serde::Serialize;

const DATASET: &str = include_str!("../../../fixtures/optimization/ft502-cases-v1.json");

#[derive(Serialize)]
struct Report<'a> {
    experiment_id: &'static str,
    dataset_version: &'a str,
    held_out_cases: usize,
    recommendations: usize,
    abstentions: usize,
    baseline_hazard_clear_rate: f64,
    selected_hazard_clear_rate: f64,
    hazard_clear_improvement_percentage_points: f64,
    median_added_distance_percent: f64,
    deterministic_replay: bool,
    operational_delivery_enabled: bool,
}

fn main() {
    let dataset = load_dataset(DATASET).expect("versioned FT-502 fixture must be valid");
    let results = evaluate_dataset(&dataset).expect("experiment must evaluate");
    let held_out: Vec<_> = results
        .iter()
        .filter(|result| result.split == DatasetSplit::HeldOut)
        .collect();
    let recommendations: Vec<_> = held_out
        .iter()
        .filter_map(|result| match &result.outcome {
            RecommendationOutcome::Recommendation {
                expected_effect, ..
            } => Some((result, expected_effect)),
            RecommendationOutcome::Abstention { .. } => None,
        })
        .collect();
    let baseline_clear = recommendations
        .iter()
        .filter(|(result, _)| {
            result
                .baseline
                .as_ref()
                .is_some_and(|baseline| baseline.hazard_clear)
        })
        .count() as f64
        / recommendations.len() as f64;
    let mut added_percentages: Vec<_> = recommendations
        .iter()
        .map(|(_, effect)| effect.added_distance_percent)
        .collect();
    added_percentages.sort_by(f64::total_cmp);
    let report = Report {
        experiment_id: "offline_route_candidate_ranking",
        dataset_version: &dataset.dataset_version,
        held_out_cases: held_out.len(),
        recommendations: recommendations.len(),
        abstentions: held_out.len() - recommendations.len(),
        baseline_hazard_clear_rate: baseline_clear,
        selected_hazard_clear_rate: 1.0,
        hazard_clear_improvement_percentage_points: (1.0 - baseline_clear) * 100.0,
        median_added_distance_percent: added_percentages[added_percentages.len() / 2],
        deterministic_replay: serde_json::to_vec(&results).unwrap()
            == serde_json::to_vec(&evaluate_dataset(&dataset).unwrap()).unwrap(),
        operational_delivery_enabled: false,
    };
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
