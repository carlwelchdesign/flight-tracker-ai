import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { OperationsTrustBanner } from "./operations-trust-banner";

describe("operational trust banner", () => {
  it.each([
    ["simulation", "Simulation environment", "Replay and public-source evaluation"],
    ["evaluation", "Evaluation environment", "Source-attributed evaluation data"],
  ] as const)("keeps %s limitations and source scope visible", (mode, label, sourceScope) => {
    render(<OperationsTrustBanner context={{ mode, label, sourceScope }} />);

    const banner = screen.getByLabelText("Operational use limitation");
    expect(banner).toHaveAttribute("data-operations-mode", mode);
    expect(banner).toHaveTextContent(label);
    expect(banner).toHaveTextContent(
      "Advisory only — not for flight planning, dispatch release, or aircraft control.",
    );
    expect(banner).toHaveTextContent(sourceScope);
    expect(banner).toHaveTextContent(/verify source authority and freshness before action/i);
  });
});
