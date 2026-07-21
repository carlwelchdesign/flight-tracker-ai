import { describe, expect, it } from "vitest";
import { parseAuthContext } from "./auth-model";

const context = {
  identity_id: "identity-1",
  operator_id: "operator-1",
  operator_code: "SIM",
  operator_name: "Simulation Operator",
  provider: "development",
  subject: "local-admin",
  session_id: "session-1",
  role: "administrator",
};

describe("authorization context", () => {
  it("accepts explicit supported roles", () => {
    expect(parseAuthContext(context)).toEqual(context);
  });

  it("rejects unknown roles and incomplete identity data", () => {
    expect(() => parseAuthContext({ ...context, role: "owner" })).toThrow(/invalid role/i);
    expect(() => parseAuthContext({ ...context, operator_id: undefined })).toThrow(
      /invalid context/i,
    );
  });
});
