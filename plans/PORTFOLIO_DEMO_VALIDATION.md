# Portfolio Demo Validation

## Decision and evidence boundary

Current decision: **Revise before public review**.

The repository is ready for a self-guided neutral-review session after the
FT-403 orientation changes are deployed. It is not yet approved for public
publication. The review recorded below is a sequential expert simulation by
one development session, supported by automated component tests; it is not an
independent human usability study and does not satisfy the participant gate.
FT-404 may create the private preview needed to run that study, but production
promotion remains blocked until the observations table contains at least one
recruiter, hiring manager, or neutral reviewer who is not the implementer.

## Scope

The evaluation covers the recruiter-facing operations console: portfolio
orientation, replay flight picture, flight selection, source and freshness
evidence, ranked alert explanation, human-controlled alert action, and recovery
from an optional position-source failure. It does not evaluate airline
operations, dispatch correctness, certification, commercial provider service
levels, or real-world flight safety.

### Representative viewers

| Viewer | What they should learn | Relevant background |
| --- | --- | --- |
| Recruiter | What was built, its engineering depth, and its limits | Can evaluate a portfolio without aviation expertise |
| Hiring manager | How evidence becomes an explainable, human-reviewed alert | Familiar with software delivery; aviation expertise optional |
| Neutral reviewer | Whether the flow, language, focus order, and states are understandable | Has not implemented or previously rehearsed this console |

### Data modes

- **Deterministic replay:** complete and repeatable portfolio demonstration.
- **Live NOAA weather:** source-attributed context with visible freshness and
  degraded-state handling.
- **Optional ADSB.lol positions:** best-effort, position-only context; disabled
  by default and never required to complete the walkthrough.
- **Degraded/unavailable source:** an intentional product state that leaves
  replay directly available.

## Unfacilitated task protocol

Give the reviewer only the preview URL and the sentence: “Please use the page
as if a candidate sent it with a job application. Think aloud, but I will not
explain the interface until you finish.” Start the timer when the page is
visible. Do not point at controls, define aviation terms, or rescue a failed
task. A reviewer may use the on-page walkthrough and labels.

| ID | Task | Success evidence | Target |
| --- | --- | --- | ---: |
| T1 | Explain what the product does, which data is repeatable versus live, and who takes action | Mentions flight attention, replay, live supporting context, and a human decision | 90 seconds; all four concepts |
| T2 | Find a flight needing attention and state why | Selects a watch flight and cites visible route, timing, or weather evidence | 90 seconds |
| T3 | Identify source and freshness limits | Locates source/freshness evidence without treating optional positions as complete truth | 120 seconds |
| T4 | Review the highest-priority alert and take one reversible human action | Opens alert evidence and completes acknowledgement or assignment deliberately | 120 seconds |
| T5 | Continue when live positions are degraded or unavailable | Recognizes degradation and selects replay without facilitator help | 60 seconds |

### Measures

- **Time to understand:** elapsed time and concepts correctly stated for T1.
- **Task completion:** pass, partial, or fail for T2–T5; note wrong turns and
  facilitator interventions (any intervention means the task failed).
- **Source-mode comprehension:** score one point each for replay, NOAA weather,
  optional position-only context, degraded availability, and human action.
  Publication requires at least 4/5 without prompting.
- **Data availability:** record the displayed replay, weather, position, stream,
  and service state at the start and end of the session.
- **Qualitative observations:** quote or paraphrase confusion about copy,
  evidence, controls, source labels, focus order, and trust boundaries without
  storing personal data.

## Sequential expert review

These role-based passes were performed sequentially by the implementing Codex
session, not by independent agents or representative participants.

| Perspective | Observation | Severity | Treatment |
| --- | --- | --- | --- |
| Staff product design | The console opened as a dense operator surface with no page-level explanation or primary route through it | Must fix | Added a compact three-minute orientation with one H1, three tasks, and direct flight-board/alert links |
| Recruiter | The portfolio purpose, repeatable/live distinction, and decision boundary had to be inferred from several distant labels | Must fix | Put promise, modes, and human-action boundary together at the top of the page |
| Hiring manager | The existing “Outage” control read as status or unexplained jargon | Must fix | Renamed it “Test outage” and the recovery state “Restore feed”; retained explicit accessible names |
| Accessibility | The page lacked an H1 and the alert destination had no stable fragment target | Must fix | Added a labeled semantic section, H1, nav, ordered steps, definition list, and `#alert-review` target |
| Demo reliability | Optional positions could distract from the deterministic path | Important | Orientation and runbook name replay as the reliable demo and optional live data as supporting context |

Automated tests confirm that the orientation exposes its heading, source modes,
human-action boundary, and working fragment targets. Existing interaction tests
cover replay availability during failure, visible degraded states, alert review,
and explicit human actions. These tests support the session; they do not measure
human comprehension or replace the participant record.

## Participant observations

Complete one row per reviewer. Use role labels rather than names.

| Date | Viewer | T1 time / concepts | T2 | T3 | T4 | T5 | Source score | Availability observed | Interventions | Key observations |
| --- | --- | --- | --- | --- | --- | --- | ---: | --- | ---: | --- |
| Pending | Neutral reviewer | — | — | — | — | — | — | — | — | Private preview required |

## Publish gate

Change the decision to **Publish** only when:

- at least one independent representative or neutral reviewer completes the
  protocol on the same candidate build;
- T1 meets its time/concept threshold, T2–T5 pass without intervention, and
  source comprehension is at least 4/5;
- no unresolved must-fix issue misstates source authority, freshness,
  availability, operational suitability, or the human decision boundary; and
- FT-404 hosted security, recovery, and browser checks pass.

Choose **Revise** when a fix can reasonably meet the gate, or **Stop** when a
trust/safety defect cannot be corrected before the portfolio deadline. Record
the decision, candidate build, observations, owner, and required follow-up in
this file before closing FT-403.
