import { TabsWithContent } from "bitvue";
import type { CSSProperties } from "react";

const previewBg: CSSProperties = { background: "#1e1e1e", padding: 24 };
const row: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  padding: "4px 0",
  fontSize: 12,
  color: "rgba(255,255,255,0.85)",
};

export const CodecInfoTabs = () => (
  <div style={previewBg}>
    <TabsWithContent
      tabs={[
        {
          id: "refs",
          label: "References",
          badge: 3,
          content: (
            <div style={row}>
              <span>L0</span>
              <span>frame 84, 83, 80</span>
            </div>
          ),
        },
        {
          id: "stats",
          label: "Statistics",
          content: (
            <>
              <div style={row}>
                <span>Total OBUs</span>
                <span>412</span>
              </div>
              <div style={row}>
                <span>Avg QP</span>
                <span>28.4</span>
              </div>
            </>
          ),
        },
      ]}
      activeTab="refs"
      onTabChange={() => {}}
    />
  </div>
);

export const PillsWithDisabledTab = () => (
  <div style={previewBg}>
    <TabsWithContent
      tabs={[
        {
          id: "hevc",
          label: "HEVC",
          content: (
            <div style={row}>
              <span>Profile</span>
              <span>Main 10</span>
            </div>
          ),
        },
        {
          id: "vvc",
          label: "VVC",
          disabled: true,
          content: (
            <div style={row}>
              <span>Profile</span>
              <span>n/a</span>
            </div>
          ),
        },
      ]}
      activeTab="hevc"
      onTabChange={() => {}}
      variant="pills"
    />
  </div>
);
