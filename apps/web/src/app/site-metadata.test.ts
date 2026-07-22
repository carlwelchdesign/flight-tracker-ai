import { describe, expect, it } from "vitest";
import {
  SITE_DESCRIPTION,
  SITE_TITLE,
  SITE_URL,
  SOCIAL_IMAGE_ALT,
  SOCIAL_IMAGE_PATH,
  siteMetadata,
} from "./site-metadata";

describe("site metadata", () => {
  it("publishes canonical and branded social-share contracts", () => {
    expect(siteMetadata).toMatchObject({
      metadataBase: SITE_URL,
      applicationName: "Flight Tracker AI",
      title: SITE_TITLE,
      description: SITE_DESCRIPTION,
      alternates: { canonical: "/" },
      openGraph: {
        type: "website",
        url: "/",
        siteName: "Flight Tracker AI",
        title: SITE_TITLE,
        description: SITE_DESCRIPTION,
        images: [
          {
            url: SOCIAL_IMAGE_PATH,
            width: 1200,
            height: 630,
            alt: SOCIAL_IMAGE_ALT,
          },
        ],
      },
      twitter: {
        card: "summary_large_image",
        title: SITE_TITLE,
        description: SITE_DESCRIPTION,
        images: [SOCIAL_IMAGE_PATH],
      },
      robots: { index: true, follow: true },
    });
  });

  it("keeps the preview accurate about portfolio and implemented capabilities", () => {
    expect(SITE_DESCRIPTION).toContain("live regional aircraft");
    expect(SITE_DESCRIPTION).toContain("aviation weather");
    expect(SITE_DESCRIPTION).toContain("deterministic replay");
    expect(SITE_DESCRIPTION).toContain("explainable attention");
    expect(SITE_DESCRIPTION).toContain("non-operational portfolio demo");
  });
});
