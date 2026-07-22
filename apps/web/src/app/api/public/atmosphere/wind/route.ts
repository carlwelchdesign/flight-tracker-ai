import { getApiBaseUrl } from "@/lib/fleet-api";
import { isWindLevelCode } from "@/lib/public-atmosphere";
import { findPublicLiveRegion } from "@/lib/public-live-regions";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

export async function GET(request: Request): Promise<Response> {
  const query = new URL(request.url).searchParams;
  const region = findPublicLiveRegion(query.get("region") ?? "sfo");
  const level = query.get("level") ?? "surface";
  if (!region || !isWindLevelCode(level)) {
    return Response.json(
      { error: { code: "atmosphere_selection_not_found", message: "The requested atmospheric selection is unavailable" } },
      { status: 404, headers: { "cache-control": "no-store" } },
    );
  }
  try {
    const response = await fetch(
      `${getApiBaseUrl()}/api/public/atmosphere/wind?region=${region.code}&level=${level}`,
      { cache: "no-store", signal: AbortSignal.timeout(12_000) },
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
      { error: { code: "atmosphere_unavailable", message: "Atmospheric model wind is unavailable" } },
      { status: 503, headers: { "cache-control": "no-store" } },
    );
  }
}
