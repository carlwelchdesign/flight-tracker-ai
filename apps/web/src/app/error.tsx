"use client";

export default function ErrorPage({ reset }: { error: Error & { digest?: string }; reset: () => void }) {
  return (
    <main className="fatal-state">
      <div className="fatal-state-card">
        <p className="ops-eyebrow">Console unavailable</p>
        <h1>The operational workspace could not be rendered.</h1>
        <p>No flight state was changed. Retry the interface or verify the API connection.</p>
        <button type="button" className="ops-primary-button" onClick={reset}>Retry console</button>
      </div>
    </main>
  );
}
