import { afterEach, describe, expect, it, vi } from "vitest";
import { getOperationalContext } from "./operational-context";

describe("operational context", () => {
  afterEach(() => vi.unstubAllEnvs());

  it("defaults development to simulation and hosted identity to evaluation", () => {
    vi.stubEnv("OPERATIONS_MODE", "");
    vi.stubEnv("AUTH_MODE", "development");
    expect(getOperationalContext()).toMatchObject({ mode: "simulation" });

    vi.stubEnv("AUTH_MODE", "clerk");
    expect(getOperationalContext()).toMatchObject({ mode: "evaluation" });
  });

  it("rejects an unrecognized environment label", () => {
    vi.stubEnv("OPERATIONS_MODE", "production");
    expect(() => getOperationalContext()).toThrow(/must be simulation or evaluation/);
  });
});
