# Operations Console Contract

## Purpose

The M1 console helps a dispatcher or operations analyst scan a simulated fleet, identify flights needing attention, select one flight, and inspect the evidence behind its state. It is advisory: it does not dispatch aircraft, change a flight plan, or send an operational message.

## Primary workflow

1. Confirm the connection and replay state in the header.
2. Scan the flight board for attention, variance, and freshness.
3. Select a flight from either the board or the map.
4. Review route, position, timing, nearby hazards, and source-attributed timeline evidence in the detail panel.
5. Use replay controls to pause, reset, or change simulation speed during development.
6. Filter and scan the ranked dispatcher queue, inspect its score and evidence, then acknowledge, assign, comment, dismiss with a structured reason, or resolve the alert.

Selection is a single shared state. A selection made on the map or board updates the map emphasis, board row, and detail panel together.

## Information hierarchy

- The header carries product identity, distinct service and stream states, last event and receipt times, and development-only replay/outage controls.
- The map and board are equal primary surfaces for spatial and comparative scanning.
- The detail panel is the evidence surface for the selected flight.
- The dispatcher queue is a separate evidence and action surface. It shows score components, rule/score/route/hazard versions, proximity, lifecycle, assignment, and audit history. Severity, status, flight, event time, and assigned-user filters sit above the bounded review workspace.
- Administrators also receive a tenant-scoped audit review below the operational workspace with redacted export and explicit privileged-action monitoring signals. Other roles do not receive or access this surface.
- Attention language remains descriptive (`watch`, `normal`) and never claims to be a safety determination.
- Timeline entries retain their source and event time so operators can distinguish observation from receipt.

## Operational states

- **Loading:** skeleton surfaces preserve the eventual layout while initial data is fetched.
- **Empty:** the board explains that no active flights are available and offers a simulation start action when replay controls exist.
- **Disconnected:** an explicit banner says live updates are unavailable while the last known snapshot remains visible.
- **Reconnecting:** the connection label and banner explain that the stream is recovering.
- **Source outage:** a prominent development banner says source events are intentionally suspended while the healthy service and stream remain distinguishable.
- **Critical worker degraded:** service health names the worker state without hiding the last accepted operational picture.
- **Stale:** flights whose last observation exceeds the freshness threshold are labeled stale in the board and detail panel.
- **Partial data:** a warning identifies when the initial fleet or a later refresh failed while retained data remains usable.
- **Fatal error:** the route-level error view provides a retry action.
- **Alert empty:** the queue explains that clear and indeterminate route–hazard cases are intentionally suppressed.
- **Alert unavailable:** the queue preserves the rest of the operational picture and offers a retry.
- **Action pending/error:** controls are disabled while a command is in flight; validation and server errors remain visible without discarding the selected evidence.
- **Concurrent action:** the stale command is rejected, current detail is reloaded, and the dispatcher is told to review the winner's state before retrying.
- **Assignment directory unavailable:** the queue, evidence, and other actions remain available while assignment controls explain the partial outage.
- **Audit review unavailable:** the administrator surface states that review is unavailable without hiding the operational console or presenting incomplete evidence as complete.

## Accessibility and interaction

- Aircraft markers and callsigns are native buttons with descriptive accessible names and pressed state.
- Keyboard users can tab to any flight in the board or map and select it with Enter or Space.
- Visible focus indicators meet the console contrast direction and are not removed.
- Connection changes and selected-flight attention are exposed through live status regions without making routine position updates noisy.
- Color is reinforced with text labels, borders, and symbols; it is never the only attention indicator.
- Reduced-motion preferences disable nonessential transitions and status animation.

## Responsive contract

The agreed minimum dense desktop viewport is **1180 by 720 CSS pixels**. At that size, map, board, and detail remain simultaneously usable without horizontal page scrolling. Below 1100 pixels, the primary panels stack while retaining the same information and selection behavior. At phone widths, the flight board becomes a vertically scrollable comparison surface and controls wrap into full-width groups.

## M1 constraints

- The SVG map uses fixed western United States bounds and a small static airport-coordinate catalog for the deterministic M1 scenario.
- The replay-backed projection is in memory; a service restart clears its current snapshot until the scenario runs again.
- Weather shown in M1 is scenario evidence, not a live or certified aviation weather product.
- Authentication and saved workflows remain later-ticket work. Paid commercial feeds are optional future production work and do not block the portfolio demonstration.

## Verification checklist

- Run component interaction tests, lint, type checking, and a production build.
- Verify loaded, empty, disconnected, stale, and error behavior.
- Inspect at 1440 by 900, 1180 by 720, and a compact/mobile viewport.
- Select flights from both the board and map using keyboard input and confirm all three surfaces synchronize.
- Confirm the console never hides source age or presents an advisory as an operational command.
