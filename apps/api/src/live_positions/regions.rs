use uuid::Uuid;

use crate::domain::OperatorId;

use super::LivePositionRegion;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PublicLiveRegionDefinition {
    pub code: &'static str,
    pub name: &'static str,
    pub region: LivePositionRegion,
}

const PUBLIC_LIVE_REGION_DEFINITIONS: [PublicLiveRegionDefinition; 7] = [
    definition("sfo", "San Francisco", 37.6213, -122.379),
    definition("lax", "Los Angeles", 33.9416, -118.4085),
    definition("sea", "Seattle", 47.4502, -122.3088),
    definition("den", "Denver", 39.8561, -104.6737),
    definition("ord", "Chicago", 41.9742, -87.9073),
    definition("atl", "Atlanta", 33.6407, -84.4277),
    definition("jfk", "New York", 40.6413, -73.7781),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LivePositionRegionPreset {
    pub code: &'static str,
    pub name: &'static str,
    pub operator_id: OperatorId,
    pub region: LivePositionRegion,
    pub worker_name: &'static str,
}

pub fn public_live_region_catalog(
    primary_operator: OperatorId,
    primary_region: LivePositionRegion,
) -> Vec<LivePositionRegionPreset> {
    vec![
        preset(
            "sfo",
            "San Francisco",
            primary_operator,
            primary_region,
            "adsb_lol_sfo",
        ),
        derived_preset(
            primary_operator,
            "lax",
            "Los Angeles",
            33.9416,
            -118.4085,
            "adsb_lol_lax",
        ),
        derived_preset(
            primary_operator,
            "sea",
            "Seattle",
            47.4502,
            -122.3088,
            "adsb_lol_sea",
        ),
        derived_preset(
            primary_operator,
            "den",
            "Denver",
            39.8561,
            -104.6737,
            "adsb_lol_den",
        ),
        derived_preset(
            primary_operator,
            "ord",
            "Chicago",
            41.9742,
            -87.9073,
            "adsb_lol_ord",
        ),
        derived_preset(
            primary_operator,
            "atl",
            "Atlanta",
            33.6407,
            -84.4277,
            "adsb_lol_atl",
        ),
        derived_preset(
            primary_operator,
            "jfk",
            "New York",
            40.6413,
            -73.7781,
            "adsb_lol_jfk",
        ),
    ]
}

pub fn find_public_live_region(
    primary_operator: OperatorId,
    primary_region: LivePositionRegion,
    code: &str,
) -> Option<LivePositionRegionPreset> {
    public_live_region_catalog(primary_operator, primary_region)
        .into_iter()
        .find(|preset| preset.code == code)
}

pub fn find_public_live_region_definition(code: &str) -> Option<PublicLiveRegionDefinition> {
    PUBLIC_LIVE_REGION_DEFINITIONS
        .iter()
        .copied()
        .find(|definition| definition.code == code)
}

fn derived_preset(
    primary_operator: OperatorId,
    code: &'static str,
    name: &'static str,
    latitude_degrees: f64,
    longitude_degrees: f64,
    worker_name: &'static str,
) -> LivePositionRegionPreset {
    let operator_id =
        OperatorId::from_uuid(Uuid::new_v5(&primary_operator.as_uuid(), code.as_bytes()));
    preset(
        code,
        name,
        operator_id,
        LivePositionRegion {
            latitude_degrees,
            longitude_degrees,
            radius_nautical_miles: 50,
        },
        worker_name,
    )
}

const fn definition(
    code: &'static str,
    name: &'static str,
    latitude_degrees: f64,
    longitude_degrees: f64,
) -> PublicLiveRegionDefinition {
    PublicLiveRegionDefinition {
        code,
        name,
        region: LivePositionRegion {
            latitude_degrees,
            longitude_degrees,
            radius_nautical_miles: 50,
        },
    }
}

fn preset(
    code: &'static str,
    name: &'static str,
    operator_id: OperatorId,
    region: LivePositionRegion,
    worker_name: &'static str,
) -> LivePositionRegionPreset {
    LivePositionRegionPreset {
        code,
        name,
        operator_id,
        region,
        worker_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_stable_bounded_and_uses_distinct_projection_keys() {
        let primary = OperatorId::new();
        let primary_region = LivePositionRegion {
            latitude_degrees: 37.6213,
            longitude_degrees: -122.379,
            radius_nautical_miles: 50,
        };
        let first = public_live_region_catalog(primary, primary_region);
        let second = public_live_region_catalog(primary, primary_region);

        assert_eq!(first, second);
        assert_eq!(first.len(), 7);
        assert_eq!(first[0].operator_id, primary);
        assert!(
            first
                .iter()
                .all(|preset| preset.region.radius_nautical_miles <= 50)
        );
        assert_eq!(
            first
                .iter()
                .map(|preset| preset.operator_id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            first.len()
        );
    }

    #[test]
    fn unknown_region_is_rejected() {
        assert!(
            find_public_live_region(
                OperatorId::new(),
                LivePositionRegion {
                    latitude_degrees: 37.6213,
                    longitude_degrees: -122.379,
                    radius_nautical_miles: 50,
                },
                "anywhere",
            )
            .is_none()
        );
        assert!(find_public_live_region_definition("anywhere").is_none());
        assert_eq!(
            find_public_live_region_definition("lax")
                .expect("LAX is allowlisted")
                .name,
            "Los Angeles"
        );
    }
}
