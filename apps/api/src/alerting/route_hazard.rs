use chrono::{DateTime, Utc};
use geo::{
    Closest, Distance, Haversine, HaversineClosestPoint, InterpolatePoint, Intersects, Line,
    LineString, Point, Polygon, Validation,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    Altitude, AltitudeBand, AltitudeReference, AltitudeUnit, PlannedRoute, PlannedRouteId,
    WeatherHazard, WeatherHazardId, WeatherHazardStatus,
};

pub const ROUTE_HAZARD_RULE_ID: &str = "route_hazard_proximity";
pub const ROUTE_HAZARD_RULE_VERSION: u32 = 1;

const METERS_PER_NAUTICAL_MILE: f64 = 1_852.0;
const FEET_PER_METER: f64 = 3.280_839_895_013_123;
const MIN_GEOMETRY_RESOLUTION_NM: f64 = 0.1;
const MAX_GEOMETRY_RESOLUTION_NM: f64 = 25.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RouteHazardRuleConfig {
    pub proximity_margin_nm: f64,
    pub geometry_resolution_nm: f64,
}

impl RouteHazardRuleConfig {
    pub fn new(
        proximity_margin_nm: f64,
        geometry_resolution_nm: f64,
    ) -> Result<Self, RuleConfigError> {
        if !proximity_margin_nm.is_finite() || proximity_margin_nm < 0.0 {
            return Err(RuleConfigError::InvalidProximityMargin(proximity_margin_nm));
        }
        if !geometry_resolution_nm.is_finite()
            || !(MIN_GEOMETRY_RESOLUTION_NM..=MAX_GEOMETRY_RESOLUTION_NM)
                .contains(&geometry_resolution_nm)
        {
            return Err(RuleConfigError::InvalidGeometryResolution(
                geometry_resolution_nm,
            ));
        }
        Ok(Self {
            proximity_margin_nm,
            geometry_resolution_nm,
        })
    }
}

impl Default for RouteHazardRuleConfig {
    fn default() -> Self {
        Self {
            proximity_margin_nm: 25.0,
            geometry_resolution_nm: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteProgress {
    segment_index: usize,
    segment_fraction: f64,
}

impl RouteProgress {
    pub fn new(segment_index: usize, segment_fraction: f64) -> Result<Self, RouteProgressError> {
        if !segment_fraction.is_finite() || !(0.0..1.0).contains(&segment_fraction) {
            return Err(RouteProgressError::InvalidSegmentFraction(segment_fraction));
        }
        Ok(Self {
            segment_index,
            segment_fraction,
        })
    }

    pub const fn segment_index(self) -> usize {
        self.segment_index
    }

    pub const fn segment_fraction(self) -> f64 {
        self.segment_fraction
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum RuleConfigError {
    #[error("proximity margin must be a finite non-negative number of nautical miles, got {0}")]
    InvalidProximityMargin(f64),
    #[error("geometry resolution must be finite and between 0.1 and 25 nautical miles, got {0}")]
    InvalidGeometryResolution(f64),
}

#[derive(Debug, Error, PartialEq)]
pub enum RouteProgressError {
    #[error("route progress segment fraction must be finite and in [0, 1), got {0}")]
    InvalidSegmentFraction(f64),
}

#[derive(Debug, Error, PartialEq)]
pub enum RuleInputError {
    #[error("route and hazard must belong to the same operator")]
    CrossOperator,
    #[error("route version must be positive")]
    InvalidRouteVersion,
    #[error("hazard revision must be positive")]
    InvalidHazardRevision,
    #[error("route effective_to must not precede effective_from")]
    InvalidRouteWindow,
    #[error("hazard valid_to must not precede valid_from")]
    InvalidHazardWindow,
    #[error("route must contain at least two distinct WGS84 points")]
    InvalidRouteGeometry,
    #[error("hazard footprint must be a closed polygon with at least three distinct vertices")]
    InvalidHazardGeometry,
    #[error("route progress segment {segment_index} does not exist")]
    InvalidRouteProgress { segment_index: usize },
    #[error("{subject} altitude band has an upper bound below its lower bound")]
    InvertedAltitudeBand { subject: &'static str },
    #[error("{subject} altitude band mixes incompatible altitude references")]
    MixedAltitudeReferences { subject: &'static str },
}

pub struct RouteHazardInput<'a> {
    pub route: &'a PlannedRoute,
    pub hazard: &'a WeatherHazard,
    pub evaluated_at: DateTime<Utc>,
    pub route_altitude_band: Option<&'a AltitudeBand>,
    pub progress: Option<RouteProgress>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteHazardOutcome {
    Match,
    NoMatch,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteTemporalState {
    Active,
    NotYetEffective,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardTemporalState {
    Active,
    NotYetValid,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalRelation {
    pub route: RouteTemporalState,
    pub hazard: HazardTemporalState,
}

impl TemporalRelation {
    fn permits_match(self) -> bool {
        self.route == RouteTemporalState::Active && self.hazard == HazardTemporalState::Active
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltitudeRelation {
    Overlap,
    Disjoint,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalRelation {
    Intersects,
    WithinMargin,
    Clear,
    BehindRouteProgress,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteHazardEvidence {
    pub route_id: PlannedRouteId,
    pub route_version: u32,
    pub hazard_id: WeatherHazardId,
    pub hazard_revision: u32,
    pub rule_id: String,
    pub rule_version: u32,
    pub evaluated_at: DateTime<Utc>,
    pub temporal_relation: TemporalRelation,
    pub altitude_relation: AltitudeRelation,
    pub horizontal_relation: HorizontalRelation,
    pub closest_approach_nm: f64,
    pub full_route_closest_approach_nm: f64,
    pub proximity_margin_nm: f64,
    pub geometry_resolution_nm: f64,
    pub closest_route_distance_from_start_nm: f64,
    pub route_progress_distance_from_start_nm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteHazardDecision {
    pub outcome: RouteHazardOutcome,
    pub evidence: RouteHazardEvidence,
}

#[derive(Debug, Clone, Copy)]
pub struct RouteHazardRule {
    config: RouteHazardRuleConfig,
}

impl RouteHazardRule {
    pub fn new(config: RouteHazardRuleConfig) -> Result<Self, RuleConfigError> {
        let validated =
            RouteHazardRuleConfig::new(config.proximity_margin_nm, config.geometry_resolution_nm)?;
        Ok(Self { config: validated })
    }

    pub fn evaluate(
        &self,
        input: RouteHazardInput<'_>,
    ) -> Result<RouteHazardDecision, RuleInputError> {
        validate_input(&input)?;

        let route_points = canonical_points(&input.route.path.coordinates);
        let hazard_points = canonical_points(&input.hazard.footprint.exterior);
        let full_assessment = assess_geometry(
            &route_points,
            &hazard_points,
            self.config.geometry_resolution_nm,
        );
        let (remaining_points, progress_distance_nm) =
            remaining_route(&route_points, input.progress)?;
        let remaining_assessment = assess_geometry(
            &remaining_points,
            &hazard_points,
            self.config.geometry_resolution_nm,
        );

        let horizontal_relation = if remaining_assessment.intersects {
            HorizontalRelation::Intersects
        } else if remaining_assessment.closest_approach_nm <= self.config.proximity_margin_nm {
            HorizontalRelation::WithinMargin
        } else if input.progress.is_some()
            && full_assessment.closest_approach_nm <= self.config.proximity_margin_nm
        {
            HorizontalRelation::BehindRouteProgress
        } else {
            HorizontalRelation::Clear
        };
        let temporal_relation = temporal_relation(&input);
        let altitude_relation = altitude_relation(
            input.route_altitude_band,
            input.hazard.altitude_band.as_ref(),
        )?;
        let outcome = match (
            temporal_relation.permits_match(),
            altitude_relation,
            horizontal_relation,
        ) {
            (false, _, _)
            | (_, _, HorizontalRelation::Clear | HorizontalRelation::BehindRouteProgress)
            | (_, AltitudeRelation::Disjoint, _) => RouteHazardOutcome::NoMatch,
            (
                true,
                AltitudeRelation::Indeterminate,
                HorizontalRelation::Intersects | HorizontalRelation::WithinMargin,
            ) => RouteHazardOutcome::Indeterminate,
            (
                true,
                AltitudeRelation::Overlap,
                HorizontalRelation::Intersects | HorizontalRelation::WithinMargin,
            ) => RouteHazardOutcome::Match,
        };

        Ok(RouteHazardDecision {
            outcome,
            evidence: RouteHazardEvidence {
                route_id: input.route.id,
                route_version: input.route.route_version,
                hazard_id: input.hazard.id,
                hazard_revision: input.hazard.revision,
                rule_id: ROUTE_HAZARD_RULE_ID.into(),
                rule_version: ROUTE_HAZARD_RULE_VERSION,
                evaluated_at: input.evaluated_at,
                temporal_relation,
                altitude_relation,
                horizontal_relation,
                closest_approach_nm: quantize_nm(remaining_assessment.closest_approach_nm),
                full_route_closest_approach_nm: quantize_nm(full_assessment.closest_approach_nm),
                proximity_margin_nm: self.config.proximity_margin_nm,
                geometry_resolution_nm: self.config.geometry_resolution_nm,
                closest_route_distance_from_start_nm: quantize_nm(
                    progress_distance_nm + remaining_assessment.closest_route_distance_nm,
                ),
                route_progress_distance_from_start_nm: quantize_nm(progress_distance_nm),
            },
        })
    }
}

impl Default for RouteHazardRule {
    fn default() -> Self {
        Self::new(RouteHazardRuleConfig::default()).expect("default rule configuration is valid")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AltitudeFrame {
    PressureOrMeanSeaLevel,
    AboveGroundLevel,
    Ellipsoid,
}

#[derive(Debug, Clone, Copy)]
struct NormalizedAltitudeBand {
    lower_feet: f64,
    upper_feet: f64,
    frame: AltitudeFrame,
}

#[derive(Debug, Clone, Copy)]
struct GeometryAssessment {
    intersects: bool,
    closest_approach_nm: f64,
    closest_route_distance_nm: f64,
}

fn validate_input(input: &RouteHazardInput<'_>) -> Result<(), RuleInputError> {
    if input.route.operator_id != input.hazard.operator_id {
        return Err(RuleInputError::CrossOperator);
    }
    if input.route.route_version == 0 {
        return Err(RuleInputError::InvalidRouteVersion);
    }
    if input.hazard.revision == 0 {
        return Err(RuleInputError::InvalidHazardRevision);
    }
    if input
        .route
        .effective_to
        .is_some_and(|end| end < input.route.effective_from)
    {
        return Err(RuleInputError::InvalidRouteWindow);
    }
    if input.hazard.valid_to < input.hazard.valid_from {
        return Err(RuleInputError::InvalidHazardWindow);
    }
    if input.route.path.coordinates.len() < 2
        || path_length_nm(&canonical_points(&input.route.path.coordinates)) <= f64::EPSILON
    {
        return Err(RuleInputError::InvalidRouteGeometry);
    }
    let footprint = &input.hazard.footprint.exterior;
    let unique_vertices = footprint
        .iter()
        .take(footprint.len().saturating_sub(1))
        .map(|point| {
            let [longitude, latitude] = point.as_geojson_position();
            (longitude.to_bits(), latitude.to_bits())
        })
        .collect::<std::collections::HashSet<_>>();
    if footprint.len() < 4 || footprint.first() != footprint.last() || unique_vertices.len() < 3 {
        return Err(RuleInputError::InvalidHazardGeometry);
    }
    let validation_polygon = Polygon::new(
        LineString::new(
            canonical_points(footprint)
                .into_iter()
                .map(|point| point.0)
                .collect(),
        ),
        vec![],
    );
    if !validation_polygon.is_valid() {
        return Err(RuleInputError::InvalidHazardGeometry);
    }
    if let Some(progress) = input.progress
        && progress.segment_index >= input.route.path.coordinates.len() - 1
    {
        return Err(RuleInputError::InvalidRouteProgress {
            segment_index: progress.segment_index,
        });
    }
    validate_band(input.route_altitude_band, "route")?;
    validate_band(input.hazard.altitude_band.as_ref(), "hazard")?;
    Ok(())
}

fn temporal_relation(input: &RouteHazardInput<'_>) -> TemporalRelation {
    let route = if input.evaluated_at < input.route.effective_from {
        RouteTemporalState::NotYetEffective
    } else if input
        .route
        .effective_to
        .is_some_and(|end| input.evaluated_at >= end)
    {
        RouteTemporalState::Expired
    } else {
        RouteTemporalState::Active
    };
    let hazard = if input.hazard.status == WeatherHazardStatus::Cancelled {
        HazardTemporalState::Cancelled
    } else if input.evaluated_at < input.hazard.valid_from {
        HazardTemporalState::NotYetValid
    } else if input.evaluated_at > input.hazard.valid_to {
        HazardTemporalState::Expired
    } else {
        HazardTemporalState::Active
    };
    TemporalRelation { route, hazard }
}

fn altitude_relation(
    route: Option<&AltitudeBand>,
    hazard: Option<&AltitudeBand>,
) -> Result<AltitudeRelation, RuleInputError> {
    let Some(hazard) = hazard else {
        return Ok(AltitudeRelation::Overlap);
    };
    let Some(route) = route else {
        return Ok(AltitudeRelation::Indeterminate);
    };
    let route = normalize_band(route, "route")?;
    let hazard = normalize_band(hazard, "hazard")?;
    if route.frame != hazard.frame {
        return Ok(AltitudeRelation::Indeterminate);
    }
    Ok(
        if route.lower_feet <= hazard.upper_feet && hazard.lower_feet <= route.upper_feet {
            AltitudeRelation::Overlap
        } else {
            AltitudeRelation::Disjoint
        },
    )
}

fn validate_band(band: Option<&AltitudeBand>, subject: &'static str) -> Result<(), RuleInputError> {
    if let Some(band) = band {
        normalize_band(band, subject)?;
    }
    Ok(())
}

fn normalize_band(
    band: &AltitudeBand,
    subject: &'static str,
) -> Result<NormalizedAltitudeBand, RuleInputError> {
    let frame = band
        .lower
        .or(band.upper)
        .map(|altitude| altitude_frame(altitude.reference))
        .unwrap_or(AltitudeFrame::PressureOrMeanSeaLevel);
    if band
        .lower
        .into_iter()
        .chain(band.upper)
        .any(|altitude| altitude_frame(altitude.reference) != frame)
    {
        return Err(RuleInputError::MixedAltitudeReferences { subject });
    }
    let lower_feet = band.lower.map(altitude_feet).unwrap_or(f64::NEG_INFINITY);
    let upper_feet = band.upper.map(altitude_feet).unwrap_or(f64::INFINITY);
    if upper_feet < lower_feet {
        return Err(RuleInputError::InvertedAltitudeBand { subject });
    }
    Ok(NormalizedAltitudeBand {
        lower_feet,
        upper_feet,
        frame,
    })
}

fn altitude_frame(reference: AltitudeReference) -> AltitudeFrame {
    match reference {
        AltitudeReference::MeanSeaLevel | AltitudeReference::FlightLevel => {
            AltitudeFrame::PressureOrMeanSeaLevel
        }
        AltitudeReference::AboveGroundLevel => AltitudeFrame::AboveGroundLevel,
        AltitudeReference::Ellipsoid => AltitudeFrame::Ellipsoid,
    }
}

fn altitude_feet(altitude: Altitude) -> f64 {
    match altitude.unit {
        AltitudeUnit::Feet => f64::from(altitude.value),
        AltitudeUnit::Meters => f64::from(altitude.value) * FEET_PER_METER,
    }
}

fn canonical_points(points: &[crate::domain::GeoPoint]) -> Vec<Point<f64>> {
    points
        .iter()
        .map(|point| {
            let [longitude, latitude] = point.as_geojson_position();
            Point::new(longitude, latitude)
        })
        .collect()
}

fn remaining_route(
    route: &[Point<f64>],
    progress: Option<RouteProgress>,
) -> Result<(Vec<Point<f64>>, f64), RuleInputError> {
    let Some(progress) = progress else {
        return Ok((route.to_vec(), 0.0));
    };
    let Some(segment) = route.get(progress.segment_index..=progress.segment_index + 1) else {
        return Err(RuleInputError::InvalidRouteProgress {
            segment_index: progress.segment_index,
        });
    };
    let start = Haversine.point_at_ratio_between(segment[0], segment[1], progress.segment_fraction);
    let mut remaining = Vec::with_capacity(route.len() - progress.segment_index);
    remaining.push(start);
    remaining.extend_from_slice(&route[progress.segment_index + 1..]);
    let progress_distance_nm = path_length_nm(&route[..=progress.segment_index])
        + meters_to_nm(Haversine.distance(segment[0], start));
    Ok((remaining, progress_distance_nm))
}

fn assess_geometry(
    route: &[Point<f64>],
    hazard: &[Point<f64>],
    resolution_nm: f64,
) -> GeometryAssessment {
    let route_dense = densify(route, resolution_nm);
    let hazard_dense = densify(hazard, resolution_nm);
    if let Some(distance_nm) = first_intersection_distance_nm(&route_dense, &hazard_dense) {
        return GeometryAssessment {
            intersects: true,
            closest_approach_nm: 0.0,
            closest_route_distance_nm: distance_nm,
        };
    }

    let mut best_distance_nm = f64::INFINITY;
    let mut best_route_distance_nm = 0.0;
    let route_distances = cumulative_distances(route);
    let hazard_vertices = &hazard[..hazard.len().saturating_sub(1)];

    for (segment_index, segment) in route.windows(2).enumerate() {
        let line = Line::new(segment[0], segment[1]);
        for vertex in hazard_vertices {
            if let Some(closest) = closest_point(&line, *vertex) {
                let distance_nm = meters_to_nm(Haversine.distance(*vertex, closest));
                if distance_nm < best_distance_nm {
                    best_distance_nm = distance_nm;
                    best_route_distance_nm = route_distances[segment_index]
                        + meters_to_nm(Haversine.distance(segment[0], closest));
                }
            }
        }
    }

    let hazard_edges: Vec<_> = hazard
        .windows(2)
        .map(|edge| Line::new(edge[0], edge[1]))
        .collect();
    for (route_index, route_point) in route.iter().enumerate() {
        for edge in &hazard_edges {
            if let Some(closest) = closest_point(edge, *route_point) {
                let distance_nm = meters_to_nm(Haversine.distance(*route_point, closest));
                if distance_nm < best_distance_nm {
                    best_distance_nm = distance_nm;
                    best_route_distance_nm = route_distances[route_index];
                }
            }
        }
    }

    GeometryAssessment {
        intersects: false,
        closest_approach_nm: best_distance_nm,
        closest_route_distance_nm: best_route_distance_nm,
    }
}

fn closest_point(line: &Line<f64>, point: Point<f64>) -> Option<Point<f64>> {
    match line.haversine_closest_point(&point) {
        Closest::Intersection(point) | Closest::SinglePoint(point) => Some(point),
        Closest::Indeterminate => None,
    }
}

fn densify(points: &[Point<f64>], resolution_nm: f64) -> Vec<Point<f64>> {
    let mut result = Vec::new();
    for segment in points.windows(2) {
        let generated: Vec<_> = Haversine
            .points_along_line(
                segment[0],
                segment[1],
                resolution_nm * METERS_PER_NAUTICAL_MILE,
                true,
            )
            .collect();
        if result.is_empty() {
            result.extend(generated);
        } else {
            result.extend(generated.into_iter().skip(1));
        }
    }
    result
}

fn first_intersection_distance_nm(route: &[Point<f64>], hazard: &[Point<f64>]) -> Option<f64> {
    let anchor = hazard.first()?.x();
    let route_unwrapped = unwrap_points(route, anchor);
    let polygon = Polygon::new(LineString::new(unwrap_points(hazard, anchor)), vec![]);
    let mut distance_nm = 0.0;
    if Point::from(route_unwrapped[0]).intersects(&polygon) {
        return Some(0.0);
    }
    for (index, segment) in route_unwrapped.windows(2).enumerate() {
        if Line::new(segment[0], segment[1]).intersects(&polygon) {
            return Some(distance_nm);
        }
        distance_nm += meters_to_nm(Haversine.distance(route[index], route[index + 1]));
    }
    None
}

fn unwrap_points(points: &[Point<f64>], anchor: f64) -> Vec<geo::Coord<f64>> {
    let mut previous = anchor;
    points
        .iter()
        .map(|point| {
            let longitude = point.x() + 360.0 * ((previous - point.x()) / 360.0).round();
            previous = longitude;
            geo::Coord {
                x: longitude,
                y: point.y(),
            }
        })
        .collect()
}

fn cumulative_distances(points: &[Point<f64>]) -> Vec<f64> {
    let mut total = 0.0;
    let mut values = Vec::with_capacity(points.len());
    values.push(0.0);
    for segment in points.windows(2) {
        total += meters_to_nm(Haversine.distance(segment[0], segment[1]));
        values.push(total);
    }
    values
}

fn path_length_nm(points: &[Point<f64>]) -> f64 {
    points
        .windows(2)
        .map(|segment| meters_to_nm(Haversine.distance(segment[0], segment[1])))
        .sum()
}

fn meters_to_nm(meters: f64) -> f64 {
    meters / METERS_PER_NAUTICAL_MILE
}

fn quantize_nm(value: f64) -> f64 {
    let rounded = (value * 1_000_000.0).round() / 1_000_000.0;
    if rounded == -0.0 { 0.0 } else { rounded }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn great_circle_densification_curves_toward_the_pole() {
        let route = [Point::new(-45.0, 70.0), Point::new(45.0, 70.0)];
        let densified = densify(&route, 250.0);

        assert!(densified.len() > 2);
        assert!(
            densified.iter().any(|point| point.y() > 75.0),
            "great-circle interpolation must not be linear in longitude/latitude"
        );
        assert_eq!(densified.first(), Some(&route[0]));
        assert_eq!(densified.last(), Some(&route[1]));
    }

    #[test]
    fn longitude_latitude_order_and_antimeridian_unwrapping_are_preserved() {
        let route = [Point::new(178.0, 0.0), Point::new(-178.0, 0.0)];
        let hazard = [
            Point::new(179.0, -1.0),
            Point::new(-179.0, -1.0),
            Point::new(-179.0, 1.0),
            Point::new(179.0, 1.0),
            Point::new(179.0, -1.0),
        ];

        let assessment = assess_geometry(&route, &hazard, 5.0);

        assert!(assessment.intersects);
        assert_eq!(assessment.closest_approach_nm, 0.0);
    }

    #[test]
    fn configuration_rejects_unbounded_or_non_finite_work() {
        assert!(matches!(
            RouteHazardRuleConfig::new(-1.0, 1.0),
            Err(RuleConfigError::InvalidProximityMargin(_))
        ));
        assert!(matches!(
            RouteHazardRuleConfig::new(25.0, f64::NAN),
            Err(RuleConfigError::InvalidGeometryResolution(_))
        ));
        assert!(matches!(
            RouteHazardRule::new(RouteHazardRuleConfig {
                proximity_margin_nm: f64::INFINITY,
                geometry_resolution_nm: 1.0,
            }),
            Err(RuleConfigError::InvalidProximityMargin(_))
        ));
    }

    #[test]
    fn altitude_units_are_normalized_before_overlap() {
        let route = AltitudeBand {
            lower: Some(Altitude {
                value: 3_000,
                unit: AltitudeUnit::Meters,
                reference: AltitudeReference::MeanSeaLevel,
            }),
            upper: Some(Altitude {
                value: 4_000,
                unit: AltitudeUnit::Meters,
                reference: AltitudeReference::MeanSeaLevel,
            }),
        };
        let hazard = AltitudeBand {
            lower: Some(Altitude {
                value: 10_000,
                unit: AltitudeUnit::Feet,
                reference: AltitudeReference::FlightLevel,
            }),
            upper: Some(Altitude {
                value: 12_000,
                unit: AltitudeUnit::Feet,
                reference: AltitudeReference::FlightLevel,
            }),
        };

        assert_eq!(
            altitude_relation(Some(&route), Some(&hazard)).unwrap(),
            AltitudeRelation::Overlap
        );
    }

    #[test]
    fn incompatible_altitude_reference_frames_are_indeterminate() {
        let route = AltitudeBand {
            lower: Some(Altitude {
                value: 10_000,
                unit: AltitudeUnit::Feet,
                reference: AltitudeReference::AboveGroundLevel,
            }),
            upper: None,
        };
        let hazard = AltitudeBand {
            lower: Some(Altitude {
                value: 10_000,
                unit: AltitudeUnit::Feet,
                reference: AltitudeReference::FlightLevel,
            }),
            upper: None,
        };

        assert_eq!(
            altitude_relation(Some(&route), Some(&hazard)).unwrap(),
            AltitudeRelation::Indeterminate
        );
    }
}
