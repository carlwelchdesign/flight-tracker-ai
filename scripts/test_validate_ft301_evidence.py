import csv
from pathlib import Path
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
            directory = Path(temporary)
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

    @staticmethod
    def _write(path: Path, rows: list[dict[str, str]]) -> None:
        with path.open("w", newline="", encoding="utf-8") as target:
            writer = csv.DictWriter(target, fieldnames=rows[0].keys())
            writer.writeheader()
            writer.writerows(rows)


if __name__ == "__main__":
    unittest.main()
