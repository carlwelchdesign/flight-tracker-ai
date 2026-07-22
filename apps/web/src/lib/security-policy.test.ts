import { describe, expect, it } from "vitest";
import nextConfig from "../../next.config";
import {
  BROWSER_SECURITY_HEADERS,
  HOSTED_CLERK_PROVIDER_OPTIONS,
  HOSTED_IDENTITY_CSP,
} from "./security-policy";

describe("browser security policy", () => {
  it("applies the approved response headers to every application route", async () => {
    const rules = await nextConfig.headers?.();

    expect(rules).toEqual([
      {
        source: "/:path*",
        headers: BROWSER_SECURITY_HEADERS,
      },
    ]);
    expect(nextConfig.poweredByHeader).toBe(false);
    expect(Object.fromEntries(BROWSER_SECURITY_HEADERS.map(({ key, value }) => [key, value])))
      .toMatchObject({
        "Cross-Origin-Opener-Policy": "same-origin-allow-popups",
        "Cross-Origin-Resource-Policy": "same-origin",
        "Referrer-Policy": "strict-origin-when-cross-origin",
        "Strict-Transport-Security": "max-age=31536000",
        "X-Content-Type-Options": "nosniff",
        "X-Frame-Options": "DENY",
        "X-Permitted-Cross-Domain-Policies": "none",
        "Permissions-Policy":
          "camera=(), geolocation=(), microphone=(), payment=(), usb=()",
      });
  });

  it("uses Clerk's nonce-aware strict CSP with restrictive application directives", () => {
    expect(HOSTED_IDENTITY_CSP).toEqual({
      strict: true,
      directives: {
        "base-uri": ["self"],
        "connect-src": ["self", "https://tiles.openfreemap.org", "https://nowcoast.noaa.gov"],
        "font-src": ["self", "data:"],
        "frame-ancestors": ["none"],
        "img-src": ["self", "blob:", "data:", "https://tiles.openfreemap.org", "https://nowcoast.noaa.gov"],
        "media-src": ["none"],
        "object-src": ["none"],
        "worker-src": ["self", "blob:"],
      },
    });
  });

  it("opts Clerk into dynamic rendering so its browser script receives the CSP nonce", () => {
    expect(HOSTED_CLERK_PROVIDER_OPTIONS).toEqual({
      dynamic: true,
      signInUrl: "/sign-in",
      signUpUrl: "/sign-up",
    });
  });
});
