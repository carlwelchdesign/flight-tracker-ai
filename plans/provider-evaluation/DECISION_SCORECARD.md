# Provider Decision Scorecard

## Decision order

The decision uses gates before weights. Do not calculate a winning score for a provider that fails a gate.

### Pass/fail gates

| Gate | Pass condition | Authority |
| --- | --- | --- |
| Exact-use rights | All R-01 through R-21 questions are accepted in controlling contract language, or every exception has an approved design control and amendment. | Legal/privacy |
| Aircraft privacy | LADD, PIA, blocked-tail, owner authorization, entitlement updates, and deletion are enforceable for the proposed package. | Legal/privacy and Engineering |
| Retention and exit | Raw, normalized, derived, audit, backup, fixture, termination, and deletion rules are implementable and priced. | Legal/privacy and Engineering |
| Comparable trial | Both providers complete a valid paired 14-day real-time target-population trial under `ft301-v1`. | Engineering/data |
| Target-fleet fitness | Pre-frozen minimums for coverage, freshness, identity, and recovery pass in every mandatory region. | Product and Engineering |
| Service terms | Uptime definition, credits/remedies, incident notice, support targets, version policy, and recovery semantics are accepted. | Product and Engineering |
| Complete price | The 20, 100, and 500-flight scenarios include every fixed and variable component under normal, peak, replay, reconnect, and failure behavior. | Product/commercial |

If neither provider passes, record `no_select`, preserve simulation, and negotiate or evaluate another licensed enterprise provider. Do not relax a gate after seeing the scores without a dated decision-log entry and all affected approvers.

### Weighted comparison after gates

Freeze metric thresholds and scoring bands before the scored trial begins.

| Dimension | Weight | Evidence |
| --- | ---: | --- |
| Target-flight and regional coverage | 30 | Expected-flight identification, position availability, longest gaps by mandatory region. |
| Freshness and data quality | 20 | Age thresholds, p50/p95/p99 lag, source quality, missing/invalid fields. |
| Recovery and operational reliability | 15 | Disconnect duration, replay completeness, duplicates, ordering, maintenance behavior. |
| Flight identity and operational events | 10 | Schedule, tail continuity, diversion, cancellation, codeshare, and swap behavior. |
| Rights and retention simplicity | 10 | Approved exceptions, field restrictions, deletion burden, attribution, auditability. |
| Three-scale total cost | 10 | Complete 20/100/500 totals and sensitivity to peak/replay/failure. |
| Implementation and support fit | 5 | Adapter complexity, protocol/tooling, sandbox parity, support and correction workflow. |

Each dimension receives 0–5 points from a pre-frozen rubric. Weighted total is `sum(points / 5 × weight)`. Publish the component scores, raw evidence references, uncertainty, and dissenting reviewer notes; never publish only the total.

## Sensitivity and recommendation

- Recalculate with coverage and freshness weights each varied by ±10 percentage points.
- Show cost at 20, 100, and 500 flights rather than collapsing to one forecast.
- Identify any ranking change caused by an unobserved region or one provider exception.
- A difference below five weighted points is a practical tie and requires a documented product judgment.
- The recommendation must include a fallback provider, termination/export plan, implementation estimate, and the conditions that would trigger reconsideration.

## Final record

| Field | Value |
| --- | --- |
| Decision | Pending |
| Selected provider | Pending |
| Effective package and data layers | Pending |
| Primary evidence window | Pending |
| Legal approval | Pending |
| Engineering approval | Pending |
| Product approval | Pending |
| Fallback provider | Pending |
| Reconsideration triggers | Pending |
| OD-002 update | Pending |
