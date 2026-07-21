import { NextRequest } from "next/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { GET, POST } from "./route";

afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("backend audit proxy", () => {
  it("allows bounded audit export and preserves download headers", async () => {
    vi.stubEnv("AUTH_MODE", "development");
    vi.stubEnv("DEV_AUTH_SUBJECT", "local-admin");
    vi.stubEnv("DEV_AUTH_TENANT_ID", "local-flight-tracker");
    vi.stubEnv("INTERNAL_AUTH_KEY_ID", "local-primary");
    vi.stubEnv("INTERNAL_AUTH_SECRET", "local-development-internal-auth-secret-change-me");
    const backendFetch = vi.fn(async () =>
      new Response("audit,csv\r\n", {
        headers: {
          "content-type": "text/csv; charset=utf-8",
          "content-disposition": "attachment; filename=flight-tracker-audit.csv",
          "cache-control": "no-store",
        },
      }),
    );
    vi.stubGlobal("fetch", backendFetch);

    const request = new NextRequest(
      "http://localhost/api/backend/api/admin/audit-events/export?from=2026-07-20T00%3A00%3A00Z&to=2026-07-21T00%3A00%3A00Z",
    );
    const response = await GET(request, {
      params: Promise.resolve({ path: ["api", "admin", "audit-events", "export"] }),
    });

    expect(response.status).toBe(200);
    expect(response.headers.get("content-disposition")).toBe(
      "attachment; filename=flight-tracker-audit.csv",
    );
    expect(backendFetch).toHaveBeenCalledWith(
      expect.objectContaining({
        pathname: "/api/admin/audit-events/export",
      }),
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("does not broaden the proxy to arbitrary administrator paths", async () => {
    const backendFetch = vi.fn();
    vi.stubGlobal("fetch", backendFetch);
    const response = await GET(new NextRequest("http://localhost/api/backend/api/admin/secrets"), {
      params: Promise.resolve({ path: ["api", "admin", "secrets"] }),
    });
    expect(response.status).toBe(404);
    expect(backendFetch).not.toHaveBeenCalled();
  });

  it("allows only the explicit retention workflow actions", async () => {
    vi.stubEnv("AUTH_MODE", "development");
    vi.stubEnv("DEV_AUTH_SUBJECT", "local-admin");
    vi.stubEnv("DEV_AUTH_TENANT_ID", "local-flight-tracker");
    vi.stubEnv("INTERNAL_AUTH_KEY_ID", "local-primary");
    vi.stubEnv("INTERNAL_AUTH_SECRET", "local-development-internal-auth-secret-change-me");
    const backendFetch = vi.fn(async () => Response.json({ status: "awaiting_approval" }));
    vi.stubGlobal("fetch", backendFetch);
    const response = await POST(
      new NextRequest("http://localhost/api/backend/api/admin/retention/runs/preview", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ policy_id: "policy-1", evidence_reference: "incident:FT-401" }),
      }),
      { params: Promise.resolve({ path: ["api", "admin", "retention", "runs", "preview"] }) },
    );
    expect(response.status).toBe(200);
    expect(backendFetch).toHaveBeenCalledWith(
      expect.objectContaining({ pathname: "/api/admin/retention/runs/preview" }),
      expect.objectContaining({ method: "POST" }),
    );

    const scheduleResponse = await POST(
      new NextRequest("http://localhost/api/backend/api/admin/retention/schedules/schedule-1/pause", {
        method: "POST",
      }),
      {
        params: Promise.resolve({
          path: ["api", "admin", "retention", "schedules", "schedule-1", "pause"],
        }),
      },
    );
    expect(scheduleResponse.status).toBe(200);
    expect(backendFetch).toHaveBeenLastCalledWith(
      expect.objectContaining({ pathname: "/api/admin/retention/schedules/schedule-1/pause" }),
      expect.objectContaining({ method: "POST" }),
    );
  });
});
