import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { PortfolioOrientation } from "./portfolio-orientation";

describe("portfolio orientation", () => {
  it("frames the product, source modes, and self-guided workflow", () => {
    render(<PortfolioOrientation />);

    expect(
      screen.getByRole("heading", { level: 1, name: /which flights need attention—and why/i }),
    ).toBeInTheDocument();
    expect(screen.getByText("Deterministic replay")).toBeInTheDocument();
    expect(screen.getByText(/NOAA weather · optional ADS-B positions/i)).toBeInTheDocument();
    expect(screen.getByText(/a person takes action/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /explore the flight picture/i })).toHaveAttribute(
      "href",
      "#flight-board",
    );
    expect(screen.getByRole("link", { name: /review an alert/i })).toHaveAttribute(
      "href",
      "#alert-review",
    );
  });
});
