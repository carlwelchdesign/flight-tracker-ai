import "server-only";

import { createInternalAssertion } from "./auth-server";
import { parseAuthContext, type AuthContext } from "./auth-model";
import { getApiBaseUrl } from "./fleet-api";

export async function getAuthContext(): Promise<AuthContext> {
  const assertion = await createInternalAssertion();
  const response = await fetch(`${getApiBaseUrl()}/api/auth/context`, {
    headers: { authorization: `Bearer ${assertion}` },
    cache: "no-store",
    signal: AbortSignal.timeout(2_500),
  });
  if (!response.ok) {
    throw new Error(`Authorization API returned HTTP ${response.status}`);
  }
  return parseAuthContext(await response.json());
}
