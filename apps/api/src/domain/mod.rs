//! Provider-independent aviation domain contracts.
//!
//! Raw provider envelopes are deliberately separate from normalized facts. The
//! types in this module do not depend on Axum, SQLx, or any provider payload so
//! ingestion adapters can change without changing operational policy.

mod geometry;
mod identity;
mod model;

pub use geometry::{
    CoordinateError, GeoLineString, GeoPoint, GeoPolygon, LatitudeDegrees, LongitudeDegrees,
};
pub use identity::{
    AircraftPositionId, AlertActionId, AlertId, FlightId, OperatorId, PlannedRouteId,
    ProviderEnvelopeId, SourceHealthId, WeatherHazardId,
};
pub use model::{
    AircraftPosition, Alert, AlertAction, AlertActionKind, AlertLifecycle, AlertSeverity, Altitude,
    AltitudeBand, AltitudeReference, AltitudeUnit, CanonicalEvent, EventTimes, Flight,
    FlightStatus, HazardSeverity, HeadingDegrees, MeasurementError, PlannedRoute, ProviderEnvelope,
    SchemaVersion, SourceAttribution, SourceHealth, SourceHealthState, SourceQuality, Speed,
    SpeedUnit, TimeValidationError, WeatherHazard,
};
