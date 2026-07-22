# FT-405 hosted live-source failure verification

Status: Verified

The closeout branch used an isolated Vercel Preview override for
`API_BASE_URL` pointing to a reserved, non-resolving `.invalid` origin. This
forces the deployed Next.js public proxy to fail without changing production,
the shared Preview environment, the Rust service, or any provider data.

## Captured result

- Deployment: `GENwMrgCEMzCfH4VVXZR3bEG7jkV`
- Preview:
  <https://flight-tracker-ai-git-test-ft-1bbff0-carlwelchdesigns-projects.vercel.app>
- Commit: `5d0de6c`
- Verified: 2026-07-22

The hosted browser first showed the live source connecting and then changed to
`Deterministic replay fallback` with the explicit message that live traffic was
unavailable. The map retained three selectable replay aircraft, OpenFreeMap,
OpenMapTiles, OpenStreetMap, NOAA, UTC/WGS84, and not-for-navigation evidence.
The selected aircraft remained usable and continued to distinguish scenario
facts from visual interpolation.

Selecting `Try live again` repeated the failed request without reloading the
page or removing the replay aircraft. The fallback, selection, labels, and
attribution remained intact.

At a 390 by 844 viewport, the document width was exactly 390 pixels, the map
was 460 pixels high, the fallback message was visible, and all three replay
aircraft remained in the current-picture list. The browser recorded zero
console errors.

The branch-specific override was removed immediately after the verification.
A branch-filtered Vercel environment listing returned no `API_BASE_URL`
override, leaving the shared Preview and Production values unchanged.
