# FT-301 Provider Evaluation Package

This folder turns the commercial-provider gate into a repeatable procurement and trial workflow. It does not select a provider and it does not treat public marketing as contractual permission.

## Current state

- Finalists: Cirium Sky Stream and FlightAware Firehose.
- Decision: pending.
- Public claims last revalidated: 2026-07-21.
- Blocking inputs: an operator-owned target-tail set, two comparable real-time trial accounts, Order-level rights responses including explicit AI/ML and upstream-data authorization, SLA schedules, retention/deletion terms, and priced proposals.
- Decision authority: Product and Legal approve rights and commercial terms; Engineering approves technical fit; neither group can waive the other's pass/fail gate.

## Required workflow

1. Send each provider the same [outreach request](OUTREACH_REQUESTS.md) and [rights and service questionnaire](RIGHTS_AND_SERVICE_QUESTIONNAIRE.md), and require clause-level written answers.
2. Record only non-confidential evidence in [the evidence register](EVIDENCE_REGISTER.md). Store contracts, credentials, target tails, and provider-confidential material outside Git.
3. Confirm both providers can supply the same 14-calendar-day real-time trial window and target population before starting either clock.
4. Freeze the test population and regions using [the trial protocol](TRIAL_PROTOCOL.md).
5. Collect raw trial observations without converting missing values into zero or silently dropping reconnect periods.
6. Populate the evidence register, `trial-scorecard.csv`, and `cost-model.csv`; validate the complete package with `python3 scripts/validate_ft301_evidence.py` from the repository root. The validator protects the required document set, R-01–R-21 and S-01–S-12 question IDs, evidence IDs/statuses, trial metrics, and cost scenarios.
7. Apply the pass/fail gates and scored comparison in [the decision scorecard](DECISION_SCORECARD.md).
8. Legal records its approval, Engineering records technical approval, Product records the commercial recommendation, and the accountable owner resolves OD-002.

## RACI

| Activity | Product | Legal/privacy | Engineering/data | Operator partner | Provider |
| --- | --- | --- | --- | --- | --- |
| Define target population and regions | A | C | R | R | I |
| Answer rights and service questions | I | A | C | I | R |
| Approve contract language | C | A/R | C | C | I |
| Provision comparable real-time trials | A | I | C | C | R |
| Collect and normalize observations | I | I | A/R | C | C |
| Validate coverage and latency | C | I | A/R | C | I |
| Validate cost model | A/R | C | C | I | C |
| Select provider and resolve OD-002 | A/R | required approval | required approval | C | I |

`A` is accountable, `R` is responsible, `C` is consulted, and `I` is informed.

## Data handling

Do not commit credentials, contracts, quotes marked confidential, real tail lists, passenger or crew data, or raw licensed provider messages. Committed evidence must be an aggregate, redacted summary with a stable reference to the controlled original. Trial collection must use the minimum fields defined in the protocol and follow the stricter provider term until Legal approves a final retention schedule. No provider data, normalized facts, evidence, screenshots, or outputs may enter an AI/ML system until the controlling Order and Legal explicitly authorize that processing.

## Completion rule

FT-301 remains incomplete until every ticket acceptance item has primary evidence, `python3 scripts/validate_ft301_evidence.py --require-complete` passes, and OD-002 names one provider or records a no-select decision. A technically superior feed cannot win without acceptable rights, and permissive rights cannot compensate for failed target-fleet coverage.

## Planning-review traceability

| Review lens | Material gap found | Integrated control |
| --- | --- | --- |
| Product strategy | Marketing coverage could become a de facto decision without a common target population. | Paired trial, frozen population, mandatory-region gates, and `no_select` outcome. |
| Trust, privacy, and rights | Public use cases do not grant tenant display, combination, AI/ML processing, upstream-data SaaS use, retention, blocked-tail, or derivative rights. | Clause-level questionnaire, deny-by-default AI boundary, controlling-Order precedence, Legal gate, and controlled evidence references. |
| Data and analytics | Coverage and latency lacked a reproducible grain, denominator, null rule, and method version. | `ft301-v1` metric contract, raw counts, region rows, explicit unknowns, CSV schemas, and validation. |
| Delivery/TPM | External inputs had no shared workflow, authority split, or auditable status. | RACI, evidence statuses, exact blockers, outreach drafts, structural validation, and a draft PR that remains open until the gate closes. |

Before outreach, Product must replace every `Pending` owner name and target date in the controlled procurement tracker. Dates and contact details are intentionally not invented in this public repository.
