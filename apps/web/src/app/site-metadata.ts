import type { Metadata } from "next";

export const SITE_URL = new URL("https://flight-tracker-ai-one.vercel.app");
export const SITE_NAME = "Flight Tracker AI";
export const SITE_TITLE = "Flight Tracker AI | Live Aviation Intelligence";
export const SITE_DESCRIPTION =
  "Explore live regional aircraft, trajectories, aviation weather, deterministic replay, and explainable attention in a non-operational portfolio demo.";
export const SOCIAL_IMAGE_PATH = "/opengraph-image";
export const SOCIAL_IMAGE_ALT =
  "Flight Tracker AI showing a live aviation map, aircraft trajectories, weather, and explainable flight attention";

export const siteMetadata: Metadata = {
  metadataBase: SITE_URL,
  applicationName: SITE_NAME,
  title: SITE_TITLE,
  description: SITE_DESCRIPTION,
  alternates: {
    canonical: "/",
  },
  openGraph: {
    type: "website",
    url: "/",
    siteName: SITE_NAME,
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
  robots: {
    index: true,
    follow: true,
  },
};
