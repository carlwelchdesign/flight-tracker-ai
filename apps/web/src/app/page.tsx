import { OperationsConsole } from "@/components/operations/operations-console";
import { getInitialFleet } from "@/lib/fleet-api";

export const dynamic = "force-dynamic";

export default async function Home() {
  const initialFleet = await getInitialFleet();
  return <OperationsConsole initialFleet={initialFleet} />;
}
