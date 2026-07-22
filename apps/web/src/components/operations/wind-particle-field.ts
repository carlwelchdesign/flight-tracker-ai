import type { PublicWindSample } from "@/lib/public-atmosphere";

export type WindVector = { east: number; north: number };

export function windVector(sample: PublicWindSample): WindVector {
  const radians = sample.direction_from_degrees * Math.PI / 180;
  return {
    east: -sample.speed_knots * Math.sin(radians),
    north: -sample.speed_knots * Math.cos(radians),
  };
}

export function nearestWindSample(
  samples: readonly PublicWindSample[],
  longitude: number,
  latitude: number,
): PublicWindSample {
  if (samples.length === 0) throw new Error("Wind field requires at least one sample");
  let nearest = samples[0];
  let nearestDistance = Number.POSITIVE_INFINITY;
  for (const sample of samples) {
    const latitudeScale = Math.cos(latitude * Math.PI / 180);
    const dx = (sample.longitude_degrees - longitude) * latitudeScale;
    const dy = sample.latitude_degrees - latitude;
    const distance = dx * dx + dy * dy;
    if (distance < nearestDistance) {
      nearest = sample;
      nearestDistance = distance;
    }
  }
  return nearest;
}
