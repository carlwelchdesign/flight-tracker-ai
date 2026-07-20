use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct LongitudeDegrees(f64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct LatitudeDegrees(f64);

#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum CoordinateError {
    #[error("longitude must be finite and between -180 and 180 degrees, got {0}")]
    InvalidLongitude(f64),
    #[error("latitude must be finite and between -90 and 90 degrees, got {0}")]
    InvalidLatitude(f64),
}

impl TryFrom<f64> for LongitudeDegrees {
    type Error = CoordinateError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && (-180.0..=180.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(CoordinateError::InvalidLongitude(value))
        }
    }
}

impl From<LongitudeDegrees> for f64 {
    fn from(value: LongitudeDegrees) -> Self {
        value.0
    }
}

impl TryFrom<f64> for LatitudeDegrees {
    type Error = CoordinateError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && (-90.0..=90.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(CoordinateError::InvalidLatitude(value))
        }
    }
}

impl From<LatitudeDegrees> for f64 {
    fn from(value: LatitudeDegrees) -> Self {
        value.0
    }
}

/// A WGS84 coordinate. GeoJSON/PostGIS transport order is always longitude,
/// then latitude; named fields prevent callers from accidentally reversing it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub longitude_degrees: LongitudeDegrees,
    pub latitude_degrees: LatitudeDegrees,
}

impl GeoPoint {
    pub fn new(longitude_degrees: f64, latitude_degrees: f64) -> Result<Self, CoordinateError> {
        Ok(Self {
            longitude_degrees: longitude_degrees.try_into()?,
            latitude_degrees: latitude_degrees.try_into()?,
        })
    }

    pub fn as_geojson_position(self) -> [f64; 2] {
        [self.longitude_degrees.into(), self.latitude_degrees.into()]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoLineString {
    pub coordinates: Vec<GeoPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoPolygon {
    /// The first and last point must be equal when converted to PostGIS.
    pub exterior: Vec<GeoPoint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geojson_position_is_longitude_then_latitude() {
        let point = GeoPoint::new(-122.3656, 37.6196).unwrap();

        assert_eq!(point.as_geojson_position(), [-122.3656, 37.6196]);
    }

    #[test]
    fn coordinates_reject_invalid_or_non_finite_values_during_deserialization() {
        assert!(GeoPoint::new(181.0, 0.0).is_err());
        assert!(GeoPoint::new(0.0, -91.0).is_err());
        assert!(GeoPoint::new(f64::NAN, 0.0).is_err());
        assert!(serde_json::from_str::<LongitudeDegrees>("181.0").is_err());
    }
}
