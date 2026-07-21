import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { PublicFlightTrackerDemo } from "./public-flight-tracker-demo";

describe("public flight tracker demo", () => {
  it("shows the map, flight board, evidence detail, and protected-control sign in", async () => {
    const user = userEvent.setup();
    render(<PublicFlightTrackerDemo />);

    expect(screen.getByRole("heading", { name: "Fleet + weather" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Flight board" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "FT101" })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /review an alert/i })).not.toBeInTheDocument();
    expect(screen.getByText(/sign in to review alerts and protected actions/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /sign in for protected controls/i })).toHaveAttribute(
      "href",
      "/sign-in",
    );

    await user.click(screen.getByRole("button", { name: /select flight ft303/i }));
    expect(screen.getByRole("heading", { name: "FT303" })).toBeInTheDocument();
  });
});
