import { TabContainer } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };

export const Default = () => (
  <div style={previewBg}>
    <TabContainer
      tabs={[
        { id: "hex", label: "Hex", icon: "codicon-symbol-numeric" },
        { id: "syntax", label: "Syntax", icon: "codicon-list-tree" },
        { id: "frame", label: "Frame", icon: "codicon-symbol-field" },
      ]}
      activeTab="syntax"
      onTabChange={() => {}}
    />
  </div>
);

export const WithBadgesAndDisabled = () => (
  <div style={previewBg}>
    <TabContainer
      tabs={[
        { id: "refs", label: "References", badge: 7 },
        { id: "probs", label: "Probabilities", badge: 128 },
        { id: "qm", label: "Quant Matrix", disabled: true },
      ]}
      activeTab="refs"
      onTabChange={() => {}}
      showIcons={false}
    />
  </div>
);

export const PillsVariant = () => (
  <div style={previewBg}>
    <TabContainer
      tabs={[
        { id: "overview", label: "Overview" },
        { id: "deblocking", label: "Deblocking" },
        { id: "residual", label: "Residual" },
      ]}
      activeTab="deblocking"
      onTabChange={() => {}}
      variant="pills"
      showIcons={false}
    />
  </div>
);

export const CompactLeftPosition = () => (
  <div style={previewBg}>
    <TabContainer
      tabs={[
        { id: "aps", label: "APS" },
        { id: "dpb", label: "DPB" },
      ]}
      activeTab="aps"
      onTabChange={() => {}}
      variant="compact"
      position="left"
      showIcons={false}
    />
  </div>
);
