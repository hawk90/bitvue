import { TabContent } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };
const row: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  padding: "4px 0",
  fontSize: 12,
  color: "rgba(255,255,255,0.85)",
};

export const ActiveHexPanel = () => (
  <div style={previewBg}>
    <TabContent tabId="hex" isActive>
      <div style={row}>
        <span>Offset</span>
        <span>0x0001A4</span>
      </div>
      <div style={row}>
        <span>Bytes</span>
        <span>02 4B 8F 00 96 3A 71 D2</span>
      </div>
    </TabContent>
  </div>
);

export const ActiveSyntaxPanel = () => (
  <div style={previewBg}>
    <TabContent tabId="syntax" isActive className="custom-syntax-content">
      <div style={row}>
        <span>frame_type</span>
        <span>INTER_FRAME</span>
      </div>
      <div style={row}>
        <span>base_q_idx</span>
        <span>96</span>
      </div>
      <div style={row}>
        <span>refresh_frame_flags</span>
        <span>0b00010010</span>
      </div>
    </TabContent>
  </div>
);

export const Inactive = () => (
  <div style={previewBg}>
    <p style={{ color: "rgba(255,255,255,0.5)", fontSize: 12 }}>
      isActive=false — TabContent renders null (nothing shown below this line
      on purpose):
    </p>
    <TabContent tabId="qm" isActive={false}>
      <div style={row}>
        <span>Quant Matrix</span>
        <span>flat</span>
      </div>
    </TabContent>
  </div>
);
