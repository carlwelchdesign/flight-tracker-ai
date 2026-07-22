import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MapFloatingPanel } from "./map-floating-panel";

afterEach(() => vi.unstubAllGlobals());

describe("MapFloatingPanel", () => {
  it("closes accessibly and remains mounted while hidden", async () => {
    const onClose = vi.fn();
    const { rerender } = renderPanel({ onClose });

    await userEvent.setup().click(screen.getByRole("button", { name: "Close NOAA weather panel" }));
    expect(onClose).toHaveBeenCalledOnce();

    rerender(panel({ visible: false, onClose }));
    expect(screen.getByText("Weather controls").closest("section")).toHaveAttribute("hidden");
  });

  it("supports bounded pointer and keyboard movement", () => {
    renderPanel();
    const panelElement = screen.getByRole("region", { name: "NOAA weather panel" });
    const handle = screen.getByRole("button", { name: /Move NOAA weather panel/i });
    const stage = panelElement.parentElement as HTMLElement;
    Object.defineProperty(handle, "setPointerCapture", { value: vi.fn() });
    Object.defineProperty(handle, "hasPointerCapture", { value: vi.fn(() => true) });
    Object.defineProperty(handle, "releasePointerCapture", { value: vi.fn() });
    vi.spyOn(stage, "getBoundingClientRect").mockReturnValue(rect(0, 0, 500, 400));
    vi.spyOn(panelElement, "getBoundingClientRect").mockReturnValue(rect(20, 20, 200, 160));

    fireEvent.pointerDown(handle, { pointerId: 1, button: 0, clientX: 30, clientY: 30 });
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 900, clientY: 900 });
    fireEvent.pointerUp(handle, { pointerId: 1 });
    expect(panelElement).toHaveStyle({ transform: "translate3d(280px, 220px, 0)" });

    fireEvent.keyDown(handle, { key: "Home" });
    expect(panelElement).toHaveStyle({ transform: "translate3d(0px, 0px, 0)" });
  });

  it("disables dragging when responsive panels return to document flow", () => {
    vi.stubGlobal("matchMedia", vi.fn(() => ({
      matches: true,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })));
    renderPanel();

    expect(screen.getByRole("button", { name: /Move NOAA weather panel/i })).toBeDisabled();
  });
});

function renderPanel(overrides: Partial<React.ComponentProps<typeof MapFloatingPanel>> = {}) {
  return render(panel(overrides));
}

function panel(overrides: Partial<React.ComponentProps<typeof MapFloatingPanel>> = {}) {
  return (
    <div>
      <MapFloatingPanel
        className="weather-map-panel"
        label="NOAA weather panel"
        title="NOAA weather"
        visible
        active
        onActivate={vi.fn()}
        onClose={vi.fn()}
        {...overrides}
      >
        Weather controls
      </MapFloatingPanel>
    </div>
  );
}

function rect(left: number, top: number, width: number, height: number): DOMRect {
  return { left, top, width, height, right: left + width, bottom: top + height, x: left, y: top, toJSON: () => ({}) } as DOMRect;
}
