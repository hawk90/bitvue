import { PanelBase, PanelInfoRow } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = {
  background: "#1e1e1e",
  padding: 24,
  height: 360,
};

export const Statistics = () => (
  <div style={previewBg}>
    <PanelBase title="Statistics" icon="codicon-graph">
      <PanelInfoRow label="Codec" value="AV1" />
      <PanelInfoRow label="Resolution" value="1920x1080" />
      <PanelInfoRow label="Frame Rate" value="29.97 fps" />
      <PanelInfoRow label="Total Frames" value="1,482" />
    </PanelBase>
  </div>
);

export const WithFooter = () => (
  <div style={previewBg}>
    <PanelBase
      title="Bookmarks"
      icon="codicon-bookmark"
      footer={<span style={{ fontSize: 11, opacity: 0.7 }}>3 bookmarks in sample.ivf</span>}
      onClose={() => {}}
    >
      <PanelInfoRow label="Frame 12" value="Scene change" />
      <PanelInfoRow label="Frame 87" value="Quality drop" />
      <PanelInfoRow label="Frame 204" value="Keyframe" />
    </PanelBase>
  </div>
);

export const NoCloseButton = () => (
  <div style={previewBg}>
    <PanelBase title="Details" icon="codicon-info" showCloseButton={false}>
      <PanelInfoRow label="Frame Index" value="204" />
      <PanelInfoRow label="Frame Type" value="KEY" />
      <PanelInfoRow label="Size" value="184.2 KB" />
    </PanelBase>
  </div>
);
