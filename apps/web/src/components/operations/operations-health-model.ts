import type { WorkerHealth } from "@/lib/backend-health";

export type ConnectionState = "connecting" | "live" | "reconnecting" | "disconnected";

export type ServiceHealthState =
  | { state: "checking"; workers: WorkerHealth[] }
  | { state: "healthy"; workers: WorkerHealth[] }
  | { state: "degraded"; workers: WorkerHealth[]; message: string };
