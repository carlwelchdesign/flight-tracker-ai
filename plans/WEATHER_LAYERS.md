# Weather and Hazard Layers

This document records the FT-202 display and read-contract decisions for live NOAA Aviation Weather and deterministic replay weather.

## Read contracts

The Rust API exposes three read-only routes:

- `GET /api/hazards` returns the latest revision for each operator and external hazard series. It includes active, cancelled, and recently expired hazards so the interface can communicate lifecycle state instead of silently removing evidence.
- `GET /api/airport-observations` returns the newest observation for each operator and station within the two-hour presentation window.
- `GET /api/source-records/{envelope_id}` returns raw NOAA evidence for a selected normalized record. The query is restricted to `noaa-awc` envelopes so this route cannot expose future private-provider payloads.

Both weather collections include a server-generated timestamp. Normalized records retain source-envelope identity, provider/feed attribution, event/receive/process times, units, and geometry conventions from the canonical event model.

The Next.js backend route proxies only explicit weather paths. Malformed response data is rejected at the web boundary; the last accepted weather layer remains visible when a later refresh fails.

## Map behavior

- Independent checkboxes control hazard polygons and METAR station markers, with visible item counts.
- The source summary shows provider attribution, the layer timestamp, and current, stale, degraded, or unavailable state.
- Hazard polygons encode severity and lifecycle. Expired, cancelled, and upcoming hazards are visually distinct from active hazards.
- A keyboard-focusable hazard marker opens an inspector with revision, severity, altitude band, validity, issue time, provider/feed attribution, and raw NOAA evidence access.
- METAR markers expose station, flight category, observation time, and current/stale state to assistive technology.
- Cancelling or expiring a hazard removes it from flight-attention scoring without removing its visible evidence.
- Simulation weather uses the replay virtual clock; NOAA weather uses the accepted weather snapshot time.

## Failure and recovery

Weather is an independently degradable layer. If one of the hazard, observation, or source-health reads fails, the fleet board and flight detail remain available, the last accepted weather layer is retained, and the map presents an explicit retry action. An unavailable response is never relabeled as current.

Operators can use the retry action after checking API readiness and NOAA source health. Raw evidence access intentionally returns `404` for missing or non-NOAA envelopes and `503` when storage is unavailable.

## Performance evidence

The deterministic benchmark renders a representative western-region set of 300 hazard polygons and 75 METAR stations. On 2026-07-20, the complete 375-item static render averaged 48.29 ms and projection averaged 0.19 ms on the development machine. The benchmark is repeatable with:

```sh
cd apps/web
npm run benchmark:weather-layers
```

Browser verification covered 1180 x 720 and 820 x 900 viewports with no horizontal overflow. Keyboard selection opened the hazard inspector, layer controls removed and restored their visual data, and the development build reported LCP 624 ms and CLS 0.03. These figures are regression evidence, not production service-level guarantees.

## Verification

- Rust unit and route suites, including the PostGIS schema contract
- Strict workspace Clippy and release build
- Web component and transport-boundary tests
- Frontend lint, typecheck, production build, and dependency audit
- Representative weather-layer benchmark
- Browser interaction, accessibility, responsive-layout, and console-error checks
