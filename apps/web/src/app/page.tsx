import { OperationsConsole } from "@/components/operations/operations-console";
import { getInitialFleet } from "@/lib/fleet-api";
import { getInitialWeather } from "@/lib/weather-api";

export const dynamic = "force-dynamic";

export default async function Home() {
  const [initialFleet, initialWeather] = await Promise.all([
    getInitialFleet(),
    getInitialWeather(),
  ]);
  return <OperationsConsole initialFleet={initialFleet} initialWeather={initialWeather} />;
}
