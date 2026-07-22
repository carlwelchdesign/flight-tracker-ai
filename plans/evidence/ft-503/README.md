# FT-503 human-reviewed drafting evidence

This package records the bounded offline research result for FT-503. It does
not enable live delivery, aircraft control, dispatch, flight planning, or an
authoritative route recommendation.

## What was verified

- Only an explicitly human-approved FT-502 recommendation can be minimized for
  drafting. The model adapter receives that minimized structure rather than a
  provider payload or operational record.
- Facts, source citations, timestamps, generated wording, generation metadata,
  and review state remain distinct serialized fields.
- A draft begins in `awaiting_review`. Approval, edited approval, and rejection
  require an identified reviewer and a timestamp that does not predate the
  generated draft. A reviewed draft cannot be reviewed again.
- There is no HTTP, persistence, messaging, or send adapter in the drafting
  module. The smoke result explicitly reports `automatic_send_available` as
  `false`.
- Seven versioned cases exercise grounded output, omission, fabricated numeric
  detail, unit change, unsafe authority phrasing, unknown source references,
  and output bounds. All seven matched their sealed expected findings.
- Generator errors and validation failures fail closed to a deterministic,
  valid template that still requires review.

## Live provider probe

A local probe used the securely stored, ignored OpenAI credential with the
Responses API and `store: false`. The provider returned a rate-limit response.
The application classified it without retaining or exposing the response body,
produced the deterministic fallback, preserved `awaiting_review`, and required
an explicit review transition. This demonstrates the unavailable-provider path;
it is not evidence of a successful model-generated draft.

The credential itself is not part of this evidence and must never be committed.
Machine-readable results are in [`EVALUATION_REPORT.json`](EVALUATION_REPORT.json).
