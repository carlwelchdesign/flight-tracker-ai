import { describe, expect, it } from "vitest";
import { parseAuditEventList, parseAuditSignalList } from "./audit-api";

describe("audit API parsers", () => {
  it("accepts the redacted review and monitoring contracts", () => {
    expect(
      parseAuditEventList({
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
      }).data[0]?.target_reference,
    ).toBeNull();
    expect(
      parseAuditSignalList({
        data: [
          {
            code: "sensitive_write_detected",
            severity: "critical",
            actor_id: "identity-1",
            occurred_at: "2026-07-21T12:00:00Z",
            event_id: "event-1",
            message: "Potential sensitive content detected in dispatcher comment",
          },
        ],
        since: "2026-07-20T12:00:00Z",
        through: "2026-07-21T12:00:00Z",
      }).data,
    ).toHaveLength(1);
  });

  it("rejects unrecognized risks and detail values", () => {
    expect(() =>
      parseAuditEventList({
        data: [
          {
            id: "event-1",
            source: "authorization",
            actor_id: "identity-1",
            action: "session.revoked",
            target_type: "auth_session",
            target_reference: null,
            occurred_at: "2026-07-21T12:00:00Z",
            details: { nested: { secret: true } },
            risk: "urgent",
          },
        ],
        from: "2026-07-20T12:00:00Z",
        to: "2026-07-21T12:00:00Z",
      }),
    ).toThrow(/unexpected event/i);
  });
});
