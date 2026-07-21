import type { NextRequest } from "next/server";
import { getApiBaseUrl } from "@/lib/fleet-api";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

const ALLOWED_PATHS = [
  /^api\/flights(?:\/[^/]+(?:\/timeline)?)?$/,
  /^api\/events\/stream$/,
  /^api\/dev\/replay(?:\/(?:pause|resume|reset|speed))?$/,
];

type RouteContext = {
  params: Promise<{ path: string[] }>;
};

export async function GET(request: NextRequest, context: RouteContext) {
  return forward(request, context, "GET");
}

export async function POST(request: NextRequest, context: RouteContext) {
  return forward(request, context, "POST");
}

async function forward(
  request: NextRequest,
  context: RouteContext,
  method: "GET" | "POST",
): Promise<Response> {
  const { path } = await context.params;
  const backendPath = path.join("/");
  if (!ALLOWED_PATHS.some((pattern) => pattern.test(backendPath))) {
    return Response.json(
      { error: { code: "proxy_path_not_allowed", message: "Backend path is not allowed" } },
      { status: 404 },
    );
  }

  const target = new URL(`${getApiBaseUrl()}/${backendPath}`);
  target.search = request.nextUrl.search;
  const headers = new Headers();
  for (const name of ["accept", "content-type", "last-event-id"]) {
    const value = request.headers.get(name);
    if (value) headers.set(name, value);
  }

  try {
    const response = await fetch(target, {
      method,
      headers,
      body: method === "POST" ? await request.text() : undefined,
      cache: "no-store",
      signal: request.signal,
    });
    const responseHeaders = new Headers();
    for (const name of ["content-type", "cache-control"]) {
      const value = response.headers.get(name);
      if (value) responseHeaders.set(name, value);
    }
    if (backendPath === "api/events/stream") {
      responseHeaders.set("cache-control", "no-cache, no-transform");
      responseHeaders.set("x-accel-buffering", "no");
    }
    return new Response(response.body, {
      status: response.status,
      headers: responseHeaders,
    });
  } catch {
    return Response.json(
      { error: { code: "backend_unavailable", message: "Operations API is unavailable" } },
      { status: 503 },
    );
  }
}
