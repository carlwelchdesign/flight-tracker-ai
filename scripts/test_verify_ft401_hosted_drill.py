import json
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, HTTPServer
from io import BytesIO
from pathlib import Path
import sys
import threading
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

from verify_ft401_hosted_drill import (
    BoundedHttpClient,
    DrillConfig,
    DrillConfigurationError,
    HttpResponse,
    validate_base_url,
    verify_hosted_drill,
)


NOW = datetime(2026, 7, 21, 18, 0, tzinfo=timezone.utc)
EXPECTED_IDS = frozenset(
    {"00000000-0000-0000-0000-000000000101", "00000000-0000-0000-0000-000000000102"}
)
FORBIDDEN_ID = "00000000-0000-0000-0000-000000000999"
SECRET_MARKER = "controlled-test-token-123456"
EMAIL_MARKER = "private.person@example.test"


class FakeClient:
    def __init__(self, responses=None):
        self.responses = responses or successful_responses()

    def get(self, authorization: str, path: str) -> HttpResponse:
        role = authorization.removesuffix("-auth")
        name = next(
            endpoint
            for endpoint in ("events", "export", "signals", "integrity")
            if endpoint_path_matches(endpoint, path)
        )
        return self.responses[(role, name)]


def endpoint_path_matches(name: str, path: str) -> bool:
    return {
        "events": "/admin/audit-events?limit=250" in path,
        "export": "/admin/audit-events/export?" in path,
        "signals": "/admin/audit-alerts?" in path,
        "integrity": path == "/admin/retention/integrity",
    }[name]


def json_response(value: object, status: int = 200) -> HttpResponse:
    return HttpResponse(status, {"content-type": "application/json"}, json.dumps(value).encode())


def successful_responses():
    signals = [
        {
            "code": "high_risk_action",
            "severity": "warning",
            "event_id": "00000000-0000-0000-0000-000000000001",
        },
        {
            "code": "privileged_action_burst",
            "severity": "critical",
            "event_id": "00000000-0000-0000-0000-000000000002",
        },
        {
            "code": "sensitive_write_detected",
            "severity": "critical",
            "event_id": "00000000-0000-0000-0000-000000000101",
        },
        {
            "code": "sensitive_write_detected",
            "severity": "warning",
            "event_id": "00000000-0000-0000-0000-000000000102",
        },
    ]
    responses = {
        ("admin", "events"): json_response({"data": [], "from": "x", "to": "y"}),
        ("admin", "export"): HttpResponse(
            200,
            {
                "content-type": "text/csv; charset=utf-8",
                "cache-control": "no-store",
                "content-disposition": "attachment; filename=audit.csv",
            },
            b"occurred_at,source\r\n",
        ),
        ("admin", "signals"): json_response({"data": signals, "since": "x", "through": "y"}),
        ("admin", "integrity"): json_response(
            {
                "operator_id": "00000000-0000-0000-0000-000000000010",
                "healthy": True,
                "violations": {
                    "raw_payloads": 0,
                    "authorization_audit": 0,
                    "session_revocations": 0,
                    "identity_mappings": 0,
                    "alert_history": 0,
                    "operational_facts": 0,
                },
                "paused_schedules": 0,
                "failed_attempts_24h": 0,
            }
        ),
    }
    for role in ("viewer", "operator"):
        for name in ("events", "export", "signals", "integrity"):
            responses[(role, name)] = json_response({"error": {"code": "authorization_denied"}}, 403)
    return responses


def config() -> DrillConfig:
    return DrillConfig(
        environment_reference="preview-audit-drill-001",
        admin_authorization="admin-auth",
        viewer_authorization="viewer-auth",
        operator_authorization="operator-auth",
        expected_signal_event_ids=EXPECTED_IDS,
        forbidden_event_ids=frozenset({FORBIDDEN_ID}),
        forbidden_markers=(SECRET_MARKER, EMAIL_MARKER),
    )


class HostedDrillVerificationTest(unittest.TestCase):
    def test_passes_complete_redacted_tenant_safe_contract(self) -> None:
        evidence = verify_hosted_drill(config(), FakeClient(), NOW)

        self.assertEqual(evidence["status"], "passed")
        self.assertEqual(evidence["summary"]["matched_expected_signal_event_ids"], 2)
        self.assertEqual(evidence["summary"]["retention_integrity"]["violation_total"], 0)
        serialized = json.dumps(evidence)
        self.assertNotIn(SECRET_MARKER, serialized)
        self.assertNotIn(EMAIL_MARKER, serialized)
        self.assertNotIn("admin-auth", serialized)

    def test_fails_without_echoing_leaked_content(self) -> None:
        responses = successful_responses()
        responses[("admin", "export")] = HttpResponse(
            200,
            {
                "content-type": "text/csv",
                "cache-control": "no-store",
                "content-disposition": "attachment",
            },
            f"unsafe,{SECRET_MARKER}\r\n".encode(),
        )

        evidence = verify_hosted_drill(config(), FakeClient(responses), NOW)

        self.assertEqual(evidence["status"], "failed")
        serialized = json.dumps(evidence)
        self.assertIn("marker indexes", serialized)
        self.assertNotIn(SECRET_MARKER, serialized)

    def test_fails_when_role_or_tenant_boundary_is_open(self) -> None:
        responses = successful_responses()
        responses[("viewer", "events")] = json_response({"data": []})
        signals = json.loads(responses[("admin", "signals")].body)
        signals["data"].append(
            {"code": "sensitive_write_detected", "severity": "critical", "event_id": FORBIDDEN_ID}
        )
        responses[("admin", "signals")] = json_response(signals)

        evidence = verify_hosted_drill(config(), FakeClient(responses), NOW)

        self.assertEqual(evidence["status"], "failed")
        self.assertTrue(any("viewer events" in failure for failure in evidence["failures"]))
        self.assertTrue(any("cross-tenant" in failure for failure in evidence["failures"]))
        self.assertNotIn(FORBIDDEN_ID, json.dumps(evidence))

    def test_fails_when_expected_ids_are_not_both_sensitive_severities(self) -> None:
        responses = successful_responses()
        signals = json.loads(responses[("admin", "signals")].body)
        signals["data"][-1]["code"] = "high_risk_action"
        signals["data"][-1]["severity"] = "warning"
        responses[("admin", "signals")] = json_response(signals)

        evidence = verify_hosted_drill(config(), FakeClient(responses), NOW)

        self.assertEqual(evidence["status"], "failed")
        self.assertTrue(any("both critical and warning" in failure for failure in evidence["failures"]))

    def test_unknown_signal_codes_cannot_enter_sanitized_evidence(self) -> None:
        responses = successful_responses()
        signals = json.loads(responses[("admin", "signals")].body)
        untrusted_code = "untrusted-sensitive-code-value"
        signals["data"].append(
            {"code": untrusted_code, "severity": "warning", "event_id": None}
        )
        responses[("admin", "signals")] = json_response(signals)

        evidence = verify_hosted_drill(config(), FakeClient(responses), NOW)

        self.assertEqual(evidence["status"], "failed")
        self.assertNotIn(untrusted_code, json.dumps(evidence))

    def test_fails_unhealthy_or_undispositioned_retention_state(self) -> None:
        responses = successful_responses()
        integrity = json.loads(responses[("admin", "integrity")].body)
        integrity["paused_schedules"] = 1
        responses[("admin", "integrity")] = json_response(integrity)

        evidence = verify_hosted_drill(config(), FakeClient(responses), NOW)

        self.assertEqual(evidence["status"], "failed")
        self.assertTrue(any("retention integrity" in failure for failure in evidence["failures"]))

    def test_configuration_rejects_unsafe_transport_and_credentials(self) -> None:
        with self.assertRaises(DrillConfigurationError):
            validate_base_url("http://example.test", allow_loopback_http=True)
        with self.assertRaises(DrillConfigurationError):
            validate_base_url("https://user:secret@example.test", allow_loopback_http=False)
        with self.assertRaises(DrillConfigurationError):
            BoundedHttpClient("https://example.test", "/api", "X-Unsafe-Auth")
        with self.assertRaises(DrillConfigurationError):
            BoundedHttpClient("https://example.test", "/api path", "Authorization")
        unsafe = config().__class__(
            **{**config().__dict__, "admin_authorization": "Bearer safe\nInjected: value"}
        )
        with self.assertRaises(DrillConfigurationError):
            unsafe.validate()

    def test_http_client_does_not_follow_authentication_bearing_redirects(self) -> None:
        class Handler(BaseHTTPRequestHandler):
            authorization = None

            def do_GET(self):
                Handler.authorization = self.headers.get("Authorization")
                self.send_response(302)
                self.send_header("Location", "/api/redirect-target")
                self.end_headers()

            def log_message(self, *_args):
                pass

        server = HTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            client = BoundedHttpClient(
                f"http://127.0.0.1:{server.server_port}",
                "/api",
                "Authorization",
                allow_loopback_http=True,
            )
            response = client.get("Bearer controlled-test", "/redirect")
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2)

        self.assertEqual(response.status, 302)
        self.assertEqual(Handler.authorization, "Bearer controlled-test")

    def test_http_client_rejects_oversized_responses(self) -> None:
        with self.assertRaises(RuntimeError):
            BoundedHttpClient._bounded_response(
                200,
                {"content-type": "application/json"},
                BytesIO(b"x" * (2 * 1024 * 1024 + 1)),
            )


if __name__ == "__main__":
    unittest.main()
