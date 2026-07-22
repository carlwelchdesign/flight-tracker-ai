import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))

from verify_ft404_public_surface import (
    HttpResponse,
    SurfaceConfig,
    VerificationConfigurationError,
    validate_https_origin,
    verify_public_surface,
)


WEB = "https://portfolio.example.test"
API = "https://api.example.test"
SECURITY_HEADERS = {
    "content-security-policy": "default-src 'self'",
    "cross-origin-opener-policy": "same-origin-allow-popups",
    "cross-origin-resource-policy": "same-origin",
    "referrer-policy": "strict-origin-when-cross-origin",
    "strict-transport-security": "max-age=31536000",
    "x-content-type-options": "nosniff",
    "x-frame-options": "DENY",
    "x-permitted-cross-domain-policies": "none",
}


def response(status, body=b"", headers=None):
    return HttpResponse(status, headers or {}, body)


def json_response(status, value, headers=None):
    return response(
        status,
        json.dumps(value).encode(),
        {"content-type": "application/json", **(headers or {})},
    )


class FakeClient:
    def __init__(self, responses):
        self.responses = responses

    def get(self, origin, path):
        return self.responses[(origin, path)]


def healthy_api():
    hsts = {"strict-transport-security": "max-age=63072000; includeSubDomains"}
    return {
        (API, "/health"): json_response(200, {"status": "ok"}, hsts),
        (API, "/readiness"): json_response(200, {"status": "ready"}, hsts),
        (API, "/api/system/health"): json_response(
            401,
            {
                "error": {
                    "code": "authentication_required",
                    "message": "A valid session is required",
                }
            },
        ),
    }


def public_tracker_body():
    return (
        b'<main><p>Flight Tracker AI</p><span>Realtime regional aircraft explorer</span>'
        b'<label>Traffic region</label><section>Current picture</section>'
        b'</main>'
    )


class PublicSurfaceVerificationTest(unittest.TestCase):
    def test_publication_ready_surface_passes_exact_contracts(self):
        responses = healthy_api()
        responses[(WEB, "/")] = response(
            200,
            public_tracker_body(),
            {"content-type": "text/html; charset=utf-8", **SECURITY_HEADERS},
        )

        evidence = verify_public_surface(
            SurfaceConfig("production-candidate-1", WEB, API),
            FakeClient(responses),
        )

        self.assertEqual(evidence["status"], "passed")
        self.assertTrue(evidence["publication_ready"])
        self.assertFalse(evidence["summary"]["deployment_protected"])
        self.assertIn(
            {"check": "web_public_flight_tracker", "status": "passed"},
            evidence["checks"],
        )

    def test_public_sign_in_wall_is_rejected(self):
        responses = healthy_api()
        responses[(WEB, "/")] = response(
            200,
            b'<main><h1>Sign in to continue</h1><a href="/sign-in">Open secure sign in</a></main>',
            {"content-type": "text/html; charset=utf-8", **SECURITY_HEADERS},
        )

        evidence = verify_public_surface(
            SurfaceConfig("production-sign-in-wall", WEB, API),
            FakeClient(responses),
        )

        self.assertEqual(evidence["status"], "failed")
        self.assertFalse(evidence["publication_ready"])

    def test_public_tracker_rejects_protected_console_invitation(self):
        responses = healthy_api()
        responses[(WEB, "/")] = response(
            200,
            public_tracker_body().replace(
                b"</main>",
                b'<a href="/sign-in">Protected operations console</a></main>',
            ),
            {"content-type": "text/html; charset=utf-8", **SECURITY_HEADERS},
        )

        evidence = verify_public_surface(
            SurfaceConfig("production-auth-prompt", WEB, API),
            FakeClient(responses),
        )

        self.assertEqual(evidence["status"], "failed")
        self.assertFalse(evidence["publication_ready"])

    def test_public_root_rejects_unbounded_html(self):
        responses = healthy_api()
        responses[(WEB, "/")] = response(
            200,
            b'<main><h1>Console</h1></main>',
            {"content-type": "text/html; charset=utf-8", **SECURITY_HEADERS},
        )

        evidence = verify_public_surface(
            SurfaceConfig("production-open-root", WEB, API),
            FakeClient(responses),
        )

        self.assertEqual(evidence["status"], "failed")
        self.assertFalse(evidence["publication_ready"])

    def test_protected_preview_passes_but_is_not_publication_ready(self):
        responses = healthy_api()
        responses[(WEB, "/")] = response(
            302,
            headers={"location": "https://vercel.com/sso-api?url=redacted"},
        )

        evidence = verify_public_surface(
            SurfaceConfig("preview-1", WEB, API, allow_deployment_protection=True),
            FakeClient(responses),
        )

        self.assertEqual(evidence["status"], "passed")
        self.assertFalse(evidence["publication_ready"])
        self.assertTrue(evidence["summary"]["deployment_protected"])

    def test_structured_vercel_protection_response_passes_for_preview(self):
        responses = healthy_api()
        responses[(WEB, "/")] = json_response(
            401,
            {
                "error": {"code": "401", "message": "Authentication Required"},
                "protection": {
                    "vercel_auth_enabled": True,
                    "password_enabled": False,
                },
            },
            {"server": "Vercel", "x-vercel-id": "iad1::safe-reference"},
        )

        evidence = verify_public_surface(
            SurfaceConfig("preview-structured-401", WEB, API, allow_deployment_protection=True),
            FakeClient(responses),
        )

        self.assertEqual(evidence["status"], "passed")
        self.assertFalse(evidence["publication_ready"])
        self.assertTrue(evidence["summary"]["deployment_protected"])

    def test_unknown_redirect_and_open_api_fail_closed(self):
        responses = healthy_api()
        responses[(WEB, "/")] = response(302, headers={"location": "https://evil.example/"})
        responses[(API, "/api/system/health")] = json_response(200, {"status": "ok"})

        evidence = verify_public_surface(
            SurfaceConfig("candidate-2", WEB, API),
            FakeClient(responses),
        )

        self.assertEqual(evidence["status"], "failed")
        self.assertFalse(evidence["publication_ready"])
        self.assertEqual(len(evidence["failures"]), 2)

    def test_missing_security_headers_fail_without_naming_attacker_content(self):
        responses = healthy_api()
        responses[(WEB, "/")] = response(
            200,
            public_tracker_body(),
            {
                "content-type": "text/html; charset=utf-8",
                "x-untrusted-secret": "do-not-echo",
            },
        )

        evidence = verify_public_surface(
            SurfaceConfig("candidate-3", WEB, API),
            FakeClient(responses),
        )

        serialized = json.dumps(evidence)
        self.assertEqual(evidence["status"], "failed")
        self.assertNotIn("do-not-echo", serialized)
        self.assertNotIn(WEB, serialized)
        self.assertNotIn(API, serialized)

    def test_rejects_non_https_or_credential_bearing_origins(self):
        for value in (
            "http://example.test",
            "https://user:secret@example.test",
            "https://example.test/path",
            "https://example.test?token=secret",
        ):
            with self.assertRaises(VerificationConfigurationError):
                validate_https_origin(value)


if __name__ == "__main__":
    unittest.main()
