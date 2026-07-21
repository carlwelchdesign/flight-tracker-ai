import { clerkMiddleware, createRouteMatcher } from "@clerk/nextjs/server";
import { NextResponse } from "next/server";
import type { NextFetchEvent, NextRequest } from "next/server";

const isPublicRoute = createRouteMatcher(["/sign-in(.*)"]);
const hostedIdentityProxy = clerkMiddleware(async (auth, request) => {
  if (!isPublicRoute(request)) await auth.protect();
});

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
