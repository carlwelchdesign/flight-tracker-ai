import type { NextRequest } from "next/server";
import { getApiBaseUrl } from "@/lib/fleet-api";
import { AuthSessionError, createInternalAssertion } from "@/lib/auth-server";

export const dynamic = "force-dynamic";
export const runtime = "nodejs";

const ALLOWED_PATHS = [
  /^api\/system\/(?:health|readiness)$/,
  /^api\/source-health$/,
  /^api\/(?:hazards|airport-observations)$/,
  /^api\/source-records\/[^/]+$/,
  /^api\/flights(?:\/[^/]+(?:\/timeline)?)?$/,
  /^api\/alerts(?:\/[^/]+(?:\/actions)?)?$/,
  /^api\/events\/stream$/,
  /^api\/auth\/context$/,
  /^api\/admin\/(?:memberships(?:\/[^/]+)?|sessions\/revoke|audit-events(?:\/export)?|audit-alerts)$/,
  /^api\/dev\/replay(?:\/(?:pause|resume|reset|speed|outage))?$/,
  /^metrics$/,
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

export async function PATCH(request: NextRequest, context: RouteContext) {
  return forward(request, context, "PATCH");
}

async function forward(
  request: NextRequest,
  context: RouteContext,
  method: "GET" | "POST" | "PATCH",
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
  for (const name of ["accept", "content-type", "last-event-id", "x-correlation-id"]) {
    const value = request.headers.get(name);
    if (value) headers.set(name, value);
  }

  try {
    const assertion = await createInternalAssertion();
    headers.set("authorization", `Bearer ${assertion}`);
    const response = await fetch(target, {
      method,
      headers,
      body: method === "GET" ? undefined : await request.text(),
      cache: "no-store",
      signal: request.signal,
    });
    const responseHeaders = new Headers();
    for (const name of ["content-type", "content-disposition", "cache-control", "x-correlation-id"]) {
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
  } catch (error) {
    if (error instanceof AuthSessionError) {
      return Response.json(
        { error: { code: "session_unavailable", message: error.message } },
        { status: error.status },
      );
    }
    return Response.json(
      { error: { code: "backend_unavailable", message: "Operations API is unavailable" } },
      { status: 503 },
    );
  }
}
