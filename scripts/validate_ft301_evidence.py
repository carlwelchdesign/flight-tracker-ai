#!/usr/bin/env python3
"""Validate the structure and, optionally, completeness of FT-301 evidence."""

from __future__ import annotations

import argparse
import csv
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path
import re
import sys


PROVIDERS = {"cirium_sky_stream", "flightaware_firehose"}
REQUIRED_FILES = {
    "README.md",
    "RIGHTS_AND_SERVICE_QUESTIONNAIRE.md",
    "EVIDENCE_REGISTER.md",
    "TRIAL_PROTOCOL.md",
    "DECISION_SCORECARD.md",
    "OUTREACH_REQUESTS.md",
    "PROCUREMENT_HANDOFF.md",
    "provider-decision.csv",
    "provider-question-responses.csv",
    "provider-scores.csv",
    "trial-scorecard.csv",
    "cost-model.csv",
}
REQUIRED_RIGHTS_IDS = {f"R-{number:02d}" for number in range(1, 22)}
REQUIRED_SERVICE_IDS = {f"S-{number:02d}" for number in range(1, 13)}
REQUIRED_EVIDENCE_IDS = {
    "FA-RIGHTS",
    "FA-SLA",
    "FA-PRICE",
    "FA-TRIAL",
    "CI-RIGHTS",
    "CI-SLA",
    "CI-PRICE",
    "CI-TRIAL",
    "TARGET-POP",
    "TRIAL-RESULT",
}
EVIDENCE_STATUSES = {
    "missing",
    "requested",
    "received",
    "accepted",
    "exception",
    "rejected",
}
TERMINAL_EVIDENCE_STATUSES = {"accepted", "exception", "rejected"}
EVIDENCE_COLUMNS = (
    "Evidence ID",
    "Provider",
    "Category",
    "Status",
    "Document or test window",
    "Controlled reference",
    "Received",
    "Owner",
    "Reviewer",
    "Notes",
)
RESPONSE_COLUMNS = (
    "provider",
    "question_id",
    "answer",
    "controlling_clause",
    "limitations",
    "additional_fee",
    "evidence_id",
    "reviewer",
    "review_status",
    "notes",
)
RESPONSE_ANSWERS = {"pending", "yes", "no", "exception_required"}
RESPONSE_REVIEW_STATUSES = {"pending", "accepted", "exception", "rejected"}
TERMINAL_RESPONSE_STATUSES = {"accepted", "exception", "rejected"}
QUESTION_IDS = REQUIRED_RIGHTS_IDS | REQUIRED_SERVICE_IDS
EXPECTED_RESPONSE_KEYS = {
    (provider, question_id)
    for provider in PROVIDERS
    for question_id in QUESTION_IDS
}
EXPECTED_RESPONSE_EVIDENCE = {
    ("flightaware_firehose", "R"): "FA-RIGHTS",
    ("flightaware_firehose", "S"): "FA-SLA",
    ("cirium_sky_stream", "R"): "CI-RIGHTS",
    ("cirium_sky_stream", "S"): "CI-SLA",
}
SCORE_DIMENSIONS = {
    "target_flight_regional_coverage": (Decimal("30"), "TRIAL-RESULT"),
    "freshness_data_quality": (Decimal("20"), "TRIAL-RESULT"),
    "recovery_operational_reliability": (Decimal("15"), "TRIAL-RESULT"),
    "flight_identity_operational_events": (Decimal("10"), "TRIAL-RESULT"),
    "rights_retention_simplicity": (Decimal("10"), None),
    "three_scale_total_cost": (Decimal("10"), None),
    "implementation_support_fit": (Decimal("5"), None),
}
EXPECTED_SCORE_KEYS = {
    (provider, dimension)
    for provider in PROVIDERS
    for dimension in SCORE_DIMENSIONS
}
SCORE_COLUMNS = (
    "provider",
    "dimension",
    "weight",
    "points",
    "evidence_ids",
    "reviewer",
    "status",
    "notes",
)
SCORE_STATUSES = {"pending", "complete", "excluded"}
PROVIDER_EVIDENCE = {
    "flightaware_firehose": {
        "rights": "FA-RIGHTS",
        "price": "FA-PRICE",
        "sla": "FA-SLA",
        "trial": "FA-TRIAL",
    },
    "cirium_sky_stream": {
        "rights": "CI-RIGHTS",
        "price": "CI-PRICE",
        "sla": "CI-SLA",
        "trial": "CI-TRIAL",
    },
}
DECISION_COLUMNS = (
    "decision",
    "selected_provider",
    "fallback",
    "scoring_method_version",
    "effective_package",
    "primary_evidence_window",
    "legal_approval_ref",
    "engineering_approval_ref",
    "product_approval_ref",
    "sensitivity_result",
    "termination_export_plan",
    "implementation_estimate",
    "reconsideration_triggers",
    "decision_date",
    "od_002_status",
    "notes",
)
TRIAL_METRICS = {
    "expected_flight_identification",
    "position_availability",
    "position_age_15s",
    "position_age_30s",
    "position_age_60s",
    "delivery_lag_p50",
    "delivery_lag_p95",
    "delivery_lag_p99",
    "longest_position_gap",
    "replay_completeness",
    "schedule_accuracy",
    "tail_continuity",
    "route_accuracy",
    "diversion_precision",
    "diversion_recall",
    "cancellation_precision",
    "cancellation_recall",
    "disconnect_count",
    "disconnect_duration",
    "duplicate_delivery_rate",
    "out_of_order_rate",
    "collector_availability",
}
COST_SCENARIOS = {"small": 20, "growth": 100, "scale": 500}
COST_BEHAVIORS = {"normal", "peak", "replay", "reconnect", "provider_failure"}
STATUSES = {"pending", "complete", "not_observed", "excluded"}
MONEY_FIELDS = (
    "fixed_monthly",
    "usage_monthly",
    "environment_monthly",
    "connection_monthly",
    "replay_monthly",
    "support_monthly",
    "other_monthly",
)
TRIAL_COLUMNS = (
    "provider", "region", "metric", "method_version", "window_start", "window_end",
    "numerator", "denominator", "value", "unit", "sample_count", "evidence_id",
    "status", "notes",
)
COST_COLUMNS = (
    "provider", "scenario", "behavior", "simultaneous_flights", "currency",
    "fixed_monthly", "usage_monthly", "environment_monthly", "connection_monthly",
    "replay_monthly", "support_monthly", "other_monthly", "total_monthly",
    "quote_evidence_id", "status", "notes",
)


def markdown_table(path: Path, header: tuple[str, ...]) -> tuple[list[list[str]], list[str]]:
    """Read a Markdown table with an exact header and return its data rows."""
    lines = path.read_text(encoding="utf-8").splitlines()
    header_index: int | None = None
    for index, line in enumerate(lines):
        if not line.startswith("|"):
            continue
        cells = tuple(cell.strip() for cell in line.strip().strip("|").split("|"))
        if cells == header:
            header_index = index
            break
    if header_index is None:
        return [], [f"{path.name}: required table header is missing or changed"]

    rows: list[list[str]] = []
    errors: list[str] = []
    for line_number, line in enumerate(lines[header_index + 1 :], start=header_index + 2):
        if not line.startswith("|"):
            if rows:
                break
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if all(cell and set(cell) <= {"-", ":"} for cell in cells):
            continue
        if len(cells) != len(header):
            errors.append(
                f"{path.name} line {line_number}: expected {len(header)} columns, found {len(cells)}"
            )
            continue
        rows.append(cells)
    return rows, errors


def validate_questionnaire(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    found = re.findall(r"^\|\s*([RS]-\d{2})\s*\|", text, flags=re.MULTILINE)
    errors: list[str] = []
    duplicates = sorted({item for item in found if found.count(item) > 1})
    if duplicates:
        errors.append(f"{path.name}: duplicate question IDs {duplicates}")
    found_set = set(found)
    expected = REQUIRED_RIGHTS_IDS | REQUIRED_SERVICE_IDS
    missing = sorted(expected - found_set)
    unexpected = sorted(found_set - expected)
    if missing:
        errors.append(f"{path.name}: missing question IDs {missing}")
    if unexpected:
        errors.append(f"{path.name}: unsupported question IDs {unexpected}")
    return errors


def validate_evidence_register(
    rows: list[list[str]], require_complete: bool
) -> list[str]:
    errors: list[str] = []
    seen: set[str] = set()
    for line_offset, row in enumerate(rows, start=1):
        evidence_id, _, _, status, _, reference, received, owner, reviewer, _ = row
        if evidence_id in seen:
            errors.append(f"EVIDENCE_REGISTER.md: duplicate evidence ID {evidence_id}")
        seen.add(evidence_id)
        if evidence_id not in REQUIRED_EVIDENCE_IDS:
            errors.append(f"EVIDENCE_REGISTER.md: unsupported evidence ID {evidence_id}")
        if status not in EVIDENCE_STATUSES:
            errors.append(
                f"EVIDENCE_REGISTER.md row {line_offset}: unsupported status {status!r}"
            )
        if not owner or not reviewer:
            errors.append(f"EVIDENCE_REGISTER.md row {line_offset}: owner and reviewer are required")
        if status in {"received", *TERMINAL_EVIDENCE_STATUSES}:
            if not reference or reference.lower() == "pending":
                errors.append(f"EVIDENCE_REGISTER.md row {line_offset}: received evidence needs a controlled reference")
            if not received or received.lower() == "pending":
                errors.append(f"EVIDENCE_REGISTER.md row {line_offset}: received evidence needs a received date")
        if require_complete and status not in TERMINAL_EVIDENCE_STATUSES:
            errors.append(f"EVIDENCE_REGISTER.md: {evidence_id} is not in a terminal review state")

    missing = sorted(REQUIRED_EVIDENCE_IDS - seen)
    if missing:
        errors.append(f"EVIDENCE_REGISTER.md: missing evidence IDs {missing}")
    return errors


def validate_question_responses(
    rows: list[dict[str, str]], require_complete: bool
) -> list[str]:
    errors: list[str] = []
    seen: set[tuple[str, str]] = set()
    for line, row in enumerate(rows, start=2):
        provider = row.get("provider", "")
        question_id = row.get("question_id", "")
        answer = row.get("answer", "")
        review_status = row.get("review_status", "")
        key = (provider, question_id)
        if provider not in PROVIDERS:
            errors.append(f"response line {line}: unsupported provider {provider!r}")
        if question_id not in QUESTION_IDS:
            errors.append(f"response line {line}: unsupported question ID {question_id!r}")
        if key in seen:
            errors.append(f"response line {line}: duplicate provider/question")
        seen.add(key)
        if answer not in RESPONSE_ANSWERS:
            errors.append(f"response line {line}: unsupported answer {answer!r}")
        if review_status not in RESPONSE_REVIEW_STATUSES:
            errors.append(
                f"response line {line}: unsupported review status {review_status!r}"
            )
        if not row.get("reviewer"):
            errors.append(f"response line {line}: reviewer is required")

        question_family = question_id[:1]
        expected_evidence = EXPECTED_RESPONSE_EVIDENCE.get((provider, question_family))
        if expected_evidence and row.get("evidence_id") != expected_evidence:
            errors.append(
                f"response line {line}: evidence_id must be {expected_evidence}"
            )

        if answer != "pending":
            required = ("controlling_clause", "limitations", "additional_fee")
            missing = [field for field in required if not row.get(field)]
            if missing:
                errors.append(
                    f"response line {line}: answered row missing {', '.join(missing)}"
                )

        compatible_answers = {
            "accepted": {"yes"},
            "exception": {"exception_required"},
            "rejected": {"no", "exception_required"},
        }
        allowed = compatible_answers.get(review_status)
        if allowed is not None and answer not in allowed:
            errors.append(
                f"response line {line}: {review_status} review is incompatible with {answer!r} answer"
            )
        if require_complete and review_status not in TERMINAL_RESPONSE_STATUSES:
            errors.append(
                f"response line {line}: question response is not in a terminal review state"
            )

    missing = sorted(EXPECTED_RESPONSE_KEYS - seen)
    if missing:
        errors.append(f"response matrix: missing provider/question rows {missing}")
    unexpected = sorted(seen - EXPECTED_RESPONSE_KEYS)
    if unexpected:
        errors.append(f"response matrix: unsupported provider/question rows {unexpected}")
    return errors


def expected_score_evidence(provider: str, dimension: str) -> str | None:
    configured = SCORE_DIMENSIONS.get(dimension)
    provider_evidence = PROVIDER_EVIDENCE.get(provider)
    if configured is None or provider_evidence is None:
        return None
    if configured[1] is not None:
        return configured[1]
    return {
        "rights_retention_simplicity": provider_evidence["rights"],
        "three_scale_total_cost": provider_evidence["price"],
        "implementation_support_fit": provider_evidence["sla"],
    }.get(dimension)


def validate_scores(
    rows: list[dict[str, str]], require_complete: bool
) -> list[str]:
    errors: list[str] = []
    seen: set[tuple[str, str]] = set()
    for line, row in enumerate(rows, start=2):
        provider = row.get("provider", "")
        dimension = row.get("dimension", "")
        status = row.get("status", "")
        key = (provider, dimension)
        if provider not in PROVIDERS:
            errors.append(f"score line {line}: unsupported provider {provider!r}")
        if dimension not in SCORE_DIMENSIONS:
            errors.append(f"score line {line}: unsupported dimension {dimension!r}")
        if key in seen:
            errors.append(f"score line {line}: duplicate provider/dimension")
        seen.add(key)
        if status not in SCORE_STATUSES:
            errors.append(f"score line {line}: unsupported status {status!r}")
        if not row.get("reviewer"):
            errors.append(f"score line {line}: reviewer is required")

        configured = SCORE_DIMENSIONS.get(dimension)
        if configured is not None:
            try:
                if Decimal(row.get("weight", "")) != configured[0]:
                    errors.append(f"score line {line}: weight does not match decision rubric")
            except InvalidOperation:
                errors.append(f"score line {line}: weight must be numeric")
        expected_evidence = expected_score_evidence(provider, dimension)
        if expected_evidence and row.get("evidence_ids") != expected_evidence:
            errors.append(
                f"score line {line}: evidence_ids must be {expected_evidence}"
            )

        if status == "complete":
            if not row.get("points") or not row.get("notes"):
                errors.append(f"score line {line}: complete score needs points and notes")
            else:
                try:
                    points = Decimal(row["points"])
                    if points < 0 or points > 5:
                        errors.append(f"score line {line}: points must be between 0 and 5")
                except InvalidOperation:
                    errors.append(f"score line {line}: points must be numeric")
        elif status == "excluded":
            if row.get("points"):
                errors.append(f"score line {line}: excluded score cannot have points")
            if not row.get("notes"):
                errors.append(f"score line {line}: excluded score needs a reason")
        elif row.get("points"):
            errors.append(f"score line {line}: pending score cannot have points")

        if require_complete and status == "pending":
            errors.append(f"score line {line}: score is still pending")

    missing = sorted(EXPECTED_SCORE_KEYS - seen)
    if missing:
        errors.append(f"score matrix: missing provider/dimension rows {missing}")
    unexpected = sorted(seen - EXPECTED_SCORE_KEYS)
    if unexpected:
        errors.append(f"score matrix: unsupported provider/dimension rows {unexpected}")
    return errors


def validate_decision(
    rows: list[dict[str, str]],
    score_rows: list[dict[str, str]],
    response_rows: list[dict[str, str]],
    evidence_rows: list[list[str]],
    require_complete: bool,
) -> list[str]:
    if len(rows) != 1:
        return [f"provider-decision.csv: expected exactly one row, found {len(rows)}"]
    row = rows[0]
    errors: list[str] = []
    decision = row.get("decision", "")
    selected = row.get("selected_provider", "")
    fallback = row.get("fallback", "")
    if decision not in {"pending", "select", "no_select"}:
        errors.append(f"decision: unsupported decision {decision!r}")
    if row.get("scoring_method_version") != "ft301-v1":
        errors.append("decision: scoring_method_version must be ft301-v1")
    if row.get("od_002_status") not in {"pending", "resolved"}:
        errors.append("decision: od_002_status must be pending or resolved")
    if decision == "pending":
        if selected:
            errors.append("decision: pending decision cannot select a provider")
    elif decision == "select":
        if selected not in PROVIDERS:
            errors.append("decision: selected_provider must name a finalist")
        if fallback not in (PROVIDERS | {"simulation"}):
            errors.append("decision: fallback must name the other finalist or simulation")
        if fallback == selected:
            errors.append("decision: fallback must differ from selected_provider")
    elif decision == "no_select":
        if selected:
            errors.append("decision: no_select cannot name a selected_provider")
        if fallback != "simulation":
            errors.append("decision: no_select fallback must be simulation")

    if require_complete:
        if decision == "pending":
            errors.append("decision: final decision is still pending")
        required = (
            "effective_package",
            "primary_evidence_window",
            "legal_approval_ref",
            "engineering_approval_ref",
            "product_approval_ref",
            "sensitivity_result",
            "termination_export_plan",
            "implementation_estimate",
            "reconsideration_triggers",
            "decision_date",
            "notes",
        )
        missing = [field for field in required if not row.get(field)]
        if missing:
            errors.append(f"decision: final record missing {', '.join(missing)}")
        if row.get("od_002_status") != "resolved":
            errors.append("decision: OD-002 is not resolved")
        if row.get("decision_date"):
            try:
                datetime.strptime(row["decision_date"], "%Y-%m-%d")
            except ValueError:
                errors.append("decision: decision_date must use YYYY-MM-DD")

        if decision == "select" and selected in PROVIDERS:
            selected_scores = [item for item in score_rows if item.get("provider") == selected]
            if any(item.get("status") != "complete" for item in selected_scores):
                errors.append("decision: selected provider must have every score completed")
            selected_responses = [
                item for item in response_rows if item.get("provider") == selected
            ]
            if any(
                item.get("review_status") not in {"accepted", "exception"}
                for item in selected_responses
            ):
                errors.append("decision: selected provider has a pending or rejected response")
            evidence_statuses = {item[0]: item[3] for item in evidence_rows}
            required_evidence = set(PROVIDER_EVIDENCE[selected].values()) | {
                "TARGET-POP",
                "TRIAL-RESULT",
            }
            if any(
                evidence_statuses.get(evidence_id) not in {"accepted", "exception"}
                for evidence_id in required_evidence
            ):
                errors.append("decision: selected provider has unaccepted required evidence")
    return errors


def validate_decision_log(
    path: Path, decision_rows: list[dict[str, str]]
) -> list[str]:
    if len(decision_rows) != 1:
        return []
    status = decision_rows[0].get("od_002_status")
    if not path.is_file():
        if status == "resolved":
            return ["decision log: DECISIONS.md is required to prove OD-002 resolution"]
        return []
    text = path.read_text(encoding="utf-8")
    open_entry = re.search(r"^\|\s*OD-002\s*\|", text, flags=re.MULTILINE) is not None
    resolution = "OD-002 is resolved" in text
    errors: list[str] = []
    if status == "resolved":
        if open_entry:
            errors.append("decision log: OD-002 is still listed under open decisions")
        if not resolution:
            errors.append("decision log: OD-002 resolution statement is missing")
    elif status == "pending" and not open_entry and not resolution:
        errors.append("decision log: pending OD-002 is missing from open decisions")
    return errors


def validate_table(path: Path, expected: tuple[str, ...]) -> tuple[list[dict[str, str]], list[str]]:
    with path.open(newline="", encoding="utf-8") as source:
        reader = csv.DictReader(source)
        rows = list(reader)
    errors: list[str] = []
    if tuple(reader.fieldnames or ()) != expected:
        errors.append(f"{path.name}: columns do not match the required schema")
    for line, row in enumerate(rows, start=2):
        if None in row:
            errors.append(f"{path.name} line {line}: contains extra columns")
    return rows, errors


def parse_utc(value: str) -> datetime:
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("timestamp lacks timezone")
    return parsed.astimezone(timezone.utc)


def validate_trial(rows: list[dict[str, str]], require_complete: bool) -> list[str]:
    errors: list[str] = []
    seen: set[tuple[str, str, str]] = set()
    global_metrics: dict[str, set[str]] = {provider: set() for provider in PROVIDERS}
    for line, row in enumerate(rows, start=2):
        provider = row.get("provider", "")
        metric = row.get("metric", "")
        region = row.get("region", "")
        status = row.get("status", "")
        key = (provider, region, metric)
        if provider not in PROVIDERS:
            errors.append(f"trial line {line}: unsupported provider {provider!r}")
        if not region:
            errors.append(f"trial line {line}: region is required")
        if metric not in TRIAL_METRICS:
            errors.append(f"trial line {line}: unsupported metric {metric!r}")
        if row.get("method_version") != "ft301-v1":
            errors.append(f"trial line {line}: method_version must be ft301-v1")
        if status not in STATUSES:
            errors.append(f"trial line {line}: unsupported status {status!r}")
        if key in seen:
            errors.append(f"trial line {line}: duplicate provider/region/metric")
        seen.add(key)
        if provider in PROVIDERS and region == "all":
            global_metrics[provider].add(metric)
        if status == "complete":
            required = ("window_start", "window_end", "value", "unit", "sample_count", "evidence_id")
            if row.get("unit") == "percent":
                required += ("numerator", "denominator")
            missing = [field for field in required if not row.get(field)]
            if missing:
                errors.append(f"trial line {line}: complete row missing {', '.join(missing)}")
                continue
            try:
                start = parse_utc(row["window_start"])
                end = parse_utc(row["window_end"])
                Decimal(row["value"])
                sample_count = int(row["sample_count"])
                if end <= start or sample_count <= 0:
                    raise ValueError("nonpositive window or sample")
                if require_complete and (end - start).total_seconds() < 14 * 24 * 60 * 60:
                    errors.append(f"trial line {line}: scored window is shorter than 14 days")
            except (ValueError, InvalidOperation) as error:
                errors.append(f"trial line {line}: invalid complete value ({error})")
        elif require_complete and status == "pending":
            errors.append(f"trial line {line}: evidence is still pending")
    for provider in sorted(PROVIDERS):
        missing = TRIAL_METRICS - global_metrics[provider]
        if missing:
            errors.append(f"trial: {provider} missing global metrics {sorted(missing)}")
    return errors


def validate_cost(rows: list[dict[str, str]], require_complete: bool) -> list[str]:
    errors: list[str] = []
    seen: set[tuple[str, str, str]] = set()
    for line, row in enumerate(rows, start=2):
        provider = row.get("provider", "")
        scenario = row.get("scenario", "")
        behavior = row.get("behavior", "")
        key = (provider, scenario, behavior)
        if provider not in PROVIDERS:
            errors.append(f"cost line {line}: unsupported provider {provider!r}")
        if scenario not in COST_SCENARIOS:
            errors.append(f"cost line {line}: unsupported scenario {scenario!r}")
        elif row.get("simultaneous_flights") != str(COST_SCENARIOS[scenario]):
            errors.append(f"cost line {line}: wrong simultaneous_flights for {scenario}")
        if behavior not in COST_BEHAVIORS:
            errors.append(f"cost line {line}: unsupported behavior {behavior!r}")
        if key in seen:
            errors.append(f"cost line {line}: duplicate provider/scenario")
        seen.add(key)
        status = row.get("status", "")
        if status not in {"pending", "complete"}:
            errors.append(f"cost line {line}: unsupported status {status!r}")
        if status == "complete":
            required = (
                "currency",
                "total_monthly",
                "quote_evidence_id",
                *MONEY_FIELDS,
            )
            missing = [field for field in required if not row.get(field)]
            if missing:
                errors.append(f"cost line {line}: complete row missing {', '.join(missing)}")
                continue
            try:
                components = [Decimal(row.get(field) or "0") for field in MONEY_FIELDS]
                total = Decimal(row["total_monthly"])
                if total != sum(components, Decimal("0")):
                    errors.append(f"cost line {line}: total_monthly does not equal component sum")
            except InvalidOperation:
                errors.append(f"cost line {line}: monetary fields must be decimal numbers")
        elif require_complete:
            errors.append(f"cost line {line}: price is still pending")
    expected = {
        (provider, scenario, behavior)
        for provider in PROVIDERS
        for scenario in COST_SCENARIOS
        for behavior in COST_BEHAVIORS
    }
    missing = expected - seen
    if missing:
        errors.append(f"cost: missing scenarios {sorted(missing)}")
    return errors


def validate(directory: Path, require_complete: bool = False) -> list[str]:
    missing_package_files = sorted(
        name for name in REQUIRED_FILES if not (directory / name).is_file()
    )
    if missing_package_files:
        return [f"missing required files: {', '.join(missing_package_files)}"]

    trial = directory / "trial-scorecard.csv"
    cost = directory / "cost-model.csv"
    responses = directory / "provider-question-responses.csv"
    scores = directory / "provider-scores.csv"
    decision = directory / "provider-decision.csv"
    trial_rows, trial_errors = validate_table(trial, TRIAL_COLUMNS)
    cost_rows, cost_errors = validate_table(cost, COST_COLUMNS)
    response_rows, response_errors = validate_table(responses, RESPONSE_COLUMNS)
    score_rows, score_errors = validate_table(scores, SCORE_COLUMNS)
    decision_rows, decision_errors = validate_table(decision, DECISION_COLUMNS)
    evidence_rows, evidence_table_errors = markdown_table(
        directory / "EVIDENCE_REGISTER.md", EVIDENCE_COLUMNS
    )
    return (
        validate_questionnaire(directory / "RIGHTS_AND_SERVICE_QUESTIONNAIRE.md")
        + evidence_table_errors
        + validate_evidence_register(evidence_rows, require_complete)
        + response_errors
        + validate_question_responses(response_rows, require_complete)
        + score_errors
        + validate_scores(score_rows, require_complete)
        + decision_errors
        + validate_decision(
            decision_rows,
            score_rows,
            response_rows,
            evidence_rows,
            require_complete,
        )
        + validate_decision_log(directory.parent / "DECISIONS.md", decision_rows)
        + trial_errors
        + cost_errors
        + validate_trial(trial_rows, require_complete)
        + validate_cost(cost_rows, require_complete)
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--directory",
        type=Path,
        default=Path("plans/provider-evaluation"),
        help="directory containing the FT-301 CSV evidence",
    )
    parser.add_argument(
        "--require-complete",
        action="store_true",
        help="fail when trial or price evidence remains pending or under 14 days",
    )
    args = parser.parse_args()
    errors = validate(args.directory, args.require_complete)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    mode = "complete" if args.require_complete else "structurally valid"
    print(f"FT-301 evidence is {mode}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
