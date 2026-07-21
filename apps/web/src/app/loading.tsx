export default function Loading() {
  return (
    <main className="operations-shell operations-loading" aria-label="Loading operations console">
      <header className="operations-header loading-header">
        <div className="loading-block loading-brand" />
        <div className="loading-block loading-summary" />
        <div className="loading-block loading-control" />
      </header>
      <div className="operations-grid">
        <div className="ops-panel loading-panel"><span className="loading-sweep" /></div>
        <div className="ops-panel loading-panel"><span className="loading-row" /><span className="loading-row" /><span className="loading-row" /></div>
        <div className="ops-panel loading-panel"><span className="loading-row" /><span className="loading-row" /></div>
      </div>
      <p className="sr-only" role="status">Loading fleet state and operational evidence.</p>
    </main>
  );
}
