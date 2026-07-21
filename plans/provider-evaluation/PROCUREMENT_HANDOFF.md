# FT-301 Procurement Handoff

This is the human-action boundary for selecting a commercial flight-data provider. The engineering package is ready; the remaining inputs must come from accountable people and the providers. Do not invent names, approvals, contract answers, prices, tails, or trial results to advance the checklists.

## Outcome required

Obtain comparable, controlled evidence for Cirium Sky Stream and FlightAware Firehose, then run the paired trial and record a selection or `no_select`. The same intended use, questions, population, window, metrics, and cost scenarios apply to both providers.

## Official entry points

| Provider | Initial contact | Trial reference | Request |
| --- | --- | --- | --- |
| Cirium Sky Stream | [Cirium Sky contact form](https://www.cirium.com/contact-us/cirium-sky/) | [Sky Stream onboarding](https://developer.cirium.com/apis/cirium-sky-stream/get-started) documents a 14-day trial for new users. | Ask for a commercial contact, controlling terms, priced 20/100/500-flight proposals, and the matched real-time trial. |
| FlightAware Firehose | [FlightAware contact page](https://www.flightaware.com/about/contact/) | [Firehose trial](https://www.flightaware.com/commercial/firehose/trial) is a sign-in path; the public FAQ says the self-service trial is historical, so the request must explicitly ask Sales for comparable real-time provisioning. | Name Firehose—not AeroAPI—and request the controlling Order, real-time trial, SLA schedule, layers, and priced proposals. |

Use the provider-specific messages in [OUTREACH_REQUESTS.md](OUTREACH_REQUESTS.md). Do not submit on behalf of a fabricated company, operator, budget owner, or legal reviewer.

## Internal inputs required before submission

- [ ] Product owner name and business email are recorded in the controlled procurement tracker.
- [ ] Legal reviewer name and response target are recorded.
- [ ] Engineering/data reviewer name and response target are recorded.
- [ ] Operator partner and proof that it may authorize the target population are recorded.
- [ ] Approved monthly budget range and procurement authority are recorded.
- [ ] Desired trial-start range, mandatory regions, operating phases, and approximate simultaneous-flight count are recorded.
- [ ] Legal approves the intended-use statement and the R-01–R-21/S-01–S-12 questionnaire for submission.

Names, emails, budgets, real tail identifiers, contracts, credentials, and confidential replies belong in the approved controlled system—not this repository.

## Submission and intake sequence

1. Submit both initial requests in the same business day using the approved owner identity and the messages in `OUTREACH_REQUESTS.md`.
2. Do not include real tails, credentials, passenger/crew data, or confidential operator details in a public web form.
3. Save each submission confirmation outside Git. Add only its opaque controlled reference and date to `EVIDENCE_REGISTER.md`; only then change the relevant status from `missing` to `requested`.
4. Establish confidentiality and trial-retention terms before sharing the authorized population.
5. Send the identical questionnaire and cost scenarios. Require clause-level answers tied to the controlling Order, license, SLA, or amendment.
6. Enter redacted answer summaries in `provider-question-responses.csv`; Legal or Engineering—not Sales—sets each review disposition.
7. Confirm both real-time trials can use the same authorized population and 14-calendar-day window before either clock starts.
8. Run [TRIAL_PROTOCOL.md](TRIAL_PROTOCOL.md), populate the trial and cost CSVs, complete the score matrix, record approvals, and update OD-002.

## Evidence routing

| Returned item | Repository record | Controlled original |
| --- | --- | --- |
| Submission confirmation | `EVIDENCE_REGISTER.md` status/reference/date | Procurement correspondence store |
| Contract, Order, license, amendment | Rights evidence row plus clause-level response rows | Contract repository |
| SLA/support/security schedule | SLA evidence row plus S-question response rows | Contract/security repository |
| Proposal and pricing assumptions | Price evidence row and `cost-model.csv` | Procurement repository |
| Authorized target population | `TARGET-POP` metadata only | Operator-approved secure location |
| Trial messages and credentials | No raw content in Git | Approved trial environment |
| Aggregate trial results | `trial-scorecard.csv` and `TRIAL-RESULT` | Controlled raw observation store |
| Final approvals and recommendation | `provider-decision.csv` and OD-002 | Approval/decision system |

## Resume-development gate

FT-302 may begin only after:

- [ ] `python3 scripts/validate_ft301_evidence.py --require-complete` passes;
- [ ] the selected provider has no rejected or pending required-use response;
- [ ] Legal, Engineering, and Product approval references are present;
- [ ] `plans/DECISIONS.md` removes OD-002 from open decisions and records its explicit resolution; and
- [ ] FT-301 acceptance checkboxes are backed by the controlled evidence above.

If neither provider passes, record `no_select`, retain deterministic simulation, and do not start a live-provider adapter.
