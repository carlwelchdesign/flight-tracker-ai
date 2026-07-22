import { getApiBaseUrl } from "@/lib/fleet-api";
import { findPublicLiveRegion } from "@/lib/public-live-regions";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

export async function GET(request: Request): Promise<Response> {
  const requestedRegion = new URL(request.url).searchParams.get("region") ?? "sfo";
  const region = findPublicLiveRegion(requestedRegion);
  if (!region) {
    return Response.json(
      { error: { code: "live_region_not_found", message: "The requested live traffic region is not available" } },
      { status: 404, headers: { "cache-control": "no-store" } },
    );
  }
  try {
    const response = await fetch(
      `${getApiBaseUrl()}/api/public/live-positions?region=${encodeURIComponent(region.code)}`,
      {
      cache: "no-store",
      signal: AbortSignal.timeout(8_000),
      },
    );
    return new Response(response.body, {
      status: response.status,
      headers: {
        "cache-control": "no-store",
        "content-type": response.headers.get("content-type") ?? "application/json",
      },
    });
  } catch {
    return Response.json(
      { error: { code: "live_positions_unavailable", message: "Live positions are unavailable" } },
      { status: 503, headers: { "cache-control": "no-store" } },
    );
  }
}
