import csv
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

from validate_ft301_evidence import MONEY_FIELDS, validate


ROOT = Path(__file__).resolve().parents[1]


class Ft301EvidenceValidationTest(unittest.TestCase):
    def test_checked_in_templates_are_structurally_valid_but_incomplete(self) -> None:
        directory = ROOT / "plans" / "provider-evaluation"
        self.assertEqual(validate(directory), [])
        self.assertTrue(any("pending" in error for error in validate(directory, True)))

    def test_rejects_duplicate_metrics_and_bad_cost_totals(self) -> None:
        source = ROOT / "plans" / "provider-evaluation"
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "provider-evaluation"
            shutil.copytree(source, directory)
            with (source / "trial-scorecard.csv").open(encoding="utf-8") as trial_source:
                trial_rows = list(csv.DictReader(trial_source))
            trial_rows.append(trial_rows[0].copy())
            self._write(directory / "trial-scorecard.csv", trial_rows)
            with (source / "cost-model.csv").open(encoding="utf-8") as cost_source:
                cost_rows = list(csv.DictReader(cost_source))
            cost_rows[0].update({field: "0" for field in MONEY_FIELDS})
            cost_rows[0].update({
                "status": "complete",
                "currency": "USD",
                "fixed_monthly": "100",
                "usage_monthly": "25",
                "total_monthly": "999",
            })
            self._write(directory / "cost-model.csv", cost_rows)
            errors = validate(directory)
            self.assertTrue(any("duplicate" in error for error in errors))
            self.assertTrue(any("component sum" in error for error in errors))

    def test_rejects_missing_right_and_invalid_evidence_status(self) -> None:
        source = ROOT / "plans" / "provider-evaluation"
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "provider-evaluation"
            shutil.copytree(source, directory)
            questionnaire = directory / "RIGHTS_AND_SERVICE_QUESTIONNAIRE.md"
            questionnaire.write_text(
                questionnaire.read_text(encoding="utf-8").replace(
                    "| R-21 | Does the Order expressly override",
                    "| R-22 | Does the Order expressly override",
                ),
                encoding="utf-8",
            )
            register = directory / "EVIDENCE_REGISTER.md"
            register.write_text(
                register.read_text(encoding="utf-8").replace(
                    "| FA-RIGHTS | FlightAware Firehose | Rights and license | missing |",
                    "| FA-RIGHTS | FlightAware Firehose | Rights and license | guessed |",
                ).replace(
                    "| FA-SLA | FlightAware Firehose | SLA and support | missing |",
                    "| FA-SLA | FlightAware Firehose | SLA and support | received |",
                ),
                encoding="utf-8",
            )
            errors = validate(directory)
            self.assertTrue(any("missing question IDs" in error for error in errors))
            self.assertTrue(any("unsupported question IDs" in error for error in errors))
            self.assertTrue(any("unsupported status" in error for error in errors))
            self.assertTrue(any("controlled reference" in error for error in errors))
            self.assertTrue(any("received date" in error for error in errors))

    def test_rejects_incomplete_or_inconsistent_question_responses(self) -> None:
        source = ROOT / "plans" / "provider-evaluation"
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "provider-evaluation"
            shutil.copytree(source, directory)
            response_path = directory / "provider-question-responses.csv"
            with response_path.open(encoding="utf-8") as response_source:
                rows = list(csv.DictReader(response_source))
            rows[0].update({"answer": "yes", "review_status": "accepted"})
            rows[1].update({
                "answer": "no",
                "controlling_clause": "Order section 1",
                "limitations": "none",
                "additional_fee": "none",
                "review_status": "accepted",
            })
            rows[2]["evidence_id"] = "CI-RIGHTS"
            rows.pop()
            self._write(response_path, rows)
            errors = validate(directory)
            self.assertTrue(any("answered row missing" in error for error in errors))
            self.assertTrue(any("incompatible" in error for error in errors))
            self.assertTrue(any("evidence_id must be" in error for error in errors))
            self.assertTrue(any("missing provider/question rows" in error for error in errors))

    def test_rejects_invalid_scores_and_unsupported_selection(self) -> None:
        source = ROOT / "plans" / "provider-evaluation"
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "provider-evaluation"
            shutil.copytree(source, directory)
            score_path = directory / "provider-scores.csv"
            with score_path.open(encoding="utf-8") as score_source:
                scores = list(csv.DictReader(score_source))
            scores[0].update({
                "weight": "99",
                "points": "6",
                "status": "complete",
                "notes": "out-of-range test",
            })
            scores[1].update({"points": "3", "status": "excluded"})
            scores.pop()
            self._write(score_path, scores)

            decision_path = directory / "provider-decision.csv"
            with decision_path.open(encoding="utf-8") as decision_source:
                decisions = list(csv.DictReader(decision_source))
            decisions[0].update({
                "decision": "select",
                "selected_provider": "flightaware_firehose",
                "fallback": "flightaware_firehose",
                "scoring_method_version": "latest",
                "decision_date": "2026/07/21",
                "od_002_status": "resolved",
            })
            self._write(decision_path, decisions)
            shutil.copy(ROOT / "plans" / "DECISIONS.md", directory.parent / "DECISIONS.md")
            copied_decisions = directory.parent / "DECISIONS.md"
            copied_decisions.write_text(
                copied_decisions.read_text(encoding="utf-8").replace(
                    "OD-002 is resolved",
                    "OD-002 remains pending",
                )
                + "\n| OD-002 | Test open decision | FT-301 | Test evidence |\n",
                encoding="utf-8",
            )

            errors = validate(directory, require_complete=True)
            self.assertTrue(any("weight does not match" in error for error in errors))
            self.assertTrue(any("points must be between" in error for error in errors))
            self.assertTrue(any("excluded score cannot have points" in error for error in errors))
            self.assertTrue(any("missing provider/dimension" in error for error in errors))
            self.assertTrue(any("fallback must differ" in error for error in errors))
            self.assertTrue(any("scoring_method_version" in error for error in errors))
            self.assertTrue(any("decision_date must use" in error for error in errors))
            self.assertTrue(any("every score completed" in error for error in errors))
            self.assertTrue(any("pending or rejected response" in error for error in errors))
            self.assertTrue(any("unaccepted required evidence" in error for error in errors))
            self.assertTrue(any("still listed under open decisions" in error for error in errors))
            self.assertTrue(any("resolution statement is missing" in error for error in errors))

    @staticmethod
    def _write(path: Path, rows: list[dict[str, str]]) -> None:
        with path.open("w", newline="", encoding="utf-8") as target:
            writer = csv.DictWriter(target, fieldnames=rows[0].keys())
            writer.writeheader()
            writer.writerows(rows)


if __name__ == "__main__":
    unittest.main()
