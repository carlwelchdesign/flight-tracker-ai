# NOAA Aviation Weather API Observations

Observed: 2026-07-20 UTC

These are reduced, human-reviewable observations from successful HTTPS requests.
They prove endpoint shape and timestamp/geometry availability; they are not an
availability or latency SLA. Full response hashes allow the research run to be
distinguished without committing large, fast-expiring operational payloads.

Requests used this user agent:

```text
flight-tracker-ai-feasibility/0.1 (github.com/carlwelchdesign/flight-tracker-ai)
```

## KSFO METAR JSON

Request:

```text
GET https://aviationweather.gov/api/data/metar?ids=KSFO&format=json&hours=2
```

Transport observation:

```text
retrieved_at: 2026-07-20T20:34:03Z
HTTP Date: Mon, 20 Jul 2026 20:34:04 GMT
Content-Type: application/json; charset=utf-8
Content-Length: 1106
Cache-Control: max-age=60
SHA-256: 5c31486d1488def6db7f2dfee0783e6ff8475c94629f267adcbe01beffd9c8f0
```

Selected first record:

```json
{
  "icaoId": "KSFO",
  "reportTime": "2026-07-20T20:00:00.000Z",
  "receiptTime": "2026-07-20T20:00:32.550Z",
  "obsTime": 1784577360,
  "rawOb": "METAR KSFO 201956Z 33008KT 10SM FEW006 SCT013 BKN100 19/14 A2995 RMK AO2 SLP141 T01890144",
  "lat": 37.6196,
  "lon": -122.3656,
  "temp": 18.9,
  "dewp": 14.4,
  "wdir": 330,
  "wspd": 8,
  "visib": "10+",
  "altim": 1014.3,
  "fltCat": "VFR"
}
```

Observed contract implications:

- report and provider receipt times are distinct and must both be preserved;
- this sample was received by AWC 32.55 seconds after `reportTime`;
- `obsTime` is numeric while report/receipt fields are ISO timestamps;
- decoded values and the raw observation are both available;
- cache headers permit a one-minute server-side cache.

## Domestic SIGMET GeoJSON

Request:

```text
GET https://aviationweather.gov/api/data/airsigmet?format=geojson
```

Transport observation:

```text
retrieved_at: 2026-07-20T20:34:50Z
HTTP Date: Mon, 20 Jul 2026 20:34:50 GMT
Content-Type: application/json; charset=utf-8
Content-Length: 15692
Cache-Control: max-age=180
Feature count: 16
SHA-256: 1bbfb4075b1618668ce62e97ba43b2986cf80f5faea0f5d5b99aef7cc8d96640
```

Selected feature:

```json
{
  "type": "Feature",
  "properties": {
    "icaoId": "KKCI",
    "airSigmetType": "SIGMET",
    "alphaChar": "W",
    "hazard": "CONVECTIVE",
    "seriesId": "64W",
    "validTimeFrom": "2026-07-20T19:55:00.000Z",
    "validTimeTo": "2026-07-20T21:55:00.000Z",
    "severity": 5,
    "altitudeHi1": 45000,
    "altitudeLow1": null
  },
  "geometry": {
    "type": "Polygon",
    "coordinates": [[
      [-109.521, 37.338],
      [-108.672, 35.34],
      [-112.627, 34.579],
      [-113.041, 37.097],
      [-109.521, 37.338]
    ]]
  }
}
```

Observed contract implications:

- hazard validity is an interval and is independent of fetch time;
- altitude bounds can be null and require explicit units from the schema/domain
  adapter rather than inference at the UI;
- GeoJSON coordinate order is longitude then latitude;
- polygon rings are closed in the observed response.

## G-AIRMET GeoJSON

Request:

```text
GET https://aviationweather.gov/api/data/gairmet?format=geojson
```

Transport observation:

```text
retrieved_at: 2026-07-20T20:37:47Z
HTTP Date: Mon, 20 Jul 2026 20:37:47 GMT
Content-Type: application/json; charset=utf-8
Content-Length: 16471
Cache-Control: max-age=60
Feature count: 23
SHA-256: a0961d9ddd7f116dc1857731407b04f3a96a1179e36041ed2b4a5211178ed27d
```

Selected feature:

```json
{
  "product": "ZULU",
  "hazard": "FZLVL",
  "tag": "1C",
  "issueTime": "2026-07-20T17:14:00.000Z",
  "validTime": "2026-07-20T21:00:00.000Z",
  "receiptTime": "2026-07-20T17:14:55.194Z",
  "forecast": 6,
  "level": "160",
  "geometryType": "LineString"
}
```

This observation proves forecast hour, issue/receipt/valid times, level, and
geometry are separately represented. FT-201 must still capture a complete allowed
fixture for the production adapter and schema tests.

## Verification boundaries

- No authenticated provider data or credentials were used.
- The observations are intentionally reduced; future adapter tests must capture
  complete allowed fixtures and verify against the published OpenAPI schema.
- Product issue cadence is not the same as API cache cadence.
- A successful response does not remove the requirement for source-health and
  stale-data behavior.
