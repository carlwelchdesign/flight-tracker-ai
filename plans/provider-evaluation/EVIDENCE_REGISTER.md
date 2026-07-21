# FT-301 Evidence Register

This register contains redacted metadata, not confidential documents. Store controlled originals in the approved contract repository and use an opaque reference that does not expose a secret URL or customer identifier.

## Evidence status

Use only `missing`, `requested`, `received`, `accepted`, `exception`, or `rejected`.

| Evidence ID | Provider | Category | Status | Document or test window | Controlled reference | Received | Owner | Reviewer | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| FA-RIGHTS | FlightAware Firehose | Rights and license | missing | Order plus July 2026 data-service terms | Pending | Pending | Product | Legal | Order must expressly close AI processing, longer retention, multi-tenant SaaS, upstream-source, display, and derivative rights. |
| FA-SLA | FlightAware Firehose | SLA and support | missing | SLA and support schedule | Pending | Pending | Product | Engineering | Public page advertises high-SLA options but no binding percentage. |
| FA-PRICE | FlightAware Firehose | Price | missing | Proposal for 20/100/500 flights | Pending | Pending | Product | Engineering | Fixed monthly fee is package-specific. |
| FA-TRIAL | FlightAware Firehose | Real-time trial | requested | Matched 14-day target-tail window | Pending | Pending | Engineering | Self-service trial is historical-only; sales must provision real time. |
| CI-RIGHTS | Cirium Sky Stream | Rights and license | missing | Contract, license, and data schedules | Pending | Pending | Product | Legal | Marketing use cases are not contractual permission. |
| CI-SLA | Cirium Sky Stream | SLA and support | missing | SLA and support schedule | Pending | Pending | Product | Engineering | Public material does not state a binding uptime remedy. |
| CI-PRICE | Cirium Sky Stream | Price | missing | Proposal for 20/100/500 flights | Pending | Pending | Product | Engineering | Contract price is not public. |
| CI-TRIAL | Cirium Sky Stream | Real-time trial | requested | Matched 14-day target-tail window | Pending | Pending | Engineering | Public documentation offers a 14-day Sky Stream trial. |
| TARGET-POP | Both | Trial population | missing | Redacted target-tail and expected-flight manifest | Pending | Pending | Operator partner | Product | Must be operator-owned or explicitly authorized. |
| TRIAL-RESULT | Both | Technical scorecard | missing | Common trial window | Pending | Pending | Engineering | Product | Do not compare different windows or populations. |

## Exception handling

Every `exception` row must name an owner, a resolution deadline, the affected pass/fail gate, and one of: negotiated amendment, design control, scope removal, or no-select. Product cannot accept a legal exception for Legal, and Engineering cannot accept an unmeasured coverage exception for Product.
