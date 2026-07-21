type IdentityFailure = {
  status: 401 | 403 | 500;
  message: string;
};

export function describeConsoleFailure(
  identityFailure: IdentityFailure | null,
  operationsMode: string | undefined,
) {
  if (identityFailure?.status === 401) {
    return {
      signedOut: true,
      message: "Your operations data remains protected until a valid session is available.",
    };
  }
  if (identityFailure?.status === 500 && operationsMode === "evaluation") {
    return {
      signedOut: false,
      message:
        "The portfolio configuration is not ready yet. Access remains closed while setup is completed.",
    };
  }
  if (identityFailure) {
    return { signedOut: false, message: identityFailure.message };
  }
  return {
    signedOut: false,
    message:
      operationsMode === "evaluation"
        ? "The portfolio service may be waking from its idle state. Wait up to one minute, then try again; no operational data is implied."
        : "The console service is not ready. Try again after its health checks recover.",
  };
}
