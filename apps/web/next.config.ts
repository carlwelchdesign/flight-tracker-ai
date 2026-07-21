import type { NextConfig } from "next";
import { BROWSER_SECURITY_HEADERS } from "./src/lib/security-policy";

const nextConfig: NextConfig = {
  output: "standalone",
  poweredByHeader: false,
  async headers() {
    return [
      {
        source: "/:path*",
        headers: BROWSER_SECURITY_HEADERS,
      },
    ];
  },
};

export default nextConfig;
