import { OperationsConsole } from "@/components/operations/operations-console";
import { PortfolioOrientation } from "@/components/operations/portfolio-orientation";
import Link from "next/link";
import { getAuthContext } from "@/lib/auth-api";
import { AuthSessionError, authMode, createInternalAssertion } from "@/lib/auth-server";
import { getInitialFleet } from "@/lib/fleet-api";
import { getInitialWeather } from "@/lib/weather-api";
import { getInitialLivePositionStatus } from "@/lib/live-positions-api";

export const dynamic = "force-dynamic";

export default async function Home() {
  const result = await loadConsole();
  if (result.state === "ready") {
    return (
      <OperationsConsole
        orientation={<PortfolioOrientation />}
        authContext={result.authContext}
        initialFleet={result.initialFleet}
        initialWeather={result.initialWeather}
        initialLivePositions={result.initialLivePositions}
      />
    );
  }
  return (
    <main className="session-state">
      <p className="section-kicker">Flight Tracker AI</p>
      <h1>{result.signedOut ? "Sign in to continue" : "Console access unavailable"}</h1>
      <p>{result.message}</p>
      {authMode() === "clerk" && <Link href="/sign-in">Open secure sign in</Link>}
    </main>
  );
}

async function loadConsole() {
  try {
    const assertion = await createInternalAssertion();
    const [authContext, initialFleet, initialWeather, initialLivePositions] = await Promise.all([
      getAuthContext(),
      getInitialFleet(assertion),
      getInitialWeather(assertion),
      getInitialLivePositionStatus(assertion),
    ]);
    return {
      state: "ready" as const,
      authContext,
      initialFleet,
      initialWeather,
      initialLivePositions,
    };
  } catch (error) {
    const signedOut = error instanceof AuthSessionError && error.status === 401;
    return {
      state: "unavailable" as const,
      signedOut,
      message: signedOut
        ? "Your operations data remains protected until a valid session is available."
        : error instanceof Error
          ? error.message
          : "The authorization boundary could not be reached.",
    };
  }
}
