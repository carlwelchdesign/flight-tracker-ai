#!/usr/bin/env python3
import math
import time

EARTH_RADIUS_NM = 3440.065
ITERATIONS = 100_000
ROUTES = (
    ((-121.72,37.00),(-121.65,37.25),(-121.95,37.45),(-122.38,37.62)),
    ((-121.72,37.00),(-121.30,36.95),(-121.70,37.60),(-122.38,37.62)),
    ((-121.72,37.00),(-122.05,37.00),(-122.15,37.35),(-122.38,37.62)),
    ((-121.72,37.00),(-121.20,37.00),(-121.80,37.70),(-122.38,37.62)),
    ((-121.72,37.00),(-122.20,36.95),(-122.30,37.35),(-122.38,37.62)),
    ((-121.72,37.00),(-121.10,36.90),(-121.90,37.75),(-122.38,37.62)),
    ((-121.72,37.00),(-122.25,37.05),(-122.35,37.45),(-122.38,37.62)),
    ((-121.72,37.00),(-121.00,36.85),(-122.00,37.80),(-122.38,37.62)),
)

def haversine_nm(a, b):
    a_lat, b_lat = math.radians(a[1]), math.radians(b[1])
    d_lat, d_lon = b_lat - a_lat, math.radians(b[0] - a[0])
    h = math.sin(d_lat/2)**2 + math.cos(a_lat)*math.cos(b_lat)*math.sin(d_lon/2)**2
    return EARTH_RADIUS_NM * 2 * math.asin(math.sqrt(h))

started = time.perf_counter()
checksum = 0.0
for _ in range(ITERATIONS):
    for route in ROUTES:
        length = sum(haversine_nm(a, b) for a, b in zip(route, route[1:]))
        intersects = any(-121.9 <= longitude <= -121.4 and 37.1 <= latitude <= 37.5 for longitude, latitude in route)
        checksum += length + (50.0 if intersects else 0.0)
elapsed = time.perf_counter() - started
evaluations = ITERATIONS * len(ROUTES)
print(f"runtime=python evaluations={evaluations} elapsed_ms={elapsed*1000:.3f} evaluations_per_second={evaluations/elapsed:.0f} checksum={checksum:.3f}")
