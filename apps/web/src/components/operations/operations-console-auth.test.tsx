import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { OperationsConsole } from "./operations-console";

class TestEventSource {
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  addEventListener() {}
  removeEventListener() {}
  close() {}
}

afterEach(() => vi.unstubAllGlobals());

describe("operations session safety", () => {
  it("replaces the console with a non-data state when access is revoked", async () => {
    vi.stubGlobal("EventSource", TestEventSource);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        Response.json(
          { error: { code: "authorization_denied", message: "Access revoked" } },
          { status: 403 },
        )),
    );

    render(
      <OperationsConsole
        authContext={{
          identity_id: "identity-1",
          operator_id: "operator-1",
          operator_code: "SIM",
          operator_name: "Simulation Operator",
          provider: "development",
          subject: "local-admin",
          session_id: "session-1",
          role: "administrator",
        }}
        initialFleet={{
          state: "ready",
          page: {
            data: [],
            pagination: { page: 1, page_size: 100, total_items: 0, total_pages: 0 },
          },
        }}
        initialWeather={{ state: "unavailable", message: "No weather" }}
        initialLivePositions={{ state: "unavailable", message: "No live positions" }}
      />,
    );

    expect(
      await screen.findByRole("heading", { name: /operations data has been cleared from view/i }),
    ).toBeInTheDocument();
    expect(screen.queryByText("Simulation Operator · administrator")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: /secure sign in/i })).toHaveAttribute(
      "href",
      "/sign-in",
    );
  });
});
