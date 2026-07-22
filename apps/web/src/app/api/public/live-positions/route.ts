import { getApiBaseUrl } from "@/lib/fleet-api";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

export async function GET(): Promise<Response> {
  try {
    const response = await fetch(`${getApiBaseUrl()}/api/public/live-positions`, {
      cache: "no-store",
      signal: AbortSignal.timeout(8_000),
    });
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
