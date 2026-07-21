import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AuditReview } from "./audit-review";

afterEach(() => vi.unstubAllGlobals());

describe("audit review", () => {
  it("shows privileged signals and a bounded redacted export", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        if (url.includes("audit-alerts")) {
          return Response.json({
            data: [
              {
                code: "high_risk_action",
                severity: "warning",
                actor_id: "identity-1",
                occurred_at: "2026-07-21T12:00:00Z",
                event_id: "event-1",
                message: "High-risk audit action recorded: session.revoked",
              },
            ],
            since: "2026-07-20T12:00:00Z",
            through: "2026-07-21T12:00:00Z",
          });
        }
        return Response.json({
          data: [
            {
              id: "event-1",
              source: "authorization",
              actor_id: "identity-1",
              action: "session.revoked",
              target_type: "auth_session",
              target_reference: null,
              occurred_at: "2026-07-21T12:00:00Z",
              details: { provider: "clerk", identity_id: "identity-2" },
              risk: "high",
            },
          ],
          from: "2026-07-20T12:00:00Z",
          to: "2026-07-21T12:00:00Z",
        });
      }),
    );

    render(<AuditReview refreshRevision={0} />);

    expect(await screen.findByText(/high-risk audit action recorded/i)).toBeInTheDocument();
    expect(screen.getByText("Session revoked")).toBeInTheDocument();
    expect(screen.queryByText(/sensitive-session/i)).not.toBeInTheDocument();
    expect(screen.getByText(/scanned for sensitive content but never returned/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /export redacted csv/i })).toHaveAttribute(
      "href",
      expect.stringContaining("audit-events/export"),
    );
  });

  it("keeps backend failures explicit", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => Response.json({}, { status: 503 })));
    render(<AuditReview refreshRevision={0} />);
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent(/temporarily unavailable/i),
    );
  });
});
