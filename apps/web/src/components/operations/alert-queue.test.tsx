import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AlertQueue } from "./alert-queue";

const operatorId = "00000000-0000-0000-0000-000000000002";
const baseAlert = {
  id: "00000000-0000-0000-0000-000000000001",
  operator_id: operatorId,
  event_time: "2026-07-20T16:03:00Z",
  flight_id: "00000000-0000-0000-0000-000000000003",
  hazard_id: "00000000-0000-0000-0000-000000000004",
  alert_type: "route_hazard_proximity",
  severity: "warning",
  lifecycle: "open",
  rule_id: "route_hazard_proximity",
  rule_version: 1,
  series_key: "series-1",
  alert_revision: 1,
  supersedes_alert_id: null,
  attention_score: 80,
  score_version: 1,
  evidence: {
    attention: {
      hazard_severity_points: 45,
      horizontal_proximity_points: 25,
      altitude_overlap_points: 10,
      time_urgency_points: 0,
      total: 80,
      score_version: 1,
    },
    route_hazard: {
      closest_approach_nm: 0,
      proximity_margin_nm: 25,
      route_version: 1,
      hazard_revision: 1,
      horizontal_relation: "intersects",
      altitude_relation: "overlap",
      evaluated_at: "2026-07-20T16:03:00Z",
    },
  },
};

afterEach(() => vi.unstubAllGlobals());

describe("dispatcher alert queue", () => {
  it("shows explainable ranking and appends a human acknowledgement", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (init?.method === "POST") {
        return Response.json({
          ...baseAlert,
          lifecycle: "acknowledged",
          actions: [{
            id: "00000000-0000-0000-0000-000000000005",
            action: "acknowledge",
            actor_id: "dispatcher:local",
            occurred_at: "2026-07-20T16:04:00Z",
            comment: null,
          }],
        });
      }
      if (url.includes(`/alerts/${baseAlert.id}`)) {
        return Response.json({ ...baseAlert, actions: [] });
      }
      return Response.json({ data: [baseAlert] });
    });
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();

    render(<AlertQueue operatorId={operatorId} refreshRevision={0} />);

    expect(await screen.findByRole("heading", { name: "80/100 attention" })).toBeInTheDocument();
    expect(screen.getByText("Hazard severity")).toBeInTheDocument();
    expect(screen.getByText(/route v1, hazard r1, rule v1, score v1/i)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Acknowledge" }));
    await waitFor(() => expect(screen.getByText("Acknowledge", { selector: ".alert-audit strong" })).toBeInTheDocument());
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining(`/alerts/${baseAlert.id}/actions`),
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("requires a reason before dismissal", async () => {
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) =>
      String(input).includes(`/alerts/${baseAlert.id}`)
        ? Response.json({ ...baseAlert, actions: [] })
        : Response.json({ data: [baseAlert] })));
    const user = userEvent.setup();
    render(<AlertQueue operatorId={operatorId} refreshRevision={0} />);

    await user.click(await screen.findByRole("button", { name: "Dismiss with reason" }));
    expect(screen.getByRole("alert")).toHaveTextContent(/enter a dismissal reason/i);
  });
});
