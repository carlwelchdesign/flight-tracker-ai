"use client";

import { useRef, useState, useSyncExternalStore, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";

type Position = { x: number; y: number };

type DragState = {
  pointerId: number;
  startX: number;
  startY: number;
  origin: Position;
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
};

type Props = {
  className: string;
  label: string;
  title: string;
  visible: boolean;
  active: boolean;
  onActivate: () => void;
  onClose: () => void;
  children: ReactNode;
};

const KEYBOARD_STEP = 16;
const COMPACT_PANEL_QUERY = "(max-width: 820px)";

export function MapFloatingPanel({
  className,
  label,
  title,
  visible,
  active,
  onActivate,
  onClose,
  children,
}: Props) {
  const panelRef = useRef<HTMLElement>(null);
  const dragRef = useRef<DragState | null>(null);
  const [position, setPosition] = useState<Position>({ x: 0, y: 0 });
  const compact = useSyncExternalStore(subscribeToCompactViewport, compactViewportSnapshot, () => false);

  function beginDrag(event: PointerEvent<HTMLButtonElement>) {
    if (compact || event.button !== 0 || !panelRef.current?.parentElement) return;
    const panelBounds = panelRef.current.getBoundingClientRect();
    const stageBounds = panelRef.current.parentElement.getBoundingClientRect();
    dragRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      origin: position,
      minX: position.x + stageBounds.left - panelBounds.left,
      maxX: position.x + stageBounds.right - panelBounds.right,
      minY: position.y + stageBounds.top - panelBounds.top,
      maxY: position.y + stageBounds.bottom - panelBounds.bottom,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    event.preventDefault();
    onActivate();
  }

  function continueDrag(event: PointerEvent<HTMLButtonElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    setPosition({
      x: clamp(drag.origin.x + event.clientX - drag.startX, drag.minX, drag.maxX),
      y: clamp(drag.origin.y + event.clientY - drag.startY, drag.minY, drag.maxY),
    });
  }

  function endDrag(event: PointerEvent<HTMLButtonElement>) {
    if (dragRef.current?.pointerId !== event.pointerId) return;
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  function moveWithKeyboard(event: KeyboardEvent<HTMLButtonElement>) {
    if (compact) return;
    const movement = keyboardMovement(event.key);
    if (!movement || !panelRef.current?.parentElement) return;
    event.preventDefault();
    onActivate();
    if (event.key === "Home") {
      setPosition({ x: 0, y: 0 });
      return;
    }
    const panelBounds = panelRef.current.getBoundingClientRect();
    const stageBounds = panelRef.current.parentElement.getBoundingClientRect();
    setPosition((current) => ({
      x: current.x + clamp(movement.x, stageBounds.left - panelBounds.left, stageBounds.right - panelBounds.right),
      y: current.y + clamp(movement.y, stageBounds.top - panelBounds.top, stageBounds.bottom - panelBounds.bottom),
    }));
  }

  return (
    <section
      ref={panelRef}
      className={`map-floating-panel ${className}${active ? " is-active" : ""}`}
      aria-label={label}
      hidden={!visible}
      style={{ transform: `translate3d(${position.x}px, ${position.y}px, 0)` }}
      onFocusCapture={onActivate}
      onPointerDown={onActivate}
    >
      <header className="map-floating-panel-header">
        <button
          type="button"
          className="map-panel-drag-handle"
          disabled={compact}
          aria-label={`Move ${label}. Use arrow keys to move and Home to reset.`}
          onPointerDown={beginDrag}
          onPointerMove={continueDrag}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          onKeyDown={moveWithKeyboard}
        >
          <span aria-hidden="true">⠿</span>
          {title}
        </button>
        <button type="button" className="map-panel-close" aria-label={`Close ${label}`} onClick={onClose}>×</button>
      </header>
      <div className="map-floating-panel-content">{children}</div>
    </section>
  );
}

function keyboardMovement(key: string): Position | null {
  if (key === "ArrowLeft") return { x: -KEYBOARD_STEP, y: 0 };
  if (key === "ArrowRight") return { x: KEYBOARD_STEP, y: 0 };
  if (key === "ArrowUp") return { x: 0, y: -KEYBOARD_STEP };
  if (key === "ArrowDown") return { x: 0, y: KEYBOARD_STEP };
  if (key === "Home") return { x: 0, y: 0 };
  return null;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(Math.max(value, minimum), maximum);
}

function subscribeToCompactViewport(listener: () => void) {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return () => undefined;
  const query = window.matchMedia(COMPACT_PANEL_QUERY);
  query.addEventListener?.("change", listener);
  return () => query.removeEventListener?.("change", listener);
}

function compactViewportSnapshot() {
  return typeof window !== "undefined" && typeof window.matchMedia === "function" &&
    window.matchMedia(COMPACT_PANEL_QUERY).matches;
}
