//! Spherical distance and compass directions; input angles are degrees.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelDir {
    Ahead,
    AheadRight,
    Right,
    BehindRight,
    Behind,
    BehindLeft,
    Left,
    AheadLeft,
}

/// Shortest surface distance on a sphere with the supplied radius in metres.
pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64, radius_m: f64) -> f64 {
    let latitude_delta = (lat2 - lat1).to_radians();
    let longitude_delta = (lon2 - lon1).to_radians();
    let haversine = (latitude_delta / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (longitude_delta / 2.0).sin().powi(2);
    // Round-off at antipodes can otherwise put the asin input just above one.
    2.0 * radius_m * haversine.clamp(0.0, 1.0).sqrt().asin()
}

/// Initial bearing clockwise from north, in [0, 360).
/// Coincident points return zero; a unique bearing does not exist at antipodes.
pub fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let latitude1 = lat1.to_radians();
    let latitude2 = lat2.to_radians();
    let longitude_delta = (lon2 - lon1).to_radians();
    let east = longitude_delta.sin() * latitude2.cos();
    let north = latitude1.cos() * latitude2.sin()
        - latitude1.sin() * latitude2.cos() * longitude_delta.cos();
    normalize(east.atan2(north).to_degrees())
}

/// Clockwise angle from the pilot's heading to a target bearing, in [0, 360).
pub fn relative_bearing(bearing: f64, heading: f64) -> f64 {
    normalize(bearing - heading)
}

/// Nearest 45-degree sector; exact halfway boundaries select clockwise.
pub fn rel_dir(rel: f64) -> RelDir {
    const DIRECTIONS: [RelDir; 8] = [
        RelDir::Ahead,
        RelDir::AheadRight,
        RelDir::Right,
        RelDir::BehindRight,
        RelDir::Behind,
        RelDir::BehindLeft,
        RelDir::Left,
        RelDir::AheadLeft,
    ];
    DIRECTIONS[sector(rel)]
}

/// Compass identifier for localization through `compass-<identifier>`.
pub fn compass(bearing: f64) -> &'static str {
    const POINTS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    POINTS[sector(bearing)]
}

fn normalize(angle: f64) -> f64 {
    // rem_euclid can round a very small negative value to exactly 360.
    angle.rem_euclid(360.0) % 360.0
}

fn sector(angle: f64) -> usize {
    ((normalize(angle) + 22.5) / 45.0) as usize % 8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn haversine_on_equator_matches_reference() {
        close(haversine_m(0.0, 0.0, 0.0, 0.1, 1_000_000.0), 1745.33, 1.0);
    }

    #[test]
    fn haversine_symmetric_and_zero_for_same_point() {
        let forward = haversine_m(27.3, -104.2, -33.7, 42.6, 1_000_000.0);
        let backward = haversine_m(-33.7, 42.6, 27.3, -104.2, 1_000_000.0);
        assert!(forward > 0.0);
        close(forward, backward, 1e-6);
        assert_eq!(haversine_m(27.3, -104.2, 27.3, -104.2, 1_000_000.0), 0.0);
    }

    #[test]
    fn bearing_east_and_north() {
        close(bearing_deg(0.0, 0.0, 0.0, 0.1), 90.0, 0.01);
        close(bearing_deg(0.0, 0.0, 0.1, 0.0), 0.0, 0.01);
    }

    #[test]
    fn relative_direction_behind() {
        assert_eq!(relative_bearing(90.0, 270.0), 180.0);
        assert_eq!(rel_dir(180.0), RelDir::Behind);
        assert_eq!(compass(225.0), "SW");
    }

    #[test]
    fn dateline_takes_short_path_in_both_directions() {
        close(
            haversine_m(0.0, 179.9, 0.0, -179.9, 1_000_000.0),
            3490.6585,
            0.001,
        );
        close(bearing_deg(0.0, 179.9, 0.0, -179.9), 90.0, 0.01);
        close(bearing_deg(0.0, -179.9, 0.0, 179.9), 270.0, 0.01);
    }

    #[test]
    fn antipodal_and_near_antipodal_distances_stay_finite() {
        let radius = 1_000_000.0;
        let half_circumference = std::f64::consts::PI * radius;
        for (lat, lon) in [(0.0, 0.0), (23.7, 91.2), (-89.99, -10.0)] {
            close(
                haversine_m(lat, lon, -lat, lon + 180.0, radius),
                half_circumference,
                0.1,
            );
            let near = haversine_m(lat, lon, -lat + 1e-8, lon + 180.0, radius);
            assert!(near.is_finite() && near <= half_circumference);
        }
    }

    #[test]
    fn eight_sectors_include_clockwise_half_angle_boundary() {
        let directions = [
            RelDir::Ahead,
            RelDir::AheadRight,
            RelDir::Right,
            RelDir::BehindRight,
            RelDir::Behind,
            RelDir::BehindLeft,
            RelDir::Left,
            RelDir::AheadLeft,
        ];
        let points = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
        for index in 0..8 {
            let center = index as f64 * 45.0;
            assert_eq!(rel_dir(center), directions[index]);
            assert_eq!(compass(center), points[index]);
            assert_eq!(rel_dir(center + 22.5 - 1e-6), directions[index]);
            assert_eq!(compass(center + 22.5), points[(index + 1) % 8]);
            assert_eq!(rel_dir(center + 22.5), directions[(index + 1) % 8]);
        }
    }

    #[test]
    fn bearings_wrap_into_zero_to_360() {
        assert_eq!(relative_bearing(10.0, 350.0), 20.0);
        assert_eq!(relative_bearing(350.0, 10.0), 340.0);
        assert_eq!(relative_bearing(-10.0, 710.0), 0.0);
        assert_eq!(rel_dir(-90.0), RelDir::Left);
        assert_eq!(compass(-45.0), "NW");
        assert_eq!(compass(720.0), "N");
        assert_eq!(bearing_deg(12.0, 23.0, 12.0, 23.0), 0.0);
    }
}
