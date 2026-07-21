import { afterEach, describe, expect, it, vi } from "vitest";
import { AuthSessionError, createInternalAssertion } from "./auth-server";

function configureDevelopmentAssertion() {
  vi.stubEnv("AUTH_MODE", "development");
  vi.stubEnv("DEV_AUTH_SUBJECT", "test-user");
  vi.stubEnv("DEV_AUTH_TENANT_ID", "test-tenant");
  vi.stubEnv("DEV_AUTH_SESSION_ID", "test-session");
  vi.stubEnv("INTERNAL_AUTH_KEY_ID", "test-primary-2026-07");
  vi.stubEnv("INTERNAL_AUTH_SECRET", "test-internal-auth-secret-at-least-thirty-two-bytes");
  vi.stubEnv("AUTH_ASSERTION_ISSUER", "test-web");
  vi.stubEnv("AUTH_ASSERTION_AUDIENCE", "test-api");
}

function decodePart(token: string, index: number): Record<string, unknown> {
  const part = token.split(".")[index];
  return JSON.parse(Buffer.from(part, "base64url").toString("utf8")) as Record<string, unknown>;
}

describe("internal assertion signing", () => {
  afterEach(() => vi.unstubAllEnvs());

  it("names the active key and binds a short-lived server-side identity", async () => {
    configureDevelopmentAssertion();

    const token = await createInternalAssertion();
    const header = decodePart(token, 0);
    const payload = decodePart(token, 1);

    expect(header).toMatchObject({ alg: "HS256", typ: "JWT", kid: "test-primary-2026-07" });
    expect(payload).toMatchObject({
      iss: "test-web",
      aud: "test-api",
      sub: "test-user",
      provider: "development",
      tenant: "test-tenant",
      sid: "test-session",
    });
    expect(Number(payload.exp) - Number(payload.iat)).toBe(30);
  });

  it("rejects missing or unsafe key identifiers", async () => {
    configureDevelopmentAssertion();
    vi.stubEnv("INTERNAL_AUTH_KEY_ID", "");
    await expect(createInternalAssertion()).rejects.toBeInstanceOf(AuthSessionError);

    vi.stubEnv("INTERNAL_AUTH_KEY_ID", "unsafe key id");
    await expect(createInternalAssertion()).rejects.toThrow(/must use 1-64 ASCII/);
  });
});
