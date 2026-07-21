use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::{
    AircraftPosition, CanonicalEvent, Flight, FlightId, OperatorId, ProviderEnvelopeId,
    SourceAttribution,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightView {
    pub flight: Flight,
    pub latest_position: Option<AircraftPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PageMetadata {
    pub page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlightPage {
    pub data: Vec<FlightView>,
    pub pagination: PageMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimelinePage {
    pub data: Vec<FleetEvent>,
    pub pagination: PageMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetEvent {
    pub id: u64,
    pub operator_id: OperatorId,
    pub flight_id: Option<FlightId>,
    pub envelope_id: ProviderEnvelopeId,
    pub event_time: DateTime<Utc>,
    pub source: Option<SourceAttribution>,
    pub event: CanonicalEvent,
}
