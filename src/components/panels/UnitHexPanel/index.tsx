/**
 * Unit HEX Panel
 *
 * Main container for hex dump visualization with tabbed interface
 * Features:
 * - Frame info display
 * - Hex dump with highlighting
 * - DPB state visualization
 */

import { useState, memo, useCallback } from 'react';
import { useFrameData } from '../../../contexts/FrameDataContext';
import { useCurrentFrame } from '../../../contexts/CurrentFrameContext';
import { FrameViewTab } from './FrameViewTab';
import { HexViewTab } from './HexViewTab';
import { DpbViewTab } from './DpbViewTab';
import '../UnitHexPanel.css';

type HexViewTab = 'Frame' | 'Hex' | 'DpbInfo';

const HEX_VIEW_TABS: { value: HexViewTab; label: string; icon: string }[] = [
  { value: 'Frame', label: 'Frame', icon: 'file' },
  { value: 'Hex', label: 'Hex', icon: 'file-code' },
  { value: 'DpbInfo', label: 'DPB', icon: 'database' },
];

export const UnitHexPanel = memo(function UnitHexPanel() {
  const { frames } = useFrameData();
  const { currentFrameIndex } = useCurrentFrame();
  const [currentTab, setCurrentTab] = useState<HexViewTab>('Frame');

  const currentFrame = frames[currentFrameIndex] || null;

  const handleTabChange = useCallback((tab: HexViewTab) => {
    setCurrentTab(tab);
  }, []);

  const renderTabContent = () => {
    switch (currentTab) {
      case 'Frame':
        return <FrameViewTab frame={currentFrame} />;
      case 'Hex':
        return <HexViewTab frameIndex={currentFrameIndex} frames={frames} />;
      case 'DpbInfo':
        return <DpbViewTab currentFrame={currentFrame} frames={frames} />;
    }
  };

  return (
    <div className="unit-hex-panel">
      <div className="panel-header">
        <span className="panel-title">Unit HEX</span>
      </div>

      {/* Tab bar */}
      <div className="hex-tabs">
        {HEX_VIEW_TABS.map(tab => (
          <button
            key={tab.value}
            className={`hex-tab ${currentTab === tab.value ? 'active' : ''}`}
            onClick={() => handleTabChange(tab.value)}
            title={tab.label}
          >
            <span className={`codicon codicon-${tab.icon}`}></span>
            <span className="hex-tab-label">{tab.label}</span>
          </button>
        ))}
      </div>

      <div className="panel-divider"></div>

      {/* Tab content */}
      <div className="hex-content">
        {renderTabContent()}
      </div>
    </div>
  );
});

export default UnitHexPanel;
