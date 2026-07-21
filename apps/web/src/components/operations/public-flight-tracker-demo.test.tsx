import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { PublicFlightTrackerDemo } from "./public-flight-tracker-demo";

describe("public flight tracker demo", () => {
  it("shows the navigable map, replay fallback, aircraft detail, and protected console link", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 503 })));
    const user = userEvent.setup();
    render(<PublicFlightTrackerDemo />);

    expect(screen.getByRole("heading", { name: "Bay Area traffic" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Aircraft" })).toBeInTheDocument();
    expect(await screen.findByText(/replay demonstration/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "FT101" })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /review an alert/i })).not.toBeInTheDocument();
    expect(screen.getByText(/sign in to review alerts and protected actions/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /protected operations console/i })).toHaveAttribute(
      "href",
      "/sign-in",
    );

    await user.click(screen.getByRole("button", { name: /FT303/i }));
    expect(screen.getByRole("heading", { name: "FT303" })).toBeInTheDocument();
    vi.unstubAllGlobals();
  });
});
