# FT-410 — Regional live-traffic selector

Status: In progress

Branch: `feat/ft-410-regional-traffic-selector`
Latest implementation commit: Pending
Final commit: Pending
Pull request: Pending
Owner: Backend and full-stack product engineering

## Outcome

Replace the San Francisco-only public traffic experience with a curated United
States region selector. Each choice maps to a server-owned, bounded ADSB.lol
circle and receives the same truthful freshness, replay fallback, trajectory,
and attribution treatment as the existing Bay Area view.

The application must not become an arbitrary coordinate proxy. Rust owns the
allowlist and polling schedule, all provider data remains ephemeral and
`no-store`, and every region remains best-effort rather than nationwide or
authoritative coverage.

## Acceptance checklist

### Backend boundary

- [x] Rust defines a small curated catalog of named airport regions and rejects
      unknown region identifiers without contacting ADSB.lol. Evidence:
      `live_positions::regions` tests and public HTTP 404 contract.
- [x] Each region uses a bounded circle, a 30-second-or-slower refresh cadence,
      one request in flight per region, existing response limits/retries, and
      no database persistence. Evidence: seven 50-NM runtimes are staggered
      across a 75-second cycle; ADR-013 records the measured rate boundary.
- [x] The public endpoint returns only the selected region's sanitized aircraft
      and retains `Cache-Control: no-store` plus ADSB.lol/ODbL attribution.
- [x] Failure and freshness status are independent per region and never erase
      another region's last accepted in-memory picture. Evidence: deterministic
      region projection keys plus existing status-store isolation tests.

### Public experience

- [x] A keyboard-accessible region/airport selector offers SFO, LAX, SEA, DEN,
      ORD, ATL, and JFK coverage without implying nationwide completeness.
- [x] Changing region updates the heading, summary, camera, live list,
      selection, and trajectory state without a page reload.
- [x] Loading, current, stale, unavailable, and replay states stay explicit for
      the selected region.
- [x] Existing selected-aircraft-first layout, motion animation, trajectory
      disclosure, weather controls, and mobile behavior remain intact.

### Verification and delivery

- [x] Rust tests prove allowlist rejection, regional isolation, no-store, and
      failure independence. Evidence: 94 Rust library tests and 13 binary tests pass.
- [x] Web tests prove region requests, state reset, labels, selection, and
      accessible control behavior. Evidence: all 95 web tests pass.
- [ ] Formatting, Clippy, lint, typecheck, unit tests, production build, and
      API/PostGIS smoke pass.
- [x] Runtime browser verification covers at least two regions on desktop and
      one mobile viewport. Evidence: production-built local browser run showed
      132 current SFO aircraft, 173 current LAX aircraft, 173 rendered LAX
      markers, no application errors, and zero horizontal overflow at 390x844.
- [ ] Ticket branch, intentional commits, PR, passing checks, and verification
      evidence are recorded before completion.

## Non-goals

- Nationwide polling, arbitrary coordinates/radii, route or schedule facts,
  persistent ADS-B history, and commercial-grade coverage guarantees.
- New cloud, radar, or wind products; those belong to FT-411.

## Current verification evidence

- Live Rust sampling completed a full 75-second staggered cycle with all seven
  regions current, zero consecutive failures, and 77–187 aircraft per region.
- `cargo fmt --all -- --check`, Clippy with warnings denied, all Rust tests,
  all 95 web tests, TypeScript, ESLint, `next build`, and `git diff --check` pass.
- CI's independent API/PostGIS job, branch commit, PR, and PR checks remain the
  final completion evidence.
