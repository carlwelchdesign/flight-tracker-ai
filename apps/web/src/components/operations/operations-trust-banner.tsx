import type { OperationalContext } from "@/lib/operational-context";

export function OperationsTrustBanner({ context }: { context: OperationalContext }) {
  return (
    <aside
      className={`operations-trust-banner trust-banner-${context.mode}`}
      aria-label="Operational use limitation"
      data-operations-mode={context.mode}
    >
      <strong>{context.label}</strong>
      <span>Advisory only — not for flight planning, dispatch release, or aircraft control.</span>
      <span>{context.sourceScope} · Verify source authority and freshness before action.</span>
    </aside>
  );
}
