export type EventTimes = {
  event_time: string;
  received_at: string;
  processed_at: string;
};

export type SourceAttribution = {
  envelope_id: string;
  provider: string;
  feed: string;
  provider_record_id: string | null;
};

export type Flight = {
  id: string;
  operator_id: string;
  schema_version: number;
  source: SourceAttribution;
  times: EventTimes;
  callsign: string | null;
  aircraft_registration: string | null;
  origin_airport_code: string | null;
  destination_airport_code: string | null;
  scheduled_departure_at: string | null;
  scheduled_arrival_at: string | null;
  status: "scheduled" | "active" | "diverted" | "landed" | "cancelled" | "unknown";
};

export type AircraftPosition = {
  id: string;
  operator_id: string;
  flight_id: string;
  schema_version: number;
  source: SourceAttribution;
  times: EventTimes;
  point: {
    longitude_degrees: number;
    latitude_degrees: number;
  };
  altitude: {
    value: number;
    unit: "feet" | "meters";
    reference: string;
  } | null;
  heading_true_degrees: number | null;
  ground_speed: {
    value: number;
    unit: "knots" | "kilometers_per_hour";
  } | null;
  quality: "observed" | "fused" | "estimated" | "unknown";
};

export type FlightView = {
  flight: Flight;
  latest_position: AircraftPosition | null;
};

export type PageMetadata = {
  page: number;
  page_size: number;
  total_items: number;
  total_pages: number;
};

export type FlightPage = {
  data: FlightView[];
  pagination: PageMetadata;
};

export type CanonicalEvent = {
  event_type: string;
  data: Record<string, unknown>;
};

export type FleetEvent = {
  id: number;
  operator_id: string;
  flight_id: string | null;
  envelope_id: string;
  event_time: string;
  source: SourceAttribution | null;
  event: CanonicalEvent;
};

export type TimelinePage = {
  data: FleetEvent[];
  pagination: PageMetadata;
};

export type FleetLoadResult =
  | { state: "ready"; page: FlightPage }
  | { state: "disconnected"; message: string };

const DEFAULT_API_BASE_URL = "http://localhost:8080";

export async function getInitialFleet(assertion: string): Promise<FleetLoadResult> {
  try {
    const response = await fetch(
      `${getApiBaseUrl()}/api/flights?page=1&page_size=100`,
      {
        headers: { authorization: `Bearer ${assertion}` },
        cache: "no-store",
        signal: AbortSignal.timeout(2_500),
      },
    );
    if (!response.ok) {
      return {
        state: "disconnected",
        message: `Fleet API returned HTTP ${response.status}`,
      };
    }
    return { state: "ready", page: parseFlightPage(await response.json()) };
  } catch (error) {
    return {
      state: "disconnected",
      message: error instanceof Error ? error.message : "Fleet API is unavailable",
    };
  }
}

export function getApiBaseUrl(): string {
  return (process.env.API_BASE_URL ?? DEFAULT_API_BASE_URL).replace(/\/$/, "");
}

export function parseFlightPage(value: unknown): FlightPage {
  if (!isRecord(value) || !Array.isArray(value.data) || !isPageMetadata(value.pagination)) {
    throw new Error("Fleet API returned an unexpected list payload");
  }
  const data = value.data.map(parseFlightView);
  return { data, pagination: value.pagination };
}

export function parseTimelinePage(value: unknown): TimelinePage {
  if (!isRecord(value) || !Array.isArray(value.data) || !isPageMetadata(value.pagination)) {
    throw new Error("Fleet API returned an unexpected timeline payload");
  }
  return { data: value.data.map(parseFleetEvent), pagination: value.pagination };
}

export function parseFleetEvent(value: unknown): FleetEvent {
  if (
    !isRecord(value) ||
    typeof value.id !== "number" ||
    typeof value.operator_id !== "string" ||
    (value.flight_id !== null && typeof value.flight_id !== "string") ||
    typeof value.envelope_id !== "string" ||
    typeof value.event_time !== "string" ||
    !isRecord(value.event) ||
    typeof value.event.event_type !== "string" ||
    !isRecord(value.event.data)
  ) {
    throw new Error("Event stream returned an unexpected payload");
  }
  return value as FleetEvent;
}

function parseFlightView(value: unknown): FlightView {
  if (!isRecord(value) || !isFlight(value.flight)) {
    throw new Error("Fleet API returned an unexpected flight payload");
  }
  if (value.latest_position !== null && !isPosition(value.latest_position)) {
    throw new Error("Fleet API returned an unexpected position payload");
  }
  return value as FlightView;
}

function isFlight(value: unknown): value is Flight {
  return (
    isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.operator_id === "string" &&
    typeof value.schema_version === "number" &&
    isSource(value.source) &&
    isEventTimes(value.times) &&
    isOptionalString(value.callsign) &&
    isOptionalString(value.aircraft_registration) &&
    isOptionalString(value.origin_airport_code) &&
    isOptionalString(value.destination_airport_code) &&
    isOptionalString(value.scheduled_departure_at) &&
    isOptionalString(value.scheduled_arrival_at) &&
    ["scheduled", "active", "diverted", "landed", "cancelled", "unknown"].includes(
      String(value.status),
    )
  );
}

function isPosition(value: unknown): value is AircraftPosition {
  return (
    isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.flight_id === "string" &&
    isSource(value.source) &&
    isEventTimes(value.times) &&
    isPoint(value.point) &&
    (value.heading_true_degrees === null || typeof value.heading_true_degrees === "number") &&
    (value.altitude === null || isRecord(value.altitude)) &&
    (value.ground_speed === null || isRecord(value.ground_speed))
  );
}

function isPageMetadata(value: unknown): value is PageMetadata {
  return (
    isRecord(value) &&
    [value.page, value.page_size, value.total_items, value.total_pages].every(
      (item) => typeof item === "number",
    )
  );
}

function isSource(value: unknown): value is SourceAttribution {
  return (
    isRecord(value) &&
    typeof value.envelope_id === "string" &&
    typeof value.provider === "string" &&
    typeof value.feed === "string" &&
    isOptionalString(value.provider_record_id)
  );
}

function isEventTimes(value: unknown): value is EventTimes {
  return (
    isRecord(value) &&
    typeof value.event_time === "string" &&
    typeof value.received_at === "string" &&
    typeof value.processed_at === "string"
  );
}

function isPoint(value: unknown): boolean {
  return (
    isRecord(value) &&
    typeof value.longitude_degrees === "number" &&
    typeof value.latitude_degrees === "number"
  );
}

function isOptionalString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
