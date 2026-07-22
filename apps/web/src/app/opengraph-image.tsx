import { ImageResponse } from "next/og";
import { SITE_NAME, SOCIAL_IMAGE_ALT } from "./site-metadata";

export const alt = SOCIAL_IMAGE_ALT;
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

const statStyle = {
  border: "1px solid rgba(125, 211, 252, 0.24)",
  borderRadius: 18,
  display: "flex",
  flexDirection: "column" as const,
  padding: "16px 22px",
  width: 290,
};

export default function OpenGraphImage() {
  return new ImageResponse(
    <div
      style={{
        background:
          "radial-gradient(circle at 78% 18%, rgba(14, 165, 233, 0.22), transparent 31%), linear-gradient(135deg, #071019 0%, #0a1925 54%, #07121b 100%)",
        color: "#f8fafc",
        display: "flex",
        flexDirection: "column",
        height: "100%",
        justifyContent: "space-between",
        padding: "62px 70px 54px",
        position: "relative",
        width: "100%",
      }}
    >
      <div
        style={{
          backgroundImage:
            "linear-gradient(rgba(125, 211, 252, 0.055) 1px, transparent 1px), linear-gradient(90deg, rgba(125, 211, 252, 0.055) 1px, transparent 1px)",
          backgroundSize: "52px 52px",
          display: "flex",
          inset: 0,
          position: "absolute",
        }}
      />

      <div style={{ display: "flex", flexDirection: "column" }}>
        <div
          style={{
            alignItems: "center",
            color: "#7dd3fc",
            display: "flex",
            fontSize: 22,
            fontWeight: 700,
            letterSpacing: 4,
            textTransform: "uppercase",
          }}
        >
          <span
            style={{
              background: "#38bdf8",
              borderRadius: 999,
              boxShadow: "0 0 24px rgba(56, 189, 248, 0.7)",
              display: "flex",
              height: 12,
              marginRight: 16,
              width: 12,
            }}
          />
          Public flight intelligence
        </div>

        <div
          style={{
            display: "flex",
            fontSize: 74,
            fontWeight: 800,
            letterSpacing: -3,
            lineHeight: 1,
            marginTop: 30,
          }}
        >
          {SITE_NAME}
        </div>
        <div
          style={{
            color: "#bac8d5",
            display: "flex",
            fontSize: 30,
            lineHeight: 1.35,
            marginTop: 24,
            maxWidth: 820,
          }}
        >
          Live traffic, trajectories, weather, and explainable attention—on one navigable map.
        </div>
      </div>

      <div
        style={{
          alignItems: "flex-end",
          display: "flex",
          justifyContent: "space-between",
        }}
      >
        <div style={{ display: "flex" }}>
          <div style={statStyle}>
            <span style={{ color: "#7dd3fc", fontSize: 16, letterSpacing: 2 }}>
              LIVE CONTEXT
            </span>
            <span style={{ fontSize: 23, marginTop: 7 }}>Regional aircraft + NOAA</span>
          </div>
          <div style={{ ...statStyle, marginLeft: 16 }}>
            <span style={{ color: "#7dd3fc", fontSize: 16, letterSpacing: 2 }}>
              RELIABLE DEMO
            </span>
            <span style={{ fontSize: 23, marginTop: 7 }}>Deterministic replay</span>
          </div>
        </div>
        <div
          style={{
            color: "#8fa4b5",
            display: "flex",
            fontSize: 18,
            paddingBottom: 4,
          }}
        >
          Portfolio demo · Not for operational use
        </div>
      </div>
    </div>,
    size,
  );
}
