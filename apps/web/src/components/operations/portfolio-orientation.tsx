export function PortfolioOrientation() {
  return (
    <section className="portfolio-orientation" aria-labelledby="portfolio-orientation-title">
      <div className="portfolio-orientation-intro">
        <p className="ops-eyebrow">Recruiter walkthrough · about 3 minutes</p>
        <h1 id="portfolio-orientation-title">See which flights need attention—and why</h1>
        <p>
          This source-attributed portfolio demo combines a repeatable flight scenario,
          live aviation weather, and explainable human-reviewed alerts.
        </p>
        <nav aria-label="Walkthrough shortcuts">
          <a href="#flight-board">Explore the flight picture</a>
          <a href="#alert-review">Review an alert</a>
        </nav>
      </div>

      <ol className="portfolio-orientation-steps" aria-label="Suggested walkthrough">
        <li><span>1</span><strong>Find attention</strong><small>Select a watch flight on the board or map.</small></li>
        <li><span>2</span><strong>Inspect evidence</strong><small>Check freshness, source, route, weather, and timing.</small></li>
        <li><span>3</span><strong>Review the decision</strong><small>Open the ranked alert and its audit-ready explanation.</small></li>
      </ol>

      <dl className="portfolio-orientation-modes">
        <div><dt>Reliable demo</dt><dd>Deterministic replay</dd></div>
        <div><dt>Live context</dt><dd>NOAA weather · optional ADSB.lol positions</dd></div>
        <div><dt>Decision boundary</dt><dd>Rules rank evidence; a person takes action</dd></div>
      </dl>
    </section>
  );
}
