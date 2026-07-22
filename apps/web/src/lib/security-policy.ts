import type { ClerkMiddlewareOptions } from "@clerk/nextjs/server";

export const BROWSER_SECURITY_HEADERS = [
  { key: "Cross-Origin-Opener-Policy", value: "same-origin-allow-popups" },
  { key: "Cross-Origin-Resource-Policy", value: "same-origin" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "Strict-Transport-Security", value: "max-age=31536000" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Permitted-Cross-Domain-Policies", value: "none" },
  {
    key: "Permissions-Policy",
    value: "camera=(), geolocation=(), microphone=(), payment=(), usb=()",
  },
];

export const HOSTED_IDENTITY_CSP: NonNullable<
  ClerkMiddlewareOptions["contentSecurityPolicy"]
> = {
  strict: true,
  directives: {
    "base-uri": ["self"],
    "font-src": ["self", "data:"],
    "frame-ancestors": ["none"],
    "connect-src": ["self", "https://tiles.openfreemap.org", "https://nowcoast.noaa.gov"],
    "img-src": ["self", "blob:", "data:", "https://tiles.openfreemap.org", "https://nowcoast.noaa.gov"],
    "media-src": ["none"],
    "object-src": ["none"],
    "worker-src": ["self", "blob:"],
  },
};

export const HOSTED_CLERK_PROVIDER_OPTIONS = {
  dynamic: true,
  signInUrl: "/sign-in",
  signUpUrl: "/sign-up",
} as const;
