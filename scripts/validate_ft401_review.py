#!/usr/bin/env python3
"""Validate the FT-401 security, privacy, and trust review package."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path
import sys


FINDINGS_COLUMNS = (
    "finding_id",
    "severity",
    "status",
    "title",
    "evidence",
    "owner",
    "deadline_gate",
    "resolution",
    "verification",
)
REQUIRED_FINDING_IDS = {f"F401-{number:03d}" for number in range(1, 11)}
REQUIRED_FILES = {
    "SECURITY_PRIVACY_TRUST_REVIEW.md",
    "DATA_LIFECYCLE_INCIDENT_POLICY.md",
    "SECURITY_FINDINGS.csv",
}
SEVERITIES = {"critical", "high", "medium", "low"}
STATUSES = {"open", "blocked", "controlled", "closed"}
INCOMPLETE_VERIFICATION = {"", "pending", "not verified", "n/a"}


def validate(directory: Path, require_complete: bool = False) -> list[str]:
    """Return validation errors for a review package directory."""
    errors: list[str] = []
    missing_files = sorted(name for name in REQUIRED_FILES if not (directory / name).is_file())
    if missing_files:
        return [f"missing required FT-401 files: {missing_files}"]

    review_text = (directory / "SECURITY_PRIVACY_TRUST_REVIEW.md").read_text(
        encoding="utf-8"
    )
    with (directory / "SECURITY_FINDINGS.csv").open(
        newline="", encoding="utf-8"
    ) as source:
        reader = csv.DictReader(source)
        if tuple(reader.fieldnames or ()) != FINDINGS_COLUMNS:
            errors.append(
                "SECURITY_FINDINGS.csv: columns must be exactly "
                f"{list(FINDINGS_COLUMNS)}"
            )
            return errors
        rows = list(reader)

    seen: set[str] = set()
    for line_number, row in enumerate(rows, start=2):
        finding_id = row["finding_id"].strip()
        severity = row["severity"].strip()
        status = row["status"].strip()

        if finding_id in seen:
            errors.append(
                f"SECURITY_FINDINGS.csv line {line_number}: duplicate finding ID {finding_id}"
            )
        seen.add(finding_id)
        if finding_id not in REQUIRED_FINDING_IDS:
            errors.append(
                f"SECURITY_FINDINGS.csv line {line_number}: unsupported finding ID {finding_id!r}"
            )
        if severity not in SEVERITIES:
            errors.append(
                f"SECURITY_FINDINGS.csv line {line_number}: unsupported severity {severity!r}"
            )
        if status not in STATUSES:
            errors.append(
                f"SECURITY_FINDINGS.csv line {line_number}: unsupported status {status!r}"
            )

        for column in FINDINGS_COLUMNS[3:]:
            if not row[column].strip():
                errors.append(
                    f"SECURITY_FINDINGS.csv line {line_number}: {column} is required"
                )

        if finding_id and finding_id not in review_text:
            errors.append(
                f"SECURITY_PRIVACY_TRUST_REVIEW.md: missing traceability for {finding_id}"
            )

        verification = row["verification"].strip().lower()
        if status in {"closed", "controlled"} and verification in INCOMPLETE_VERIFICATION:
            errors.append(
                f"SECURITY_FINDINGS.csv line {line_number}: {status} finding needs verification evidence"
            )
        if status == "controlled" and severity in {"critical", "high"}:
            errors.append(
                f"SECURITY_FINDINGS.csv line {line_number}: {severity} finding must be closed, not controlled"
            )

        if require_complete:
            if status == "blocked":
                errors.append(f"SECURITY_FINDINGS.csv: {finding_id} remains blocked")
            if severity in {"critical", "high"} and status != "closed":
                errors.append(
                    f"SECURITY_FINDINGS.csv: {finding_id} {severity} finding is not closed"
                )
            if severity in {"medium", "low"} and status == "open":
                errors.append(
                    f"SECURITY_FINDINGS.csv: {finding_id} needs closure or documented risk acceptance"
                )

    missing_ids = sorted(REQUIRED_FINDING_IDS - seen)
    if missing_ids:
        errors.append(f"SECURITY_FINDINGS.csv: missing finding IDs {missing_ids}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "directory",
        nargs="?",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "plans",
    )
    parser.add_argument(
        "--require-complete",
        action="store_true",
        help="also enforce the FT-401 approval rule",
    )
    args = parser.parse_args()
    errors = validate(args.directory, args.require_complete)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("FT-401 review package is valid.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
