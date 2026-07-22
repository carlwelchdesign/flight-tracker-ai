import type { FlightView } from "./fleet-api";
import type { AirportObservation, Hazard } from "./weather-api";

const OPERATOR_ID = "00000000-0000-0000-0000-000000000999";

export const PUBLIC_DEMO_FLIGHTS: FlightView[] = [
  flight("101", "FT101", "SFO", "LAX", "active", -121.95, 37.25, 24_000, 142, 435, "2026-07-20T16:01:00Z"),
  flight("202", "FT202", "SEA", "SFO", "scheduled", -122.3088, 47.4502, 433, 160, 0, "2026-07-20T16:00:01Z"),
  flight("303", "FT303", "LAS", "SFO", "active", -121.62, 37.18, 27_000, 315, 438, "2026-07-20T16:01:00Z"),
];

export const PUBLIC_DEMO_HAZARDS: Hazard[] = [
  {
    id: "hazard-1",
    operator_id: OPERATOR_ID,
    schema_version: 1,
    source: {
      envelope_id: "00000000-0000-0000-0000-000000000901",
      provider: "noaa-awc",
      feed: "airsigmet",
      provider_record_id: "21W",
    },
    times: eventTimes("2026-07-21T16:00:00Z"),
    external_series_id: "KKCI:21W:2026-07-21",
    revision: 2,
    supersedes_id: "00000000-0000-0000-0000-000000000900",
    status: "active",
    issued_at: "2026-07-21T16:00:00Z",
    provider_received_at: "2026-07-21T16:00:04Z",
    hazard_type: "convective_cell",
    severity: "significant",
    valid_from: "2026-07-21T16:00:00Z",
    valid_to: "2026-07-21T16:15:00Z",
    altitude_band: {
      lower: { value: 18_000, unit: "feet", reference: "flight_level" },
      upper: { value: 42_000, unit: "feet", reference: "flight_level" },
    },
    footprint: {
      exterior: [
        { longitude_degrees: -121.9, latitude_degrees: 37.1 },
        { longitude_degrees: -121.4, latitude_degrees: 37.1 },
        { longitude_degrees: -121.4, latitude_degrees: 37.5 },
        { longitude_degrees: -121.9, latitude_degrees: 37.5 },
        { longitude_degrees: -121.9, latitude_degrees: 37.1 },
      ],
    },
  },
];

export const PUBLIC_DEMO_OBSERVATIONS: AirportObservation[] = [
  {
    id: "observation-1",
    operator_id: OPERATOR_ID,
    schema_version: 1,
    source: {
      envelope_id: "00000000-0000-0000-0000-000000000902",
      provider: "noaa-awc",
      feed: "metar",
      provider_record_id: "KSFO-1600",
    },
    times: eventTimes("2026-07-21T16:00:00Z"),
    station_code: "KSFO",
    report_type: "METAR",
    raw_text: "METAR KSFO 211600Z 28012KT 10SM FEW010 18/13 A2992",
    provider_received_at: "2026-07-21T16:00:03Z",
    point: { longitude_degrees: -122.3656, latitude_degrees: 37.6196 },
    wind_direction_true_degrees: 280,
    wind_speed: { value: 12, unit: "knots" },
    wind_gust: null,
    visibility_statute_miles: 10,
    visibility_greater_than: false,
    ceiling: null,
    flight_category: "visual",
  },
];

function flight(
  suffix: string,
  callsign: string,
  origin: string,
  destination: string,
  status: FlightView["flight"]["status"],
  longitude: number,
  latitude: number,
  altitudeFeet: number,
  headingTrueDegrees: number,
  groundSpeedKnots: number,
  eventTime: string,
): FlightView {
  const source = {
    envelope_id: `00000000-0000-0000-0000-000000000${suffix}`,
    provider: "simulation",
    feed: "portfolio-replay-v1",
    provider_record_id: `record-${suffix}`,
  };
  const times = eventTimes(eventTime);
  const flightId = `00000000-0000-0000-0000-000000000${suffix}`;
  return {
    flight: {
      id: flightId,
      operator_id: OPERATOR_ID,
      schema_version: 1,
      source,
      times,
      callsign,
      aircraft_registration: `N${suffix}FT`,
      origin_airport_code: origin,
      destination_airport_code: destination,
      scheduled_departure_at: "2026-07-21T15:30:00Z",
      scheduled_arrival_at: "2026-07-21T17:30:00Z",
      status,
    },
    latest_position: {
      id: `10000000-0000-0000-0000-000000000${suffix}`,
      operator_id: OPERATOR_ID,
      flight_id: flightId,
      schema_version: 1,
      source,
      times,
      point: { longitude_degrees: longitude, latitude_degrees: latitude },
      altitude: { value: altitudeFeet, unit: "feet", reference: "mean_sea_level" },
      heading_true_degrees: headingTrueDegrees,
      ground_speed: { value: groundSpeedKnots, unit: "knots" },
      quality: "observed",
    },
  };
}

function eventTimes(eventTime: string) {
  return {
    event_time: eventTime,
    received_at: eventTime,
    processed_at: eventTime,
  };
}
