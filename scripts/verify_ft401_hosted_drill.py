#!/usr/bin/env python3
"""Verify the hosted FT-401 audit and retention drill without printing sensitive data."""

from __future__ import annotations

import argparse
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
import json
import os
import re
import sys
from typing import Mapping, Protocol
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener
from uuid import UUID


MAX_RESPONSE_BYTES = 2 * 1024 * 1024
DEFAULT_TIMEOUT_SECONDS = 10.0
REQUIRED_SIGNAL_CODES = frozenset(
    {"high_risk_action", "privileged_action_burst", "sensitive_write_detected"}
)
SAFE_REFERENCE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$")
SAFE_API_PREFIX = re.compile(r"^/[A-Za-z0-9/_-]+$")
SAFE_MARKER = re.compile(r"^[A-Za-z0-9@._:+/=-]{8,512}$")


class DrillConfigurationError(ValueError):
    """The verifier was not configured safely or completely."""


@dataclass(frozen=True)
class HttpResponse:
    status: int
    headers: Mapping[str, str]
    body: bytes = field(repr=False)


class DrillHttpClient(Protocol):
    def get(self, authorization: str, path: str) -> HttpResponse: ...


@dataclass(frozen=True)
class DrillConfig:
    environment_reference: str
    admin_authorization: str = field(repr=False)
    viewer_authorization: str = field(repr=False)
    operator_authorization: str = field(repr=False)
    expected_signal_event_ids: frozenset[str]
    forbidden_event_ids: frozenset[str]
    forbidden_markers: tuple[str, ...] = field(repr=False)
    window_hours: int = 24
    allowed_paused_schedules: int = 0
    allowed_failed_attempts_24h: int = 0

    def validate(self) -> None:
        if not SAFE_REFERENCE.fullmatch(self.environment_reference):
            raise DrillConfigurationError(
                "environment reference must be 1-128 safe non-whitespace characters"
            )
        for role, value in (
            ("administrator", self.admin_authorization),
            ("viewer", self.viewer_authorization),
            ("operator", self.operator_authorization),
        ):
            if (
                not value.strip()
                or len(value.encode("utf-8")) > 16 * 1024
                or "\r" in value
                or "\n" in value
            ):
                raise DrillConfigurationError(f"{role} authentication header is invalid")
        if not 1 <= self.window_hours <= 24:
            raise DrillConfigurationError("window hours must be between 1 and 24")
        for label, value in (
            ("allowed paused schedules", self.allowed_paused_schedules),
            ("allowed failed attempts", self.allowed_failed_attempts_24h),
        ):
            if (
                isinstance(value, bool)
                or not isinstance(value, int)
                or not 0 <= value <= 10_000
            ):
                raise DrillConfigurationError(f"{label} must be between 0 and 10,000")
        if len(self.expected_signal_event_ids) < 2:
            raise DrillConfigurationError(
                "at least two expected sensitive-write signal event IDs are required"
            )
        if not self.forbidden_event_ids:
            raise DrillConfigurationError("at least one cross-tenant event ID is required")
        if self.expected_signal_event_ids & self.forbidden_event_ids:
            raise DrillConfigurationError("expected and forbidden event IDs must be disjoint")
        _validate_event_ids(self.expected_signal_event_ids | self.forbidden_event_ids)
        if len(self.forbidden_markers) < 2:
            raise DrillConfigurationError("at least two controlled forbidden markers are required")
        if len(set(self.forbidden_markers)) != len(self.forbidden_markers):
            raise DrillConfigurationError("controlled forbidden markers must be distinct")
        if any(not SAFE_MARKER.fullmatch(marker) for marker in self.forbidden_markers):
            raise DrillConfigurationError(
                "forbidden markers must be 8-512 printable token or email characters"
            )


class NoRedirectHandler(HTTPRedirectHandler):
    def redirect_request(self, request, file_pointer, code, message, headers, new_url):
        del request, file_pointer, code, message, headers, new_url
        return None


class BoundedHttpClient:
    def __init__(
        self,
        base_url: str,
        api_prefix: str,
        authentication_header: str,
        timeout_seconds: float = DEFAULT_TIMEOUT_SECONDS,
        allow_loopback_http: bool = False,
    ) -> None:
        self._origin = validate_base_url(base_url, allow_loopback_http)
        self._api_prefix = validate_api_prefix(api_prefix)
        self._authentication_header = validate_header_name(authentication_header)
        if not 0 < timeout_seconds <= 30:
            raise DrillConfigurationError("request timeout must be greater than 0 and at most 30 seconds")
        self._timeout_seconds = timeout_seconds
        self._opener = build_opener(NoRedirectHandler())

    def get(self, authorization: str, path: str) -> HttpResponse:
        if not path.startswith("/") or ".." in path:
            raise DrillConfigurationError("request path is invalid")
        request = Request(
            f"{self._origin}{self._api_prefix}{path}",
            headers={
                self._authentication_header: authorization,
                "Accept": "application/json, text/csv",
                "User-Agent": "flight-tracker-ft401-drill/1",
            },
            method="GET",
        )
        try:
            with self._opener.open(request, timeout=self._timeout_seconds) as response:
                return self._bounded_response(response.status, response.headers, response)
        except HTTPError as error:
            with error:
                return self._bounded_response(error.code, error.headers, error)
        except (URLError, TimeoutError, OSError) as error:
            raise RuntimeError("hosted drill request failed") from error

    @staticmethod
    def _bounded_response(status: int, headers, stream) -> HttpResponse:
        body = stream.read(MAX_RESPONSE_BYTES + 1)
        if len(body) > MAX_RESPONSE_BYTES:
            raise RuntimeError("hosted drill response exceeded the safe size limit")
        return HttpResponse(
            status=status,
            headers={name.lower(): value for name, value in headers.items()},
            body=body,
        )


def validate_base_url(value: str, allow_loopback_http: bool) -> str:
    parsed = urlsplit(value)
    if parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise DrillConfigurationError("base URL cannot contain credentials, query, or fragment")
    if parsed.path not in {"", "/"} or not parsed.hostname or not parsed.netloc:
        raise DrillConfigurationError("base URL must be an origin without a path")
    if parsed.scheme == "https":
        pass
    elif (
        parsed.scheme == "http"
        and allow_loopback_http
        and parsed.hostname in {"localhost", "127.0.0.1", "::1"}
    ):
        pass
    else:
        raise DrillConfigurationError("base URL must use HTTPS; HTTP is allowed only for loopback")
    return f"{parsed.scheme}://{parsed.netloc}".rstrip("/")


def validate_api_prefix(value: str) -> str:
    if not value.startswith("/") or value.endswith("/") or ".." in value:
        raise DrillConfigurationError("API prefix must be an absolute path without a trailing slash")
    parsed = urlsplit(value)
    if parsed.query or parsed.fragment or parsed.scheme or parsed.netloc:
        raise DrillConfigurationError("API prefix must contain only a path")
    if not SAFE_API_PREFIX.fullmatch(value):
        raise DrillConfigurationError("API prefix contains unsafe characters")
    return value


def validate_header_name(value: str) -> str:
    if value.lower() not in {"authorization", "cookie"}:
        raise DrillConfigurationError("authentication header must be Authorization or Cookie")
    return value


def _validate_event_ids(event_ids: frozenset[str]) -> None:
    for event_id in event_ids:
        try:
            parsed = UUID(event_id)
        except (ValueError, AttributeError) as error:
            raise DrillConfigurationError("signal event IDs must be canonical UUIDs") from error
        if str(parsed) != event_id.lower():
            raise DrillConfigurationError("signal event IDs must be canonical UUIDs")


def verify_hosted_drill(
    config: DrillConfig,
    client: DrillHttpClient,
    now: datetime | None = None,
) -> dict[str, object]:
    config.validate()
    checked_at = (now or datetime.now(timezone.utc)).astimezone(timezone.utc)
    since = checked_at - timedelta(hours=config.window_hours)
    query_from = quote(since.isoformat().replace("+00:00", "Z"), safe="")
    query_to = quote(checked_at.isoformat().replace("+00:00", "Z"), safe="")
    paths = {
        "events": "/admin/audit-events?limit=250",
        "export": f"/admin/audit-events/export?from={query_from}&to={query_to}",
        "signals": f"/admin/audit-alerts?since={query_from}",
        "integrity": "/admin/retention/integrity",
    }
    checks: list[dict[str, str]] = []
    failures: list[str] = []

    admin_responses: dict[str, HttpResponse] = {}
    for name, path in paths.items():
        try:
            response = client.get(config.admin_authorization, path)
        except (RuntimeError, DrillConfigurationError):
            failures.append(f"administrator {name} request failed")
            continue
        admin_responses[name] = response
        if response.status == 200:
            checks.append({"check": f"administrator_{name}", "status": "passed"})
        else:
            failures.append(f"administrator {name} request returned an unexpected status")

    for role, authorization in (
        ("viewer", config.viewer_authorization),
        ("operator", config.operator_authorization),
    ):
        denied = True
        for name, path in paths.items():
            try:
                response = client.get(authorization, path)
            except (RuntimeError, DrillConfigurationError):
                failures.append(f"{role} {name} denial request failed")
                denied = False
                continue
            if response.status != 403:
                failures.append(f"{role} {name} did not fail closed with 403")
                denied = False
        if denied:
            checks.append({"check": f"{role}_role_denial", "status": "passed"})

    summary: dict[str, object] = {}
    if set(admin_responses) == set(paths) and all(
        response.status == 200 for response in admin_responses.values()
    ):
        _verify_admin_contracts(config, admin_responses, checks, failures, summary)

    evidence: dict[str, object] = {
        "schema_version": 1,
        "ticket": "FT-401",
        "status": "failed" if failures else "passed",
        "checked_at": checked_at.isoformat().replace("+00:00", "Z"),
        "environment_reference": config.environment_reference,
        "checks": checks,
        "summary": summary,
    }
    if failures:
        evidence["failures"] = failures
    return evidence


def _verify_admin_contracts(
    config: DrillConfig,
    responses: Mapping[str, HttpResponse],
    checks: list[dict[str, str]],
    failures: list[str],
    summary: dict[str, object],
) -> None:
    events = _parse_json_object(responses["events"], "audit events", failures)
    signals = _parse_json_object(responses["signals"], "audit signals", failures)
    integrity = _parse_json_object(responses["integrity"], "retention integrity", failures)

    leaked_marker_indexes = []
    combined_bodies = b"\n".join(response.body for response in responses.values())
    for index, marker in enumerate(config.forbidden_markers, start=1):
        if marker.encode("utf-8") in combined_bodies:
            leaked_marker_indexes.append(index)
    if leaked_marker_indexes:
        failures.append(
            "controlled forbidden marker content appeared in a response "
            f"(marker indexes: {leaked_marker_indexes})"
        )
    else:
        checks.append({"check": "sensitive_content_redaction", "status": "passed"})

    export_headers = responses["export"].headers
    if (
        "text/csv" not in export_headers.get("content-type", "").lower()
        or "no-store" not in export_headers.get("cache-control", "").lower()
        or "attachment" not in export_headers.get("content-disposition", "").lower()
    ):
        failures.append("audit export headers are not safely bounded and non-cacheable")
    else:
        checks.append({"check": "redacted_export_headers", "status": "passed"})

    if events is not None:
        event_data = events.get("data")
        if not isinstance(event_data, list):
            failures.append("audit event response has an unexpected contract")
        else:
            summary["audit_event_count"] = len(event_data)

    if signals is not None:
        signal_data = signals.get("data")
        if not isinstance(signal_data, list):
            failures.append("audit signal response has an unexpected contract")
        else:
            signal_codes: dict[str, int] = {}
            event_ids: set[str] = set()
            signals_by_event_id: dict[str, dict[str, object]] = {}
            valid = True
            for signal in signal_data:
                if not isinstance(signal, dict) or not isinstance(signal.get("code"), str):
                    valid = False
                    break
                code = signal["code"]
                if code not in REQUIRED_SIGNAL_CODES:
                    valid = False
                    break
                signal_codes[code] = signal_codes.get(code, 0) + 1
                event_id = signal.get("event_id")
                if isinstance(event_id, str):
                    event_ids.add(event_id)
                    signals_by_event_id[event_id] = signal
            if not valid:
                failures.append("audit signal response has an unexpected signal contract")
            else:
                missing_codes = sorted(REQUIRED_SIGNAL_CODES - signal_codes.keys())
                missing_ids = sorted(config.expected_signal_event_ids - event_ids)
                forbidden_ids = sorted(config.forbidden_event_ids & event_ids)
                expected_sensitive_signals = [
                    signals_by_event_id[event_id]
                    for event_id in config.expected_signal_event_ids
                    if event_id in signals_by_event_id
                ]
                expected_severities = {
                    signal.get("severity")
                    for signal in expected_sensitive_signals
                    if signal.get("code") == "sensitive_write_detected"
                    and isinstance(signal.get("severity"), str)
                }
                expected_contract_valid = (
                    len(expected_sensitive_signals) == len(config.expected_signal_event_ids)
                    and all(
                        signal.get("code") == "sensitive_write_detected"
                        for signal in expected_sensitive_signals
                    )
                    and {"critical", "warning"}.issubset(expected_severities)
                )
                if missing_codes:
                    failures.append(f"required monitoring signal codes are missing: {missing_codes}")
                if missing_ids:
                    failures.append(
                        f"expected sensitive-write event IDs are missing ({len(missing_ids)} total)"
                    )
                if forbidden_ids:
                    failures.append(
                        f"cross-tenant event IDs appeared in monitoring ({len(forbidden_ids)} total)"
                    )
                if not expected_contract_valid:
                    failures.append(
                        "expected records do not prove both critical and warning sensitive-write signals"
                    )
                if (
                    not missing_codes
                    and not missing_ids
                    and not forbidden_ids
                    and expected_contract_valid
                ):
                    checks.append({"check": "monitoring_and_tenant_isolation", "status": "passed"})
                summary["signal_counts"] = dict(sorted(signal_codes.items()))
                summary["matched_expected_signal_event_ids"] = len(
                    config.expected_signal_event_ids & event_ids
                )

    if integrity is not None:
        violations = integrity.get("violations")
        healthy = integrity.get("healthy") is True
        zero_violations = isinstance(violations, dict) and violations and all(
            isinstance(value, int) and not isinstance(value, bool) and value == 0
            for value in violations.values()
        )
        paused = integrity.get("paused_schedules")
        failed = integrity.get("failed_attempts_24h")
        disposition_counts_match = (
            isinstance(paused, int)
            and not isinstance(paused, bool)
            and paused == config.allowed_paused_schedules
            and isinstance(failed, int)
            and not isinstance(failed, bool)
            and failed == config.allowed_failed_attempts_24h
        )
        if healthy and zero_violations and disposition_counts_match:
            checks.append({"check": "retention_integrity", "status": "passed"})
        else:
            failures.append(
                "retention integrity is unhealthy or does not match the disposition counts"
            )
        summary["retention_integrity"] = {
            "healthy": healthy,
            "violation_total": (
                sum(violations.values()) if zero_violations and isinstance(violations, dict) else None
            ),
            "paused_schedules": paused if isinstance(paused, int) else None,
            "failed_attempts_24h": failed if isinstance(failed, int) else None,
        }


def _parse_json_object(
    response: HttpResponse, label: str, failures: list[str]
) -> dict[str, object] | None:
    try:
        value = json.loads(response.body)
    except (UnicodeDecodeError, json.JSONDecodeError):
        failures.append(f"{label} response is not valid JSON")
        return None
    if not isinstance(value, dict):
        failures.append(f"{label} response is not a JSON object")
        return None
    return value


def _required_environment(name: str) -> str:
    value = os.environ.get(name)
    if value is None:
        raise DrillConfigurationError(f"required environment variable is missing: {name}")
    return value


def _marker_environment(name: str) -> tuple[str, ...]:
    raw = _required_environment(name)
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as error:
        raise DrillConfigurationError(f"{name} must contain a JSON string array") from error
    if not isinstance(value, list) or not all(isinstance(marker, str) for marker in value):
        raise DrillConfigurationError(f"{name} must contain a JSON string array")
    return tuple(value)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", required=True, help="Hosted origin without an API path")
    parser.add_argument("--api-prefix", default="/api", help="/api for Rust or /api/backend/api for Next.js")
    parser.add_argument("--environment-reference", required=True)
    parser.add_argument("--authentication-header", default="Authorization")
    parser.add_argument("--admin-auth-env", default="FT401_ADMIN_AUTH")
    parser.add_argument("--viewer-auth-env", default="FT401_VIEWER_AUTH")
    parser.add_argument("--operator-auth-env", default="FT401_OPERATOR_AUTH")
    parser.add_argument("--forbidden-markers-env", default="FT401_FORBIDDEN_MARKERS_JSON")
    parser.add_argument("--expected-signal-event-id", action="append", required=True)
    parser.add_argument("--forbidden-event-id", action="append", required=True)
    parser.add_argument("--window-hours", type=int, default=24)
    parser.add_argument("--allowed-paused-schedules", type=int, default=0)
    parser.add_argument("--allowed-failed-attempts-24h", type=int, default=0)
    parser.add_argument("--timeout-seconds", type=float, default=DEFAULT_TIMEOUT_SECONDS)
    parser.add_argument("--allow-loopback-http", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    try:
        args = parse_args(argv)
        config = DrillConfig(
            environment_reference=args.environment_reference,
            admin_authorization=_required_environment(args.admin_auth_env),
            viewer_authorization=_required_environment(args.viewer_auth_env),
            operator_authorization=_required_environment(args.operator_auth_env),
            expected_signal_event_ids=frozenset(args.expected_signal_event_id),
            forbidden_event_ids=frozenset(args.forbidden_event_id),
            forbidden_markers=_marker_environment(args.forbidden_markers_env),
            window_hours=args.window_hours,
            allowed_paused_schedules=args.allowed_paused_schedules,
            allowed_failed_attempts_24h=args.allowed_failed_attempts_24h,
        )
        client = BoundedHttpClient(
            args.base_url,
            args.api_prefix,
            args.authentication_header,
            args.timeout_seconds,
            args.allow_loopback_http,
        )
        evidence = verify_hosted_drill(config, client)
    except DrillConfigurationError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2
    except RuntimeError:
        print("ERROR: hosted drill verification could not complete", file=sys.stderr)
        return 1
    print(json.dumps(evidence, indent=2, sort_keys=True))
    return 0 if evidence["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
