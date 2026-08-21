/**
 * Syntax Detail Panel
 *
 * Main container for syntax information tabs
 * Features:
 * - Frame syntax display
 * - Reference frame information (RefListTab)
 * - Frame statistics with intra/inter ratio and QP distribution
 * - Search functionality
 * - Codec-specific tabs: QM (HEVC), Probs (VP9), APS (VVC)
 */

import { useState, useCallback, useMemo, memo } from "react";
import { useFrameData } from "../../../contexts/FrameDataContext";
import { useCurrentFrame } from "../../../contexts/SelectionContext";
import { useFileState } from "../../../contexts/FileStateContext";
import { FrameSyntaxTab } from "./FrameSyntaxTab";
import { StatisticsTab } from "./StatisticsTab";
import { SearchTab } from "./SearchTab";
import { QmTab } from "./QmTab";
import { ProbsTab } from "./ProbsTab";
import { ApsTab } from "./ApsTab";
import { RefListTab } from "./RefListTab";
import "../SyntaxDetailPanel.css";

// ─── Types ────────────────────────────────────────────────────────────────────

type SyntaxTab = "Frame" | "Refs" | "Stats" | "Search" | "QM" | "Probs" | "APS";

// ─── Codec detection ──────────────────────────────────────────────────────────

function detectCodecFromPath(path: string | null): string {
  if (!path) return "unknown";
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  switch (ext) {
    case "ivf":
    case "av1":
      return "av1";
    case "webm":
    case "vp9":
      return "vp9";
    case "h264":
    case "264":
    case "avc":
      return "avc";
    case "h265":
    case "265":
    case "hevc":
      return "hevc";
    case "h266":
    case "266":
    case "vvc":
      return "vvc";
    case "av3":
      return "av3";
    default:
      return "unknown";
  }
}

// ─── Component ────────────────────────────────────────────────────────────────

export const SyntaxDetailPanel = memo(function SyntaxDetailPanel() {
  const { frames } = useFrameData();
  const { currentFrameIndex, setCurrentFrameIndex } = useCurrentFrame();
  const { filePath } = useFileState();
  const [currentTab, setCurrentTab] = useState<SyntaxTab>("Frame");
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<number[]>([]);

  const currentFrame = frames[currentFrameIndex] || null;

  const codec = detectCodecFromPath(filePath ?? null);

  const SYNTAX_TABS = useMemo<
    { value: SyntaxTab; label: string; icon: string }[]
  >(() => {
    const tabs: { value: SyntaxTab; label: string; icon: string }[] = [
      { value: "Frame", label: "Frame", icon: "file" },
      { value: "Refs", label: "Refs", icon: "list-tree" },
      { value: "Stats", label: "Stats", icon: "graph" },
      { value: "Search", label: "Search", icon: "search" },
    ];
    if (codec === "hevc")
      tabs.push({ value: "QM", label: "QM", icon: "symbol-array" });
    if (codec === "vp9")
      tabs.push({ value: "Probs", label: "Probs", icon: "table" });
    if (codec === "vvc")
      tabs.push({ value: "APS", label: "APS", icon: "settings-gear" });
    return tabs;
  }, [codec]);

  const handleTabChange = useCallback((tab: SyntaxTab) => {
    setCurrentTab(tab);
  }, []);

  // Toggle node expansion
  const toggleNode = useCallback((nodePath: string) => {
    setExpandedNodes((prev) => {
      const next = new Set(prev);
      if (next.has(nodePath)) {
        next.delete(nodePath);
      } else {
        next.add(nodePath);
      }
      return next;
    });
  }, []);

  // Search functionality
  const handleSearch = useCallback(
    (query: string) => {
      setSearchQuery(query);

      if (!query.trim()) {
        setSearchResults([]);
        return;
      }

      const lowerQuery = query.toLowerCase();
      const results: number[] = [];

      frames.forEach((frame, idx) => {
        const matchByType = frame.frame_type.toLowerCase().includes(lowerQuery);
        const matchByIndex = String(frame.frame_index).includes(lowerQuery);
        const matchByPts = String(frame.pts ?? "").includes(lowerQuery);

        if (matchByType || matchByIndex || matchByPts) {
          results.push(idx);
        }
      });

      setSearchResults(results);
    },
    [frames],
  );

  const handleClearSearch = useCallback(() => {
    setSearchQuery("");
    setSearchResults([]);
  }, []);

  const renderTabContent = () => {
    switch (currentTab) {
      case "Frame":
        return (
          <FrameSyntaxTab
            frame={currentFrame}
            expandedNodes={expandedNodes}
            onToggleNode={toggleNode}
            filePath={filePath ?? undefined}
          />
        );
      case "Refs":
        return (
          <RefListTab
            filePath={filePath ?? undefined}
            frameIndex={currentFrameIndex}
            frameType={currentFrame?.frame_type}
          />
        );
      case "Stats":
        return (
          <StatisticsTab
            currentFrame={currentFrame}
            frames={frames}
            filePath={filePath ?? undefined}
            frameIndex={currentFrameIndex}
          />
        );
      case "Search":
        return (
          <SearchTab
            frames={frames}
            currentFrameIndex={currentFrameIndex}
            searchQuery={searchQuery}
            searchResults={searchResults}
            onSearchChange={handleSearch}
            onClearSearch={handleClearSearch}
            onNavigateToFrame={setCurrentFrameIndex}
          />
        );
      case "QM":
        return (
          <QmTab
            filePath={filePath ?? undefined}
            frameIndex={currentFrameIndex}
          />
        );
      case "Probs":
        return (
          <ProbsTab
            filePath={filePath ?? undefined}
            frameIndex={currentFrameIndex}
          />
        );
      case "APS":
        return (
          <ApsTab
            filePath={filePath ?? undefined}
            frameIndex={currentFrameIndex}
          />
        );
    }
  };

  return (
    <div className="syntax-detail-panel">
      <div className="panel-header">
        <span className="panel-title">Syntax Detail</span>
      </div>

      <div className="syntax-panel-body">
        {/* Tab bar - vertical on the left */}
        <div className="syntax-tabs">
          {SYNTAX_TABS.map((tab) => (
            <button
              key={tab.value}
              className={`syntax-tab ${currentTab === tab.value ? "active" : ""}`}
              onClick={() => handleTabChange(tab.value)}
              title={tab.label}
            >
              <span className={`codicon codicon-${tab.icon}`}></span>
              <span className="syntax-tab-label">{tab.label}</span>
            </button>
          ))}
        </div>

        <div className="syntax-content-wrapper">
          {/* Tab content */}
          <div className="syntax-content">{renderTabContent()}</div>
        </div>
      </div>
    </div>
  );
});
