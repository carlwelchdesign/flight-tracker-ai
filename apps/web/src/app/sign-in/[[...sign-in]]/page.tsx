import { authMode } from "@/lib/auth-server";
import Link from "next/link";

export const dynamic = "force-dynamic";

export default async function SignInPage() {
  if (authMode() === "development") {
    return (
      <main className="session-state">
        <p className="section-kicker">Development identity</p>
        <h1>Local sign in is automatic</h1>
        <p>Return to the console to use the configured development administrator.</p>
        <Link href="/">Open operations console</Link>
      </main>
    );
  }
  const { SignIn } = await import("@clerk/nextjs");
  return (
    <main className="session-state">
      <SignIn />
    </main>
  );
}
