import { OperationsConsole } from "@/components/operations/operations-console";
import { PortfolioOrientation } from "@/components/operations/portfolio-orientation";
import { PublicFlightTrackerDemo } from "@/components/operations/public-flight-tracker-demo";
import Link from "next/link";
import { getAuthContext } from "@/lib/auth-api";
import { AuthSessionError, authMode, createInternalAssertion } from "@/lib/auth-server";
import { getInitialFleet } from "@/lib/fleet-api";
import { getInitialWeather } from "@/lib/weather-api";
import { getInitialLivePositionStatus } from "@/lib/live-positions-api";
import { describeConsoleFailure } from "@/lib/console-availability";

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
  if (result.signedOut) return <PublicFlightTrackerDemo />;
  return (
    <main className="session-state">
      <p className="section-kicker">Flight Tracker AI</p>
      <h1>Console access unavailable</h1>
      <p>{result.message}</p>
      {authMode() === "clerk" && <Link href="/sign-in">Open secure sign in</Link>}
      {!result.signedOut && <Link href="/?retry=1" prefetch={false}>Try again</Link>}
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
    const failure = describeConsoleFailure(
      error instanceof AuthSessionError
        ? { status: error.status, message: error.message }
        : null,
      process.env.OPERATIONS_MODE,
    );
    return {
      state: "unavailable" as const,
      ...failure,
    };
  }
}
