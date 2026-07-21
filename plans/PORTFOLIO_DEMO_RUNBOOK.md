# Recruiter Portfolio Demo Runbook

## Promise

This is a non-commercial portfolio demonstration for recruiters and hiring
managers. It is not certified, connected to an airline, or suitable for flight
planning or dispatch. Deterministic replay is the reliable demonstration; live
NOAA weather and optional ADSB.lol positions are supporting evidence with
visible source and freshness limits.

## Five-minute path

1. Start with the trust banner and explain the three boundaries: simulation,
   live weather, and optional best-effort positions.
2. Start or reset replay, select the hazard-adjacent flight, and show how the
   map, board, detail, timeline, and evidence agree.
3. Open the dispatcher queue, review the score breakdown, and perform one
   explicit human action. Explain that the system does not send instructions.
4. Use the development **Test outage** control. Point out that the source-outage
   banner appears while the last picture, service state, and stream state remain
   distinct. Restore the feed or reset replay.
5. If the optional position layer is enabled, show its coverage, freshness,
   attribution, and `Position only` labels. If it is degraded or unavailable,
   select **Use replay view** and continue without apologizing for the source.

## Failure-safe presentation

- Never wait on an external feed during the demo. If it is not already current,
  leave it disabled or use replay immediately.
- A reconnecting stream may retain the last picture; describe it as retained,
  not current.
- A degraded/unavailable ADSB.lol card is an intentional product state. It
  demonstrates transparent source handling, not a failed demo.
- If service readiness fails, stop interactive mutations, capture the named
  failed boundary, restart the stack, reset replay, and verify the recovery
  checklist before continuing.
- Do not run the database restore or receiver-overflow drill in front of a
  viewer. Cite the linked CI evidence; those drills use isolated infrastructure.

## Pre-demo check

- [ ] The root trust banner is visible.
- [ ] `/health` and `/readiness` are healthy.
- [ ] Authenticated service health names every critical worker as running.
- [ ] Replay reset produces the expected three-flight picture and one
      route-hazard alert.
- [ ] The outage control shows and restores the source-outage state.
- [ ] Optional live positions are either disabled or visibly attributed and
      fresh; replay remains selectable.
- [ ] No production credentials, real operator data, or provider payloads are
      present in the browser, logs, or prepared materials.

## Recovery confirmation

After any restart, require healthy service/readiness, a live SSE connection, an
advancing replay event time, the expected alert count, and preserved alert
action history. A complete replay reset is mandatory after a reported alert
consumer lag because the bounded in-memory channel does not claim recovery of
skipped unique events.
