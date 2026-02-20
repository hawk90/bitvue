/**
 * Syntax Detail Panel
 *
 * Main container for syntax information tabs
 * Features:
 * - Frame syntax display
 * - Reference frame information
 * - Frame statistics
 * - Search functionality
 */

import { useState, useCallback, memo } from "react";
import { useFrameData } from "../../../contexts/FrameDataContext";
import { useCurrentFrame } from "../../../contexts/CurrentFrameContext";
import { useFileState } from "../../../contexts/FileStateContext";
import { FrameSyntaxTab } from "./FrameSyntaxTab";
import { ReferencesTab } from "./ReferencesTab";
import { StatisticsTab } from "./StatisticsTab";
import { SearchTab } from "./SearchTab";
import "../SyntaxDetailPanel.css";

type SyntaxTab = "Frame" | "Refs" | "Stats" | "Search";

const SYNTAX_TABS: { value: SyntaxTab; label: string; icon: string }[] = [
  { value: "Frame", label: "Frame", icon: "file" },
  { value: "Refs", label: "Refs", icon: "database" },
  { value: "Stats", label: "Stats", icon: "graph" },
  { value: "Search", label: "Search", icon: "search" },
];

export const SyntaxDetailPanel = memo(function SyntaxDetailPanel() {
  const { frames } = useFrameData();
  const { currentFrameIndex } = useCurrentFrame();
  const { filePath } = useFileState();
  const [currentTab, setCurrentTab] = useState<SyntaxTab>("Frame");
  const [expandedNodes, setExpandedNodes] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<number[]>([]);

  const currentFrame = frames[currentFrameIndex] || null;

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
        return <ReferencesTab currentFrame={currentFrame} frames={frames} />;
      case "Stats":
        return <StatisticsTab currentFrame={currentFrame} frames={frames} />;
      case "Search":
        return (
          <SearchTab
            frames={frames}
            currentFrameIndex={currentFrameIndex}
            searchQuery={searchQuery}
            searchResults={searchResults}
            onSearchChange={handleSearch}
            onClearSearch={handleClearSearch}
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
