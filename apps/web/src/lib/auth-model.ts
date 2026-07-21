export type AuthRole = "viewer" | "dispatcher" | "operator" | "administrator";

export type AuthContext = {
  identity_id: string;
  operator_id: string;
  operator_code: string;
  operator_name: string;
  provider: string;
  subject: string;
  session_id: string;
  role: AuthRole;
};

export function parseAuthContext(value: unknown): AuthContext {
  if (!isRecord(value)) throw new Error("Authorization API returned an invalid context");
  for (const key of [
    "identity_id",
    "operator_id",
    "operator_code",
    "operator_name",
    "provider",
    "subject",
    "session_id",
  ] as const) {
    if (typeof value[key] !== "string") {
      throw new Error("Authorization API returned an invalid context");
    }
  }
  if (!["viewer", "dispatcher", "operator", "administrator"].includes(String(value.role))) {
    throw new Error("Authorization API returned an invalid role");
  }
  return value as AuthContext;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
