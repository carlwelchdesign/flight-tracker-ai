import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TrackerExplorationControls } from "./tracker-exploration-controls";

describe("tracker exploration controls", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("searches, clears, and explains an expired shared aircraft", async () => {
    const user = userEvent.setup();
    const onQueryChange = vi.fn();
    const onClearMissingAircraft = vi.fn();
    render(<TrackerExplorationControls
      query="UAL"
      visibleCount={1}
      totalCount={3}
      missingAircraftKey="UAL404"
      onQueryChange={onQueryChange}
      onClearMissingAircraft={onClearMissingAircraft}
      getShareUrl={() => "https://example.test/?aircraft=UAL404"}
    />);

    expect(screen.getByText("1 of 3 aircraft")).toBeInTheDocument();
    expect(screen.getByText(/no longer in this snapshot/i)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Clear search" }));
    expect(onQueryChange).toHaveBeenCalledWith("");
    await user.click(screen.getByRole("button", { name: "Clear aircraft selection" }));
    expect(onClearMissingAircraft).toHaveBeenCalledOnce();
  });

  it("copies the exact bounded view URL and reports success", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    render(<TrackerExplorationControls
      query=""
      visibleCount={2}
      totalCount={2}
      missingAircraftKey={null}
      onQueryChange={() => undefined}
      onClearMissingAircraft={() => undefined}
      getShareUrl={() => "https://example.test/?mode=replay&t=60000"}
    />);

    await user.click(screen.getByRole("button", { name: "Copy link" }));
    expect(writeText).toHaveBeenCalledWith("https://example.test/?mode=replay&t=60000");
    expect(screen.getByRole("status")).toHaveTextContent("Link copied");
  });
});
