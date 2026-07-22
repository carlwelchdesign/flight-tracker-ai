import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "maplibre-gl/dist/maplibre-gl.css";
import "./globals.css";
import { authMode } from "@/lib/auth-server";
import { HOSTED_CLERK_PROVIDER_OPTIONS } from "@/lib/security-policy";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Flight Tracker AI · Portfolio Demonstration",
  description:
    "A non-commercial portfolio demonstration of source-attributed airline operations monitoring.",
};

export default async function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const document = (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="min-h-full">
        {children}
      </body>
    </html>
  );
  if (authMode() === "development") return document;
  const { ClerkProvider } = await import("@clerk/nextjs");
  return (
    <ClerkProvider {...HOSTED_CLERK_PROVIDER_OPTIONS}>{document}</ClerkProvider>
  );
}
