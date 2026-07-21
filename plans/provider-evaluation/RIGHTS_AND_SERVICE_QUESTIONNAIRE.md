# Rights and Service Questionnaire

Send the same questions to Cirium and FlightAware. A response is decision evidence only when it cites the controlling Order, data license, service schedule, or negotiated amendment. Public documentation and sales email may inform negotiation but do not close a contractual gate. Record each clause-level answer and Legal/Engineering disposition in `provider-question-responses.csv`; do not paste confidential contract text into the repository.

## Intended use statement

The customer will operate a multi-tenant, business-to-business advisory operations console for airline or charter personnel. The service displays active aircraft, schedules, route and position facts, correlates those facts with independently sourced weather and hazards, prioritizes human review, retains an audit trail, and may replay historical events for incident review and product testing. It is not an air-navigation, collision-avoidance, air-traffic-control, aircraft-separation, or autonomous dispatch system.

## Required rights

For every answer, provide `yes`, `no`, or `exception required`, the controlling clause, any field or source limitations, and any additional fee.

| ID | Question | Pass condition |
| --- | --- | --- |
| R-01 | May the data be displayed in a commercial aircraft situational-awareness and airline/charter operations console? | Explicit use is permitted for the intended advisory product. |
| R-02 | May authenticated personnel of multiple customer operators view only their own entitled fleet data? | Tenant/customer display and authorized-user scope are explicit. |
| R-03 | May the service provider host, process, and transmit the data on behalf of each operator? | Cloud/SaaS processing and subprocessors are permitted. |
| R-04 | May raw provider records be combined with NOAA, FAA, operator, and other licensed data? | Source combination is permitted without contaminating ownership of independent data. |
| R-05 | May normalized facts, alert evidence, aggregates, and derived metrics be created and displayed? | Derived works are defined, usable, and owned or perpetually licensed as needed. |
| R-06 | May source attribution and provider identity be shown in evidence and audit history? | Required attribution is known and implementable. |
| R-07 | May raw data be retained for ingestion replay, debugging, billing disputes, and incident investigation? | Exact fields and maximum periods are stated. |
| R-08 | May normalized facts and append-only action history outlive the raw-data retention period? | Exact derivative and audit retention periods are stated. |
| R-09 | May redacted, synthetic, or contract-approved fixtures be retained in automated tests? | Fixture creation and post-termination use are explicit. |
| R-10 | What deletion is required at termination, user revocation, aircraft entitlement loss, or provider request? | Objects, derivatives, backups, logs, deadlines, and attestations are defined. |
| R-11 | How must LADD, PIA, blocked, sensitive, government, military, and owner-authorized aircraft be handled? | Entitlement source, update cadence, display restrictions, and deletion behavior are defined. |
| R-12 | May operator-authorized blocked aircraft be shown, and what proof is required? | Authorization workflow and audit evidence are implementable. |
| R-13 | Are screenshots, support exports, customer reports, and incident packages allowed? | External/export scope and redaction rules are explicit. |
| R-14 | May aggregate, non-identifying service metrics be used for reliability and product improvement? | Aggregation threshold and prohibited uses are explicit. |
| R-15 | Do provider terms claim ownership or a license over customer routes, annotations, actions, or operational data? | Customer data rights and provider processing scope are acceptable. |
| R-16 | Which countries, users, aircraft, and purposes are excluded by export controls or upstream licenses? | Exclusions can be enforced before display. |
| R-17 | What attribution, branding, copyright, and third-party notices are mandatory? | UI and documentation obligations are complete. |
| R-18 | Do negotiated Order terms expressly control over conflicting standard terms? | Precedence is unambiguous and exceptions cite the displaced clause. |
| R-19 | May provider data, normalized facts, derived evidence, screenshots, or outputs be disclosed, transferred, processed, uploaded to, or used with an LLM, machine-learning model, or other contract-defined AI system? | Prior written authorization identifies permitted fields, purposes, model providers, subprocessors, locations, retention, logging, human review, and training restrictions. |
| R-20 | Do upstream licenses, including Aireon or equivalent third-party data terms, permit the proposed multi-tenant SaaS processing and display to each operator's authorized users? | Each upstream layer is mapped to permitted tenants, users, purposes, regions, and technical controls in the controlling Order. |
| R-21 | Does the Order expressly override any default 24-hour or other short retention limit for the raw records, normalized facts, audit history, incident evidence, and approved fixtures the product must retain? | Field-level periods, termination deletion, backup handling, and any surviving aggregate or audit rights are explicit. |

## Service and operational terms

| ID | Question | Required evidence |
| --- | --- | --- |
| S-01 | What production uptime percentage applies, and what components and regions are measured? | SLA schedule and calculation formula. |
| S-02 | What exclusions, maintenance windows, and force-majeure rules apply? | SLA exclusions and maintenance policy. |
| S-03 | What service credits or termination rights apply after repeated misses? | Credit table and chronic-failure remedy. |
| S-04 | How quickly are planned maintenance and material incidents communicated? | Notification channels and minimum notice. |
| S-05 | What are P1/P2 support response and restoration targets, including nights and weekends? | Support schedule and escalation path. |
| S-06 | What data-correction, dispute, and root-cause workflows exist? | Case process, evidence requirements, and response target. |
| S-07 | What security evidence, breach notice, subprocessor notice, and audit rights are available? | Security schedule, notice deadline, and current reports. |
| S-08 | What schema/version notice and backward-compatibility period applies? | Version policy and deprecation notice. |
| S-09 | What reconnect, replay, and point-in-time recovery guarantees apply? | Recovery window, ordering, duplication, and completeness semantics. |
| S-10 | Which position sources and quality flags are included in the proposed package? | Layer list and field-level source/quality semantics. |
| S-11 | What update interval is contracted by phase and region? | Minimum/target cadence and exclusions. |
| S-12 | What happens during provider outage, contract suspension, expiration, and termination? | Export, wind-down, deletion, and continuity terms. |

## Pricing request

Require one proposal for each of 20, 100, and 500 simultaneously monitored flights. Each proposal must identify:

- fixed platform, account, environment, connection, and support fees;
- included data layers, regions, operators, surface/oceanic/polar coverage, and update cadence;
- usage units, minimums, overages, burst rules, replay and historical charges;
- development, preview, disaster-recovery, and active-active connection charges;
- implementation, onboarding, professional-services, and support fees;
- annual escalator, term, renewal, early termination, and price-protection terms;
- taxes and currency;
- cost behavior during peak traffic, reconnect replay, duplicate delivery, and provider failure.

## Approval record

| Role | Name | Decision | Date | Evidence references | Open exceptions |
| --- | --- | --- | --- | --- | --- |
| Legal/privacy | Pending | Pending | Pending | Pending | Pending |
| Engineering/data | Pending | Pending | Pending | Pending | Pending |
| Product/commercial | Pending | Pending | Pending | Pending | Pending |
