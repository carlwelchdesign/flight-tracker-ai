import { clerkMiddleware, createRouteMatcher } from "@clerk/nextjs/server";
import { NextResponse } from "next/server";
import type { NextFetchEvent, NextRequest } from "next/server";
import { HOSTED_IDENTITY_CSP } from "@/lib/security-policy";

const isPublicRoute = createRouteMatcher([
  "/",
  "/api/public/live-positions",
  "/api/public/atmosphere/wind",
  "/api/public/replay/attention",
  "/api/public/weather",
  "/sign-in(.*)",
  "/sign-up(.*)",
]);
const hostedIdentityProxy = clerkMiddleware(
  async (auth, request) => {
    if (!isPublicRoute(request)) await auth.protect();
  },
  { contentSecurityPolicy: HOSTED_IDENTITY_CSP },
);

export function proxy(request: NextRequest, event: NextFetchEvent) {
  if ((process.env.AUTH_MODE ?? "development") === "development") {
    return NextResponse.next();
  }
  return hostedIdentityProxy(request, event);
}

export const config = {
  matcher: [
    "/((?!_next/static|_next/image|favicon.ico|.*\\.(?:svg|png|jpg|jpeg|gif|webp)$).*)",
  ],
};
