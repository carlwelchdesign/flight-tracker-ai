use std::{hint::black_box, time::Instant};

const EARTH_RADIUS_NM: f64 = 3_440.065;
const ITERATIONS: usize = 100_000;
const ROUTES: [[(f64, f64); 4]; 8] = [
    [
        (-121.72, 37.00),
        (-121.65, 37.25),
        (-121.95, 37.45),
        (-122.38, 37.62),
    ],
    [
        (-121.72, 37.00),
        (-121.30, 36.95),
        (-121.70, 37.60),
        (-122.38, 37.62),
    ],
    [
        (-121.72, 37.00),
        (-122.05, 37.00),
        (-122.15, 37.35),
        (-122.38, 37.62),
    ],
    [
        (-121.72, 37.00),
        (-121.20, 37.00),
        (-121.80, 37.70),
        (-122.38, 37.62),
    ],
    [
        (-121.72, 37.00),
        (-122.20, 36.95),
        (-122.30, 37.35),
        (-122.38, 37.62),
    ],
    [
        (-121.72, 37.00),
        (-121.10, 36.90),
        (-121.90, 37.75),
        (-122.38, 37.62),
    ],
    [
        (-121.72, 37.00),
        (-122.25, 37.05),
        (-122.35, 37.45),
        (-122.38, 37.62),
    ],
    [
        (-121.72, 37.00),
        (-121.00, 36.85),
        (-122.00, 37.80),
        (-122.38, 37.62),
    ],
];

fn main() {
    let started = Instant::now();
    let mut checksum = 0.0;
    for _ in 0..ITERATIONS {
        for route in black_box(ROUTES) {
            let length: f64 = route
                .windows(2)
                .map(|pair| haversine_nm(pair[0], pair[1]))
                .sum();
            let intersects = route.iter().any(|(longitude, latitude)| {
                (-121.9..=-121.4).contains(longitude) && (37.1..=37.5).contains(latitude)
            });
            checksum += length + if intersects { 50.0 } else { 0.0 };
        }
    }
    let elapsed = started.elapsed();
    let evaluations = ITERATIONS * ROUTES.len();
    println!("runtime=rust evaluations={evaluations} elapsed_ms={:.3} evaluations_per_second={:.0} checksum={:.3}", elapsed.as_secs_f64()*1000.0, evaluations as f64/elapsed.as_secs_f64(), checksum);
}

fn haversine_nm(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (a_lat, b_lat) = (a.1.to_radians(), b.1.to_radians());
    let d_lat = b_lat - a_lat;
    let d_lon = (b.0 - a.0).to_radians();
    let h = (d_lat / 2.0).sin().powi(2) + a_lat.cos() * b_lat.cos() * (d_lon / 2.0).sin().powi(2);
    EARTH_RADIUS_NM * 2.0 * h.sqrt().asin()
}
