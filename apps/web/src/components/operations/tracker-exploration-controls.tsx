"use client";

import { useState } from "react";

export function TrackerExplorationControls({
  query,
  visibleCount,
  totalCount,
  missingAircraftKey,
  onQueryChange,
  onClearMissingAircraft,
  getShareUrl,
}: {
  query: string;
  visibleCount: number;
  totalCount: number;
  missingAircraftKey: string | null;
  onQueryChange: (query: string) => void;
  onClearMissingAircraft: () => void;
  getShareUrl: () => string;
}) {
  const [feedback, setFeedback] = useState<string | null>(null);

  async function copyLink(success = "Link copied") {
    try {
      await navigator.clipboard.writeText(getShareUrl());
      setFeedback(success);
    } catch {
      setFeedback("Copy failed — use the browser address bar");
    }
  }

  async function shareLink() {
    if (typeof navigator.share !== "function") {
      await copyLink("Link copied — native share is unavailable here");
      return;
    }
    try {
      await navigator.share({ title: "Flight Tracker AI", url: getShareUrl() });
      setFeedback("Share opened");
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        setFeedback("Share cancelled");
      } else {
        await copyLink("Native share failed — link copied instead");
      }
    }
  }

  return (
    <div className="tracker-exploration-controls">
      <label className="aircraft-search-control">
        <span>Search this picture</span>
        <input
          type="search"
          value={query}
          placeholder="Callsign, ICAO hex, or registration"
          maxLength={24}
          onChange={(event) => onQueryChange(event.target.value)}
        />
      </label>
      <div className="tracker-search-summary" aria-live="polite">
        {query.trim() ? `${visibleCount} of ${totalCount} aircraft` : `${totalCount} aircraft in this picture`}
        {query && <button type="button" onClick={() => onQueryChange("")}>Clear search</button>}
      </div>
      {missingAircraftKey && (
        <div className="shared-aircraft-missing" role="status">
          <strong>{missingAircraftKey} is no longer in this snapshot.</strong>
          <span>The shared region and map view are still available.</span>
          <button type="button" onClick={onClearMissingAircraft}>Clear aircraft selection</button>
        </div>
      )}
      <div className="tracker-share-actions">
        <button type="button" onClick={() => void copyLink()}>Copy link</button>
        <button type="button" onClick={() => void shareLink()}>Share view</button>
        {feedback && <span role="status">{feedback}</span>}
      </div>
    </div>
  );
}
