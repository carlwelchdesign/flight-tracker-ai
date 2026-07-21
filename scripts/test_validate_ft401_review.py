import csv
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

from validate_ft401_review import validate


ROOT = Path(__file__).resolve().parents[1]


class Ft401ReviewValidationTest(unittest.TestCase):
    def test_checked_in_review_is_structurally_valid_but_incomplete(self) -> None:
        directory = ROOT / "plans"
        self.assertEqual(validate(directory), [])
        completion_errors = validate(directory, require_complete=True)
        self.assertTrue(any("remains blocked" in error for error in completion_errors))
        self.assertTrue(any("is not closed" in error for error in completion_errors))
        self.assertTrue(any("risk acceptance" in error for error in completion_errors))

    def test_rejects_duplicate_missing_and_unsupported_findings(self) -> None:
        with self._copy_review() as directory:
            path = directory / "SECURITY_FINDINGS.csv"
            rows = self._read(path)
            rows[1]["finding_id"] = rows[0]["finding_id"]
            rows[2]["severity"] = "urgent"
            rows.pop()
            self._write(path, rows)

            errors = validate(directory)
            self.assertTrue(any("duplicate finding ID" in error for error in errors))
            self.assertTrue(any("unsupported severity" in error for error in errors))
            self.assertTrue(any("missing finding IDs" in error for error in errors))

    def test_rejects_closed_finding_without_verification(self) -> None:
        with self._copy_review() as directory:
            path = directory / "SECURITY_FINDINGS.csv"
            rows = self._read(path)
            rows[0].update({"status": "closed", "verification": "Pending"})
            self._write(path, rows)

            errors = validate(directory)
            self.assertTrue(any("needs verification evidence" in error for error in errors))

    def _copy_review(self):
        temporary = tempfile.TemporaryDirectory()
        directory = Path(temporary.name) / "plans"
        directory.mkdir()
        for name in (
            "SECURITY_PRIVACY_TRUST_REVIEW.md",
            "DATA_LIFECYCLE_INCIDENT_POLICY.md",
            "SECURITY_FINDINGS.csv",
        ):
            shutil.copy(ROOT / "plans" / name, directory / name)
        return _TemporaryReview(temporary, directory)

    @staticmethod
    def _read(path: Path) -> list[dict[str, str]]:
        with path.open(newline="", encoding="utf-8") as source:
            return list(csv.DictReader(source))

    @staticmethod
    def _write(path: Path, rows: list[dict[str, str]]) -> None:
        with path.open("w", newline="", encoding="utf-8") as target:
            writer = csv.DictWriter(target, fieldnames=rows[0].keys())
            writer.writeheader()
            writer.writerows(rows)


class _TemporaryReview:
    def __init__(self, temporary: tempfile.TemporaryDirectory, path: Path) -> None:
        self._temporary = temporary
        self.path = path

    def __enter__(self) -> Path:
        return self.path

    def __exit__(self, *_: object) -> None:
        self._temporary.cleanup()


if __name__ == "__main__":
    unittest.main()
