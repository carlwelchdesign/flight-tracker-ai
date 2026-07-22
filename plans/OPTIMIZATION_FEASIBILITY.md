# Offline Route-Candidate Recommendation Feasibility

Status: Approved for the bounded FT-502 experiment only. This is not approval
for live recommendations, flight planning, dispatch, clearance, or automatic
messaging.

## Narrow problem

For one versioned replay scenario in which a planned route overlaps a
convective hazard, rank a maximum of 12 pre-authored alternative route
candidates and recommend at most one candidate for **human review**.

The experiment does not generate routes. It chooses among fixture-owned
candidates that share the scenario's origin, destination, evaluation time, and
aircraft altitude context. A recommendation means only “review this candidate
first in the offline demonstration.”

## Explicit non-goals

- No live ADS-B, NOAA, filed-route, tail-performance, passenger, crew, airport
  capacity, NOTAM, fuel-price, or dispatch-release data enters the experiment.
- No route filing, clearance, flight-plan amendment, operational message,
  aircraft control, or automatic action exists.
- Added great-circle distance is a transparent geometric proxy. It is not fuel,
  time, emissions, cost, feasibility, or safety performance.
- An LLM neither selects candidates nor changes eligibility, constraints,
  scores, units, or abstention.

## Versioned inputs and rights

| Input | FT-502 source | Availability and rights | Required evidence |
| --- | --- | --- | --- |
| Current and alternative route geometry | Project-authored WGS84 replay fixtures | Available and repository-owned | Fixture ID, route ID/version, ordered coordinates, origin/destination |
| Hazard geometry, validity, severity, and altitude band | Project-authored synthetic convective fixtures | Available and repository-owned; not copied from live NOAA records | Hazard ID/revision, issue/valid times, polygon, altitude units/reference |
| Aircraft altitude context and scenario clock | Project-authored replay observations | Available and repository-owned | Observation time, altitude value/unit/reference, replay offset |
| Distance and complexity proxies | Deterministic calculation from the above | Available; no external data rights | Candidate distance, added distance, segment count, algorithm version |
| Expected eligibility and reviewer disposition | Versioned labels authored for the experiment | Must be created before scoring the held-out set | Constraint label, acceptable/unacceptable/abstain label, reviewer reason |

This scope deliberately excludes ADSB.lol data because ADR-011 prohibits
sending or persisting it for analysis, and excludes live provider data because
the portfolio has no operational truth or outcome license. NOAA remains visible
weather context in the tracker but is not an optimization input. If a future
experiment uses provider data, R-13 requires written field-, purpose-, vendor-,
retention-, training-, and deletion-level authorization first.

## Hard constraints

A candidate is eligible only when deterministic Rust code can prove all of the
following:

1. The input set is versioned, complete, WGS84-valid, and contains 2–32 route
   points and no more than 12 candidates.
2. Candidate origin and destination match the scenario and its endpoints remain
   within the fixture tolerance.
3. Route, hazard, aircraft altitude, and evaluation time use compatible units,
   altitude references, and validity windows.
4. The remaining candidate path neither intersects nor comes within the fixed
   25 NM hazard margin while altitude bands overlap.
5. Added great-circle distance is finite and no more than 25 percent above the
   fixture's direct reference route.
6. The candidate contains no provider-derived, live, personal, tenant,
   dispatcher, or free-form instruction data.

Unknown geometry, time, altitude, unit, or reference data is not “probably
safe”; it makes that candidate indeterminate and ineligible.

## Baseline and scoring

The documented baseline is the shortest pre-authored candidate by great-circle
distance without hazard filtering. It is intentionally hazard-blind and makes
the value of the hard constraint measurable.

After constraints remove unsafe or indeterminate candidates, the experiment
ranks the remainder lexicographically by:

1. lowest added nautical miles;
2. fewest path segments;
3. stable candidate ID.

There are no learned weights. Every result records the winning candidate and
all rejected candidates with constraint outcomes and calculation versions.

## Development and held-out protocol

FT-502 must create at least 30 versioned cases before evaluating results:

- 18 development cases, visible while implementing;
- 12 held-out cases whose expected disposition is sealed before the first
  aggregate evaluation;
- at least four impossible cases, four missing/indeterminate evidence cases,
  four baseline-safe cases, and six multi-candidate hazard cases across the
  combined set.

The held-out labels remain unchanged after the first run. Corrections require a
new dataset version and a report that preserves the prior result.

## Success, failure, and abstention

The experiment passes only if all are true:

- 100 percent of recommendations satisfy every hard constraint;
- 100 percent of impossible or indeterminate held-out cases abstain;
- hazard-clear selection improves by at least 30 percentage points over the
  hazard-blind shortest-route baseline;
- median added distance among recommendations is no more than 20 percent of the
  direct fixture route;
- at least 90 percent of held-out recommendations are marked acceptable by the
  independent domain reviewer, with no unsafe recommendation;
- repeated runs produce byte-identical structured results.

Any hard-constraint violation, live delivery path, invented route, unit change,
missing evidence reference, or reviewer-identified unsafe output fails the
experiment. Insufficient evidence, no eligible candidate, or a calculation
error must produce a structured abstention with reasons.

## Rust versus Python benchmark and decision

The repository includes equivalent dependency-free candidate-kernel benchmarks:

```sh
rustc -O apps/api/examples/ft501_candidate_benchmark.rs -o /tmp/ft501_candidate_benchmark
/tmp/ft501_candidate_benchmark
python3 scripts/ft501_candidate_benchmark.py
```

Both evaluate 800,000 fixed candidate paths and emit checksum
`73791440.510`. Two July 22, 2026 development-machine runs measured Rust at
43.971–52.927 ms and Python at 1,605.756–2,771.787 ms. Comparing the slower
Rust result with the faster Python result still gives Rust about a 30x
kernel-throughput advantage; the individual runs ranged to about 63x. This
measures only the geometry/scoring kernel, is expected to vary by machine and
load, and is not an end-to-end or operational claim.

Rust is selected for FT-502. The problem is bounded enumeration, not continuous
or mixed-integer optimization; Rust already owns the typed geometry, route,
hazard, replay, and evidence boundaries. A Python service would add deployment,
serialization, observability, security, and version-skew surfaces without a
needed numerical library. Re-open the decision only if a later, separately
approved experiment proves a need for a mature Python-only optimization library
or model workflow that cannot be met safely in the modular Rust backend.

## Human and AI boundary

Deterministic code owns input validation, eligibility, scoring, selection, and
abstention. The candidate is labeled “offline recommendation for review.” A
human reviewer may accept, reject, or annotate it, but no acceptance triggers an
external action. FT-503 may draft wording only from an explicitly approved,
minimized structured result and must retain a deterministic template fallback.
