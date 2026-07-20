# Risk Register

Scores use likelihood and impact from 1 (low) to 5 (high). Owners are roles until named people are assigned.

| ID | Risk | L | I | Trigger | Mitigation | Owner | Review gate |
|---|---|---:|---:|---|---|---|---|
| R-01 | Flight-data license does not permit intended commercial use | 3 | 5 | Provider terms are ambiguous or restrictive | Complete written feasibility matrix and obtain commercial terms before integration | Product | M3 |
| R-02 | Stale or partial data appears current | 4 | 5 | Feed delay exceeds threshold without visible warning | Track event/receive/process times; show stale and degraded states; test outages | Backend | M2 |
| R-03 | Alert fatigue makes the product unusable | 4 | 4 | High dismissal rate or duplicate alerts | Dedupe keys, severity policy, suppression windows, dismissal reasons, replay tuning | Product | M2/M4 |
| R-04 | Users mistake advisory output for certified guidance | 3 | 5 | Copy or UI implies authority | Explicit advisory labeling, human review, legal review, role-based controls | Product/Trust | M4 |
| R-05 | Route–hazard geometry is wrong around time or altitude | 3 | 5 | False negative/positive in validation cases | Version geometry and rules; include altitude/time; golden fixtures; independent review | Backend | M2 |
| R-06 | Rust slows iteration due to unfamiliarity | 3 | 3 | Repeated implementation delays | Modular monolith, documented patterns, narrow dependencies, strong fixtures | Engineering | M1 |
| R-07 | External feed outages break demos and tests | 4 | 3 | Provider unavailable | Deterministic replay, cached fixtures, circuit breakers, degraded UI | Backend | M1/M2 |
| R-08 | Tenant data leaks across operators | 2 | 5 | Query lacks tenant scope | Tenant key in schema, scoped repository APIs, authorization tests | Security | M3 |
| R-09 | LLM summary omits or fabricates operational detail | 3 | 5 | Draft differs materially from evidence | Constrained structured inputs, citations, human approval, evaluation set | AI/Product | M5 |
| R-10 | Optimization cannot be validated with available truth data | 4 | 4 | No historical baseline or outcome data | Make data availability a research gate before product commitment | Product/Data | M5 |
