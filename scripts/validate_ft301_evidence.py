#!/usr/bin/env python3
"""Validate the structure and, optionally, completeness of FT-301 evidence."""

from __future__ import annotations

import argparse
import csv
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path
import sys


PROVIDERS = {"cirium_sky_stream", "flightaware_firehose"}
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
    trial = directory / "trial-scorecard.csv"
    cost = directory / "cost-model.csv"
    missing = [path.name for path in (trial, cost) if not path.is_file()]
    if missing:
        return [f"missing required files: {', '.join(missing)}"]
    trial_rows, trial_errors = validate_table(trial, TRIAL_COLUMNS)
    cost_rows, cost_errors = validate_table(cost, COST_COLUMNS)
    return (
        trial_errors
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
