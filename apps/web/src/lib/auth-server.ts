import "server-only";

import { createHmac, randomUUID } from "node:crypto";

export class AuthSessionError extends Error {
  constructor(
    message: string,
    public readonly status: 401 | 403 | 500 = 401,
  ) {
    super(message);
  }
}

type SessionIdentity = {
  provider: "development" | "clerk";
  subject: string;
  tenant: string;
  sessionId: string;
};

export function authMode(): "development" | "clerk" {
  const mode = process.env.AUTH_MODE ?? "development";
  if (mode !== "development" && mode !== "clerk") {
    throw new AuthSessionError("AUTH_MODE must be development or clerk", 500);
  }
  return mode;
}

export async function createInternalAssertion(): Promise<string> {
  const identity = await currentIdentity();
  const secret = process.env.INTERNAL_AUTH_SECRET ?? "";
  if (Buffer.byteLength(secret) < 32) {
    throw new AuthSessionError("INTERNAL_AUTH_SECRET must contain at least 32 bytes", 500);
  }
  const now = Math.floor(Date.now() / 1_000);
  const header = encode({ alg: "HS256", typ: "JWT" });
  const payload = encode({
    iss: process.env.AUTH_ASSERTION_ISSUER ?? "flight-tracker-web",
    aud: process.env.AUTH_ASSERTION_AUDIENCE ?? "flight-tracker-api",
    sub: identity.subject,
    provider: identity.provider,
    tenant: identity.tenant,
    sid: identity.sessionId,
    jti: randomUUID(),
    iat: now,
    nbf: now,
    exp: now + 30,
  });
  const unsigned = `${header}.${payload}`;
  const signature = createHmac("sha256", secret).update(unsigned).digest("base64url");
  return `${unsigned}.${signature}`;
}

async function currentIdentity(): Promise<SessionIdentity> {
  if (authMode() === "development") {
    return {
      provider: "development",
      subject: required("DEV_AUTH_SUBJECT"),
      tenant: required("DEV_AUTH_TENANT_ID"),
      sessionId: process.env.DEV_AUTH_SESSION_ID ?? "local-development-session",
    };
  }

  const { auth } = await import("@clerk/nextjs/server");
  const session = await auth();
  if (!session.userId || !session.sessionId) {
    throw new AuthSessionError("Sign in is required");
  }
  if (!session.orgId) {
    throw new AuthSessionError("Select an organization before opening the console", 403);
  }
  return {
    provider: "clerk",
    subject: session.userId,
    tenant: session.orgId,
    sessionId: session.sessionId,
  };
}

function required(name: string): string {
  const value = process.env[name]?.trim();
  if (!value) throw new AuthSessionError(`${name} must be configured`, 500);
  return value;
}

function encode(value: object): string {
  return Buffer.from(JSON.stringify(value)).toString("base64url");
}
