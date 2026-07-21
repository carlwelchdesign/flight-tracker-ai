import { describe, expect, it } from "vitest";
import { describeConsoleFailure } from "./console-availability";

describe("console availability copy", () => {
  it("keeps signed-out data protected", () => {
    expect(describeConsoleFailure({ status: 401, message: "raw" }, "evaluation")).toEqual({
      signedOut: true,
      message: "Your operations data remains protected until a valid session is available.",
    });
  });

  it("preserves actionable identity errors", () => {
    expect(
      describeConsoleFailure(
        { status: 403, message: "Select an organization before opening the console" },
        "evaluation",
      ),
    ).toEqual({
      signedOut: false,
      message: "Select an organization before opening the console",
    });
  });

  it("does not expose hosted configuration details", () => {
    expect(
      describeConsoleFailure(
        { status: 500, message: "INTERNAL_AUTH_SECRET must contain at least 32 bytes" },
        "evaluation",
      ),
    ).toEqual({
      signedOut: false,
      message:
        "The portfolio configuration is not ready yet. Access remains closed while setup is completed.",
    });
  });

  it("describes the portfolio cold start without leaking a backend error", () => {
    expect(describeConsoleFailure(null, "evaluation")).toEqual({
      signedOut: false,
      message:
        "The portfolio service may be waking from its idle state. Wait up to one minute, then try again; no operational data is implied.",
    });
  });
});
