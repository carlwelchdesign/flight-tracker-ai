import { getApiBaseUrl } from "@/lib/fleet-api";
import { PUBLIC_LIVE_REGIONS } from "@/lib/public-live-regions";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

const AIRPORTS = new Set(PUBLIC_LIVE_REGIONS.map((region) => `K${region.airport}`));

export async function GET(request: Request): Promise<Response> {
  const airport = new URL(request.url).searchParams.get("airport")?.toUpperCase() ?? "KSFO";
  if (!AIRPORTS.has(airport)) return Response.json({ error: { code: "invalid_airport", message: "Choose an allowlisted airport" } }, { status: 400 });
  try {
    const response = await fetch(`${getApiBaseUrl()}/api/public/airport-intelligence?airport=${airport}`, { cache: "no-store", signal: AbortSignal.timeout(12_000) });
    return new Response(response.body, { status: response.status, headers: { "cache-control": response.headers.get("cache-control") ?? "no-store", "content-type": "application/json" } });
  } catch {
    return Response.json({ error: { code: "airport_intelligence_unavailable", message: "Airport forecast and pilot reports are unavailable" } }, { status: 503, headers: { "cache-control": "no-store" } });
  }
}
