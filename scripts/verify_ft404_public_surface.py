#!/usr/bin/env python3
"""Verify the sanitized public FT-404 web and API boundary."""

from __future__ import annotations

import argparse
from dataclasses import dataclass, field
import json
import re
import sys
from typing import Mapping, Protocol
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener


MAX_RESPONSE_BYTES = 1024 * 1024
SAFE_REFERENCE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$")
REDIRECT_STATUSES = frozenset({301, 302, 303, 307, 308})
REQUIRED_WEB_HEADERS = {
    "cross-origin-opener-policy": "same-origin-allow-popups",
    "cross-origin-resource-policy": "same-origin",
    "referrer-policy": "strict-origin-when-cross-origin",
    "strict-transport-security": "max-age=31536000",
    "x-content-type-options": "nosniff",
    "x-frame-options": "DENY",
    "x-permitted-cross-domain-policies": "none",
}


class VerificationConfigurationError(ValueError):
    """The verifier configuration is incomplete or unsafe."""


@dataclass(frozen=True)
class HttpResponse:
    status: int
    headers: Mapping[str, str]
    body: bytes = field(repr=False)


class SurfaceHttpClient(Protocol):
    def get(self, origin: str, path: str) -> HttpResponse: ...


@dataclass(frozen=True)
class SurfaceConfig:
    environment_reference: str
    web_origin: str = field(repr=False)
    api_origin: str = field(repr=False)
    allow_deployment_protection: bool = False

    def validate(self) -> None:
        if not SAFE_REFERENCE.fullmatch(self.environment_reference):
            raise VerificationConfigurationError(
                "environment reference must contain 1-128 safe characters"
            )
        validate_https_origin(self.web_origin)
        validate_https_origin(self.api_origin)


class NoRedirectHandler(HTTPRedirectHandler):
    def redirect_request(self, request, file_pointer, code, message, headers, new_url):
        del request, file_pointer, code, message, headers, new_url
        return None


class BoundedHttpClient:
    def __init__(self, timeout_seconds: float = 10.0) -> None:
        if not 0 < timeout_seconds <= 30:
            raise VerificationConfigurationError(
                "timeout must be greater than zero and at most 30 seconds"
            )
        self._timeout_seconds = timeout_seconds
        self._opener = build_opener(NoRedirectHandler())

    def get(self, origin: str, path: str) -> HttpResponse:
        if not path.startswith("/") or ".." in path:
            raise VerificationConfigurationError("request path is invalid")
        request = Request(
            f"{validate_https_origin(origin)}{path}",
            headers={"Accept": "application/json,text/html", "User-Agent": "ft404-verifier/1"},
            method="GET",
        )
        try:
            with self._opener.open(request, timeout=self._timeout_seconds) as response:
                return self._read(response.status, response.headers, response)
        except HTTPError as error:
            with error:
                return self._read(error.code, error.headers, error)
        except (URLError, TimeoutError, OSError) as error:
            raise RuntimeError("public surface request failed") from error

    @staticmethod
    def _read(status: int, headers, stream) -> HttpResponse:
        body = stream.read(MAX_RESPONSE_BYTES + 1)
        if len(body) > MAX_RESPONSE_BYTES:
            raise RuntimeError("public surface response exceeded the safe size limit")
        return HttpResponse(
            status=status,
            headers={name.lower(): value for name, value in headers.items()},
            body=body,
        )


def validate_https_origin(value: str) -> str:
    parsed = urlsplit(value)
    if (
        parsed.scheme != "https"
        or not parsed.hostname
        or not parsed.netloc
        or parsed.username
        or parsed.password
        or parsed.path not in {"", "/"}
        or parsed.query
        or parsed.fragment
    ):
        raise VerificationConfigurationError(
            "hosted origins must be HTTPS origins without credentials, paths, queries, or fragments"
        )
    return f"https://{parsed.netloc}".rstrip("/")


def verify_public_surface(config: SurfaceConfig, client: SurfaceHttpClient) -> dict[str, object]:
    config.validate()
    checks: list[dict[str, str]] = []
    failures: list[str] = []
    deployment_protected = False

    try:
        root = client.get(config.web_origin, "/")
        location = root.headers.get("location", "")
        if root.status == 200 and _is_public_flight_tracker(root):
            checks.append({"check": "web_public_flight_tracker", "status": "passed"})
            _verify_web_headers(root, checks, failures)
        elif (
            _is_vercel_protection_response(root, location)
            and config.allow_deployment_protection
        ):
            deployment_protected = True
            checks.append({"check": "preview_deployment_protection", "status": "passed"})
        else:
            failures.append("web root did not expose the approved public flight-tracker boundary")
    except (RuntimeError, VerificationConfigurationError):
        failures.append("web root request failed")

    for path, expected in (("/health", {"status": "ok"}), ("/readiness", {"status": "ready"})):
        name = path.removeprefix("/")
        try:
            response = client.get(config.api_origin, path)
            if response.status != 200 or _parse_json(response) != expected:
                failures.append(f"API {name} contract did not match")
            elif "max-age=" not in response.headers.get("strict-transport-security", ""):
                failures.append(f"API {name} response did not prove HSTS")
            else:
                checks.append({"check": f"api_{name}", "status": "passed"})
        except (RuntimeError, VerificationConfigurationError, ValueError):
            failures.append(f"API {name} request failed")

    try:
        protected = client.get(config.api_origin, "/api/system/health")
        if protected.status != 401 or _parse_json(protected) != {
            "error": {
                "code": "authentication_required",
                "message": "A valid session is required",
            }
        }:
            failures.append("protected API route did not fail closed")
        else:
            checks.append({"check": "api_unauthenticated_denial", "status": "passed"})
    except (RuntimeError, VerificationConfigurationError, ValueError):
        failures.append("protected API denial request failed")

    evidence: dict[str, object] = {
        "schema_version": 1,
        "ticket": "FT-404",
        "status": "failed" if failures else "passed",
        "environment_reference": config.environment_reference,
        "publication_ready": not deployment_protected and not failures,
        "checks": checks,
        "summary": {
            "deployment_protected": deployment_protected,
            "web_header_contract_checked": not deployment_protected,
        },
    }
    if failures:
        evidence["failures"] = failures
    return evidence


def _is_public_flight_tracker(response: HttpResponse) -> bool:
    content_type = response.headers.get("content-type", "").lower()
    return (
        "text/html" in content_type
        and b"Flight Tracker AI" in response.body
        and b"Realtime regional aircraft explorer" in response.body
        and b"Traffic region" in response.body
        and b"Current picture" in response.body
        and b"Protected operations console" in response.body
        and b'href="/sign-in"' in response.body
        and b"Sign in to continue" not in response.body
    )


def _is_vercel_protection(location: str) -> bool:
    parsed = urlsplit(location)
    return parsed.scheme == "https" and parsed.netloc == "vercel.com" and parsed.path == "/sso-api"


def _is_vercel_protection_response(response: HttpResponse, location: str) -> bool:
    if response.status in REDIRECT_STATUSES:
        return _is_vercel_protection(location)
    if (
        response.status != 401
        or response.headers.get("server", "").lower() != "vercel"
        or not response.headers.get("x-vercel-id")
    ):
        return False
    try:
        payload = _parse_json(response)
    except (ValueError, json.JSONDecodeError):
        return False
    return (
        isinstance(payload, dict)
        and isinstance(payload.get("error"), dict)
        and payload["error"].get("code") == "401"
        and isinstance(payload.get("protection"), dict)
        and payload["protection"].get("vercel_auth_enabled") is True
    )


def _verify_web_headers(
    response: HttpResponse,
    checks: list[dict[str, str]],
    failures: list[str],
) -> None:
    missing = [
        name
        for name, expected in REQUIRED_WEB_HEADERS.items()
        if expected.lower() not in response.headers.get(name, "").lower()
    ]
    if "content-security-policy" not in response.headers:
        missing.append("content-security-policy")
    if missing:
        failures.append("web security-header contract did not match")
    else:
        checks.append({"check": "web_security_headers", "status": "passed"})


def _parse_json(response: HttpResponse) -> object:
    if "application/json" not in response.headers.get("content-type", ""):
        raise ValueError("response is not JSON")
    return json.loads(response.body)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--environment-reference", required=True)
    parser.add_argument("--web-origin", required=True)
    parser.add_argument("--api-origin", required=True)
    parser.add_argument("--allow-deployment-protection", action="store_true")
    parser.add_argument("--timeout-seconds", type=float, default=10.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        config = SurfaceConfig(
            environment_reference=args.environment_reference,
            web_origin=args.web_origin,
            api_origin=args.api_origin,
            allow_deployment_protection=args.allow_deployment_protection,
        )
        evidence = verify_public_surface(config, BoundedHttpClient(args.timeout_seconds))
    except VerificationConfigurationError as error:
        print(json.dumps({"ticket": "FT-404", "status": "configuration_error", "message": str(error)}))
        return 2
    print(json.dumps(evidence, indent=2, sort_keys=True))
    return 0 if evidence["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
