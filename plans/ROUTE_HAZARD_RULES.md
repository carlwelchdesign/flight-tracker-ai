# Route–Hazard Rule Contract

FT-203 introduces deterministic rule version `1`, identified as `route_hazard_proximity`. It is a pure Rust domain policy: it does not read the database, call providers, inspect wall-clock time, or depend on Axum. FT-204 can consume its decision and evidence when creating and managing alerts.

## Inputs

Each evaluation receives:

- one versioned canonical `PlannedRoute` with an ordered WGS84 path;
- one versioned canonical `WeatherHazard` with validity, status, altitude volume, and a closed WGS84 polygon;
- an explicit UTC evaluation time;
- the route altitude band for the evaluated portion of flight; and
- optional progress expressed as a segment index and fraction on the ordered route.

Route progress is directional. When supplied, only the remaining ordered path is eligible. If the full route is within the configured margin but the remaining path is not, the evidence reports `behind_route_progress`.

The rule rejects cross-operator inputs, zero versions, invalid time windows, degenerate routes, invalid/self-intersecting polygons, invalid progress, inverted altitude bands, and mixed reference frames inside one band.

## Version 1 policy

The default configuration uses a 25 NM proximity margin and 1 NM geometry resolution. Configuration is explicit, validated, and repeated in evidence.

An evaluation matches only when all of the following are true:

1. The route version is effective at the evaluation time. Route effective windows are half-open: `effective_from <= time < effective_to`. A missing `effective_to` remains active.
2. The hazard is active and its inclusive validity window contains the evaluation time. Cancelled hazards never match.
3. Route and hazard altitude bands overlap.
4. The remaining route intersects the footprint or comes within the configured proximity margin.

A bounded hazard with missing route altitude returns `indeterminate`, not a silent match or clear result. Mean-sea-level and flight-level values are compared in a common pressure/MSL advisory frame after meters are converted to feet. AGL or ellipsoid values are compared only with the same reference frame; incompatible frames are `indeterminate`. An unbounded hazard altitude applies at every route altitude.

## Geometry method

- Coordinate order remains longitude, latitude throughout.
- Route segments and polygon edges are densified along spherical great-circle arcs.
- Planar topology is applied only after great-circle densification and longitude unwrapping; this supports antimeridian crossings without treating them as globe-spanning lines.
- Closest approach uses spherical point-to-great-circle-arc measurements and is reported in nautical miles.
- Intersection encounter distance is bounded by the configured geometry resolution. Version 1 records that resolution in evidence so a future algorithm change requires a rule-version decision rather than silently changing prior meaning.

## Evidence

Every decision contains:

- route ID and route version;
- hazard ID and hazard revision;
- rule ID and rule version;
- evaluation time;
- route and hazard temporal states;
- altitude and horizontal relations;
- remaining-path and full-route closest approach;
- closest route distance from the ordered route start;
- route progress distance from the route start; and
- proximity margin and geometry resolution.

The decision is `match`, `no_match`, or `indeterminate`. Evidence is serializable and deterministic for the same canonical inputs, evaluation time, and configuration.

## Golden and independent review evidence

The versioned golden set is [route-hazard-golden-v1.json](../fixtures/rules/route-hazard-golden-v1.json). It covers direct intersection, near miss within margin, expired and cancelled hazards, non-overlapping altitude, a hazard behind route progress, missing route altitude near a hazard, and missing altitude on a horizontally clear route. Each case includes a plain-language expected-outcome rationale.

The source replay is [m2-route-hazard-v1.json](../fixtures/replay/m2-route-hazard-v1.json). Replay normalization is repeated from the immutable JSON, and serialized decisions must be byte-identical across reloads.

The API/PostGIS CI job independently re-evaluates the golden geometry, temporal windows, altitude overlap, direction, closest approach, and final outcome using PostGIS 3.5. This cross-engine oracle is separate from the Rust `geo` implementation and prevents the rule from grading its own spatial expectations.

## Non-goals

- Creating, ranking, deduplicating, or persisting alerts; FT-204 owns that lifecycle.
- Inferring a route altitude profile from aircraft position or flight plans.
- Predicting waypoint arrival times or weather evolution.
- Certified separation, dispatch release, or autonomous rerouting.
