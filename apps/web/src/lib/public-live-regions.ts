export type PublicLiveRegion = {
  code: "sfo" | "lax" | "sea" | "den" | "ord" | "atl" | "jfk";
  airport: string;
  name: string;
  center: readonly [longitude: number, latitude: number];
};

export const PUBLIC_LIVE_REGIONS: readonly PublicLiveRegion[] = [
  { code: "sfo", airport: "SFO", name: "San Francisco", center: [-122.379, 37.6213] },
  { code: "lax", airport: "LAX", name: "Los Angeles", center: [-118.4085, 33.9416] },
  { code: "sea", airport: "SEA", name: "Seattle", center: [-122.3088, 47.4502] },
  { code: "den", airport: "DEN", name: "Denver", center: [-104.6737, 39.8561] },
  { code: "ord", airport: "ORD", name: "Chicago", center: [-87.9073, 41.9742] },
  { code: "atl", airport: "ATL", name: "Atlanta", center: [-84.4277, 33.6407] },
  { code: "jfk", airport: "JFK", name: "New York", center: [-73.7781, 40.6413] },
] as const;

export const DEFAULT_PUBLIC_LIVE_REGION = PUBLIC_LIVE_REGIONS[0];

export function findPublicLiveRegion(code: string | null | undefined): PublicLiveRegion | null {
  if (!code) return null;
  return PUBLIC_LIVE_REGIONS.find((region) => region.code === code.toLowerCase()) ?? null;
}
