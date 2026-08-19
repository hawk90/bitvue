import { FilmstripDropdown, type DisplayView } from "bitvue";
import { useState } from "react";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Thumbnails = () => {
  const [view, setView] = useState<DisplayView>("thumbnails");
  return (
    <div style={previewBg}>
      <FilmstripDropdown displayView={view} onViewChange={setView} />
    </div>
  );
};

export const HrdBuffer = () => {
  const [view, setView] = useState<DisplayView>("hrdbuffer");
  return (
    <div style={previewBg}>
      <FilmstripDropdown displayView={view} onViewChange={setView} />
    </div>
  );
};
