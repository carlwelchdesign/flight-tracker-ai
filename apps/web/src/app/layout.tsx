import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { OperationsTrustBanner } from "@/components/operations/operations-trust-banner";
import { authMode } from "@/lib/auth-server";
import { getOperationalContext } from "@/lib/operational-context";

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
  const operationalContext = getOperationalContext();
  const document = (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="min-h-full">
        <OperationsTrustBanner context={operationalContext} />
        {children}
      </body>
    </html>
  );
  if (authMode() === "development") return document;
  const { ClerkProvider } = await import("@clerk/nextjs");
  return <ClerkProvider>{document}</ClerkProvider>;
}
