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
  flight_callsign: "SIM204",
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
  workflow_version: 1,
  assigned_identity_id: null,
  assigned_subject: null,
  assigned_display_name: null,
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
const assignee = {
  identity_id: "00000000-0000-0000-0000-000000000099",
  subject: "dispatcher-one",
  display_name: "Dispatcher One",
  role: "dispatcher",
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
            assigned_identity_id: null,
            dismissal_reason: null,
          }],
        });
      }
      if (url.endsWith("/alerts/assignees")) {
        return Response.json({ data: [] });
      }
      if (url.includes(`/alerts/${baseAlert.id}`)) {
        return Response.json({ ...baseAlert, actions: [] });
      }
      return Response.json({ data: [baseAlert] });
    });
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();

    render(<AlertQueue canManage refreshRevision={0} />);

    expect(await screen.findByRole("heading", { name: "80/100 attention" })).toBeInTheDocument();
    expect(screen.getByText("Hazard severity")).toBeInTheDocument();
    expect(screen.getByText(/route v1, hazard r1, rule v1, score v1/i)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Acknowledge" }));
    await waitFor(() => expect(screen.getByText("Acknowledge", { selector: ".alert-audit strong" })).toBeInTheDocument());
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining(`/alerts/${baseAlert.id}/actions`),
      expect.objectContaining({ method: "POST" }),
    );
    const actionCall = fetchMock.mock.calls.find(([, init]) => init?.method === "POST");
    const body = JSON.parse(String(actionCall?.[1]?.body));
    expect(body).not.toHaveProperty("operator_id");
    expect(body).not.toHaveProperty("actor_id");
    expect(body.expected_workflow_version).toBe(1);
  });

  it("requires an explanation for an other dismissal reason", async () => {
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) =>
      String(input).endsWith("/alerts/assignees")
        ? Response.json({ data: [] })
        : String(input).includes(`/alerts/${baseAlert.id}`)
        ? Response.json({ ...baseAlert, actions: [] })
        : Response.json({ data: [baseAlert] })));
    const user = userEvent.setup();
    render(<AlertQueue canManage refreshRevision={0} />);

    await screen.findByRole("button", { name: "Dismiss" });
    await user.selectOptions(screen.getByLabelText("Dismissal reason"), "other");
    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.getByRole("alert")).toHaveTextContent(/explain the other dismissal reason/i);
  });

  it("assigns an active dispatcher with optimistic concurrency feedback", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (init?.method === "POST") {
        return Response.json({
          ...baseAlert,
          workflow_version: 2,
          assigned_identity_id: assignee.identity_id,
          assigned_subject: assignee.subject,
          assigned_display_name: assignee.display_name,
          actions: [{
            id: "00000000-0000-0000-0000-000000000098",
            action: "assign",
            actor_id: "dispatcher:local",
            occurred_at: "2026-07-20T16:04:00Z",
            comment: null,
            assigned_identity_id: assignee.identity_id,
            dismissal_reason: null,
          }],
        });
      }
      if (url.endsWith("/alerts/assignees")) return Response.json({ data: [assignee] });
      if (url.includes(`/alerts/${baseAlert.id}`)) return Response.json({ ...baseAlert, actions: [] });
      return Response.json({ data: [baseAlert] });
    });
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();
    render(<AlertQueue canManage refreshRevision={0} />);

    await user.selectOptions(await screen.findByLabelText("Assigned dispatcher"), assignee.identity_id);
    await user.click(screen.getByRole("button", { name: "Assign" }));
    expect(await screen.findByRole("status")).toHaveTextContent(/assignment updated/i);
    const actionCall = fetchMock.mock.calls.find(([, init]) => init?.method === "POST");
    expect(JSON.parse(String(actionCall?.[1]?.body))).toMatchObject({
      action: "assign",
      expected_workflow_version: 1,
      assigned_identity_id: assignee.identity_id,
    });
  });

  it("applies severity, status, flight, time, and assignee filters", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/alerts/assignees")) return Response.json({ data: [assignee] });
      if (url.includes(`/alerts/${baseAlert.id}`)) return Response.json({ ...baseAlert, actions: [] });
      return Response.json({ data: [baseAlert] });
    });
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();
    render(<AlertQueue canManage refreshRevision={0} />);

    await screen.findByRole("heading", { name: "80/100 attention" });
    await user.selectOptions(screen.getByLabelText("Severity"), "warning");
    await user.selectOptions(screen.getByLabelText("Status"), "open");
    await user.type(screen.getByLabelText("Flight"), baseAlert.flight_callsign);
    await user.selectOptions(screen.getByLabelText("Event time"), "6h");
    await user.selectOptions(screen.getByLabelText("Assigned user"), assignee.identity_id);
    await user.click(screen.getByRole("button", { name: "Apply filters" }));

    await waitFor(() => expect(fetchMock.mock.calls.some(([input]) => {
      const url = String(input);
      return url.includes("severity=warning") && url.includes("status=open") &&
        url.includes(`flight=${baseAlert.flight_callsign}`) && url.includes("event_from=") &&
        url.includes(`assigned_to=${assignee.identity_id}`);
    })).toBe(true));
  });

  it("loads the latest state when another dispatcher wins a concurrent update", async () => {
    let detailLoads = 0;
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (init?.method === "POST") {
        return Response.json({ error: { code: "alert_conflict", message: "stale version" } }, { status: 409 });
      }
      if (url.endsWith("/alerts/assignees")) return Response.json({ data: [] });
      if (url.includes(`/alerts/${baseAlert.id}`)) {
        detailLoads += 1;
        return Response.json({ ...baseAlert, workflow_version: detailLoads > 1 ? 2 : 1, lifecycle: detailLoads > 1 ? "acknowledged" : "open", actions: [] });
      }
      return Response.json({ data: [baseAlert] });
    });
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();
    render(<AlertQueue canManage refreshRevision={0} />);

    await user.click(await screen.findByRole("button", { name: "Acknowledge" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/another dispatcher updated/i);
    expect(detailLoads).toBeGreaterThan(1);
  });

  it("keeps a representative 150-alert queue bounded and reviewable", async () => {
    const volume = Array.from({ length: 150 }, (_, index) => ({
      ...baseAlert,
      id: `00000000-0000-0000-0000-${String(index).padStart(12, "0")}`,
      attention_score: 100 - (index % 100),
    }));
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/alerts/assignees")) return Response.json({ data: [] });
      if (/\/alerts\/[0-9a-f-]+$/.test(url)) return Response.json({ ...volume[0], actions: [] });
      return Response.json({ data: volume });
    }));
    render(<AlertQueue canManage refreshRevision={0} />);

    expect(await screen.findByText("150 shown")).toBeInTheDocument();
    expect(document.querySelectorAll(".alert-row")).toHaveLength(150);
  });
});
