const LIVE_MARKER_GLYPH_OFFSET_DEGREES = -90;

export function liveMarkerRotationDegrees(headingTrueDegrees: number | null): number {
  return (headingTrueDegrees ?? 0) + LIVE_MARKER_GLYPH_OFFSET_DEGREES;
}
