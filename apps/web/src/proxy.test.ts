import { beforeEach, describe, expect, it, vi } from "vitest";
import { NextRequest } from "next/server";

const clerk = vi.hoisted(() => ({
  protect: vi.fn(),
  publicPatterns: [] as string[],
}));

vi.mock("@clerk/nextjs/server", () => ({
  createRouteMatcher: (patterns: string[]) => {
    clerk.publicPatterns = patterns;
    return (request: NextRequest) =>
      patterns.some((pattern) =>
        pattern === "/"
          ? request.nextUrl.pathname === "/"
          : request.nextUrl.pathname.startsWith(pattern.replace("(.*)", "")),
      );
  },
  clerkMiddleware:
    (handler: (auth: { protect: () => Promise<void> }, request: NextRequest) => Promise<void>) =>
    (request: NextRequest) =>
      handler({ protect: clerk.protect }, request),
}));

vi.mock("@/lib/security-policy", () => ({ HOSTED_IDENTITY_CSP: {} }));

describe("hosted identity proxy", () => {
  beforeEach(() => {
    vi.stubEnv("AUTH_MODE", "clerk");
    clerk.protect.mockReset();
  });

  it("keeps the portfolio landing and sign-in routes public", async () => {
    const { proxy } = await import("./proxy");

    await proxy(new NextRequest("https://example.test/"), {} as never);
    await proxy(new NextRequest("https://example.test/sign-in"), {} as never);

    expect(clerk.publicPatterns).toEqual(["/", "/sign-in(.*)"]);
    expect(clerk.protect).not.toHaveBeenCalled();
  });

  it("protects operational and backend routes", async () => {
    const { proxy } = await import("./proxy");

    await proxy(new NextRequest("https://example.test/flights/FT-101"), {} as never);
    await proxy(new NextRequest("https://example.test/api/backend/health"), {} as never);

    expect(clerk.protect).toHaveBeenCalledTimes(2);
  });
});
