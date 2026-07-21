import "server-only";

import { AuthSessionError, authMode } from "./auth-server";

export type OperationsMode = "simulation" | "evaluation";

export type OperationalContext = {
  mode: OperationsMode;
  label: string;
  sourceScope: string;
};

export function getOperationalContext(): OperationalContext {
  const configured = process.env.OPERATIONS_MODE?.trim();
  const mode = configured || (authMode() === "development" ? "simulation" : "evaluation");
  if (mode === "simulation") {
    return {
      mode,
      label: "Simulation environment",
      sourceScope: "Replay and public-source evaluation",
    };
  }
  if (mode === "evaluation") {
    return {
      mode,
      label: "Evaluation environment",
      sourceScope: "Source-attributed evaluation data",
    };
  }
  throw new AuthSessionError("OPERATIONS_MODE must be simulation or evaluation", 500);
}
