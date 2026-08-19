import { ReferenceLinesOverlay } from "bitvue";
import type { CSSProperties } from "react";
import { useRef } from "react";

// Unlike the other 25 components in this group, ReferenceLinesOverlay IS a real React component
// (SVG-based, see frontend/components/ReferenceLinesOverlay.tsx) -- it just needs a
// `containerRef` pointing at a DOM node whose children carry `data-frame-index` attributes (it
// queries `[data-frame-index="${info.to}"]` to measure the target's width for arrow fan-out).
const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

const frameBoxStyle: CSSProperties = {
  position: "absolute",
  top: 90,
  width: 56,
  height: 36,
  borderRadius: 3,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontFamily: "monospace",
  fontSize: 11,
  color: "#fff",
};

function FilmstripRow({ frames, highlight }: { frames: number[]; highlight: number }) {
  return (
    <>
      {frames.map((f, i) => (
        <div
          key={f}
          data-frame-index={f}
          style={{
            ...frameBoxStyle,
            left: 20 + i * 70,
            background: f === highlight ? "#2b6cb0" : "#333",
            outline: f === highlight ? "2px solid #63b3ed" : "1px solid #555",
          }}
        >
          #{f}
        </div>
      ))}
    </>
  );
}

export const BFrameTwoReferences = () => {
  const containerRef = useRef<HTMLDivElement>(null);
  const frames = [88, 89, 90, 91, 92];
  const expansionInfo = [
    {
      from: 92,
      fromPos: { x: 20 + 4 * 70 + 28, y: 90 },
      to: 90,
      toPos: { x: 20 + 2 * 70 + 28, y: 90 },
      color: "#63b3ed",
      arrowIndex: 0,
      arrowTotal: 2,
    },
    {
      from: 88,
      fromPos: { x: 20 + 0 * 70 + 28, y: 90 },
      to: 90,
      toPos: { x: 20 + 2 * 70 + 28, y: 90 },
      color: "#f6ad55",
      arrowIndex: 1,
      arrowTotal: 2,
    },
  ];
  return (
    <div style={previewBg}>
      <div ref={containerRef} style={{ position: "relative", width: 420, height: 160 }}>
        <FilmstripRow frames={frames} highlight={90} />
        <ReferenceLinesOverlay expansionInfo={expansionInfo} containerRef={containerRef} />
      </div>
    </div>
  );
};

export const PFrameSingleReference = () => {
  const containerRef = useRef<HTMLDivElement>(null);
  const frames = [60, 61, 62, 63];
  const expansionInfo = [
    {
      from: 60,
      fromPos: { x: 20 + 0 * 70 + 28, y: 90 },
      to: 63,
      toPos: { x: 20 + 3 * 70 + 28, y: 90 },
      color: "#68d391",
      arrowIndex: 0,
      arrowTotal: 1,
    },
  ];
  return (
    <div style={previewBg}>
      <div ref={containerRef} style={{ position: "relative", width: 320, height: 160 }}>
        <FilmstripRow frames={frames} highlight={63} />
        <ReferenceLinesOverlay expansionInfo={expansionInfo} containerRef={containerRef} />
      </div>
    </div>
  );
};
