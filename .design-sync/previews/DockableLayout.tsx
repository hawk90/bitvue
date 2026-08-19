import { DockableLayout } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

// DockableLayout's panels rely on `height: 100%` all the way down (react-resizable-panels), so
// the wrapper needs a concrete height for anything to be visible.
const layoutWrapper: CSSProperties = {
  ...previewBg,
  position: "relative",
  height: 720,
  width: "100%",
};

// Simple placeholder panel bodies with real Bitvue-domain content (not "foo"/"lorem") -- the
// component prop only needs to satisfy `React.ComponentType<Record<string, never>>`.
const StreamTreePanel = () => (
  <div style={{ padding: 10, fontSize: 12, color: "#ccc", fontFamily: "monospace" }}>
    <div>sample_1080p.ivf</div>
    <div style={{ paddingLeft: 12 }}>├─ Sequence Header</div>
    <div style={{ paddingLeft: 12 }}>├─ Frame #0 (KEY, 48213 B)</div>
    <div style={{ paddingLeft: 12 }}>├─ Frame #1 (P, 21044 B)</div>
    <div style={{ paddingLeft: 12 }}>└─ Frame #2 (B, 9887 B)</div>
  </div>
);

const SyntaxPanel = () => (
  <div style={{ padding: 10, fontSize: 12, color: "#ccc" }}>
    frame_header_obu · base_q_idx = 96 · primary_ref_frame = 2
  </div>
);

const SelectionPanel = () => (
  <div style={{ padding: 10, fontSize: 12, color: "#ccc" }}>No selection</div>
);

const InfoPanel = () => (
  <div style={{ padding: 10, fontSize: 12, color: "#ccc" }}>
    Codec: AV1 &nbsp;|&nbsp; 1920x1080 &nbsp;|&nbsp; 250 frames
  </div>
);

const DetailsPanel = () => (
  <div style={{ padding: 10, fontSize: 12, color: "#ccc" }}>
    Frame #86 · P-frame · QP 96 · 184220 B
  </div>
);

const StatsPanel = () => (
  <div style={{ padding: 10, fontSize: 12, color: "#ccc" }}>
    Avg bitrate: 4.8 Mbps &nbsp;|&nbsp; Key frames: 8
  </div>
);

const FilmstripBar = () => (
  <div
    style={{
      display: "flex",
      gap: 4,
      padding: 6,
      height: "100%",
      alignItems: "center",
    }}
  >
    {Array.from({ length: 24 }, (_, i) => (
      <div
        key={i}
        style={{
          width: 32,
          height: 40,
          flexShrink: 0,
          background: i % 8 === 0 ? "#4a7a4a" : i % 3 === 0 ? "#4a5a7a" : "#3a3a3a",
          border: "1px solid #555",
          borderRadius: 2,
        }}
      />
    ))}
  </div>
);

const MainViewer = () => (
  <div
    style={{
      height: "100%",
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      color: "#888",
      fontSize: 13,
    }}
  >
    YUV Viewer — Frame #86 (1920×1080, AV1)
  </div>
);

export const Default = () => (
  <div style={layoutWrapper}>
    <DockableLayout
      pinnedLeftPanel={{
        id: "stream-tree",
        title: "Stream Tree",
        component: StreamTreePanel,
        icon: "list-tree",
      }}
      leftPanels={[
        { id: "syntax", title: "Syntax", component: SyntaxPanel, icon: "symbol-structure" },
        { id: "selection", title: "Selection", component: SelectionPanel, icon: "target" },
      ]}
      mainView={MainViewer}
      topPanels={[{ id: "filmstrip", title: "Filmstrip", component: FilmstripBar }]}
      bottomRowPanels={[
        { id: "info", title: "Info", component: InfoPanel, icon: "info" },
        { id: "details", title: "Details", component: DetailsPanel, icon: "list-flat" },
        { id: "stats", title: "Stats", component: StatsPanel, icon: "graph" },
      ]}
    />
  </div>
);

// No pinned left panel / no top filmstrip bar -- exercises the plain tabbed-list left sidebar
// branch (LeftSidebar without a pinnedPanel) instead of the pinned+dropdown-inspectors branch.
export const WithoutPinnedPanel = () => (
  <div style={layoutWrapper}>
    <DockableLayout
      leftPanels={[
        { id: "syntax", title: "Syntax", component: SyntaxPanel, icon: "symbol-structure" },
        { id: "selection", title: "Selection", component: SelectionPanel, icon: "target" },
        { id: "info", title: "Info", component: InfoPanel, icon: "info" },
      ]}
      mainView={MainViewer}
      bottomRowPanels={[
        { id: "details", title: "Details", component: DetailsPanel, icon: "list-flat" },
        { id: "stats", title: "Stats", component: StatsPanel, icon: "graph" },
      ]}
    />
  </div>
);
