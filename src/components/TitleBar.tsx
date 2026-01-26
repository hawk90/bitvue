/**
 * TitleBar - VQAnalyzer Style Menu Bar
 *
 * Complete menu implementation matching egui reference with full submenu structure.
 *
 * This component is shown on Windows/Linux only. On macOS, the native
 * system menu is used instead (see utils/menu/setup.ts).
 */

import { useState, memo, useCallback } from 'react';
import './TitleBar.css';

interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  action?: () => void;  // Optional for submenus
  separator?: boolean;
  items?: MenuItem[]; // For submenus
}

interface MenuConfig {
  id: string;
  label: string;
  items: MenuItem[];
}

interface TitleBarProps {
  fileName: string;
  onOpenFile: () => void;
}

export const TitleBar = memo(function TitleBar({ onOpenFile }: TitleBarProps) {
  const [activeMenu, setActiveMenu] = useState<string | null>(null);
  const [activeSubmenu, setActiveSubmenu] = useState<string | null>(null);

  const menuItems: MenuConfig[] = [
    {
      id: 'file',
      label: 'File',
      items: [
        { id: 'open', label: 'Open bitstream...', shortcut: 'Ctrl+O', action: onOpenFile },
        {
          id: 'open-as',
          label: 'Open bitstream as...',
          items: [
            { id: 'open-av1', label: 'AV1', action: () => {} },
            { id: 'open-hevc', label: 'HEVC', action: () => {} },
            { id: 'open-avc', label: 'AVC/H.264', action: () => {} },
            { id: 'open-vp9', label: 'VP9', action: () => {} },
            { id: 'open-vvc', label: 'VVC/H.266', action: () => {} },
            { id: 'open-mpeg2', label: 'MPEG-2', action: () => {} },
          ]
        },
        { id: 'open-dependent', label: 'Open dependent bitstream...', action: () => {} },
        { id: 'sep1', label: '', separator: true, action: () => {} },
        { id: 'close', label: 'Close bitstream', shortcut: 'Ctrl+W', action: () => {} },
        { id: 'sep2', label: '', separator: true, action: () => {} },
        {
          id: 'extract',
          label: 'Extract...',
          items: [
            { id: 'extract-yuv', label: 'YUV frames', action: () => {} },
            { id: 'extract-pred', label: 'Prediction frames', action: () => {} },
            { id: 'extract-recon', label: 'Reconstruction frames', action: () => {} },
            { id: 'extract-transform', label: 'Transform coefficients', action: () => {} },
          ]
        },
        { id: 'sep3', label: '', separator: true, action: () => {} },
        { id: 'recent', label: 'Recent Files', action: () => {} },
        { id: 'sep4', label: '', separator: true, action: () => {} },
        { id: 'quit', label: 'Quit', shortcut: 'Ctrl+Q', action: () => {} },
      ]
    },
    {
      id: 'mode',
      label: 'Mode',
      items: [
        { id: 'overview', label: 'Overview', shortcut: 'F1', action: () => {} },
        { id: 'coding', label: 'Coding Flow', shortcut: 'F2', action: () => {} },
        { id: 'prediction', label: 'Prediction', shortcut: 'F3', action: () => {} },
        { id: 'transform', label: 'Transform', shortcut: 'F4', action: () => {} },
        { id: 'qp', label: 'QP Map', shortcut: 'F5', action: () => {} },
        { id: 'mv', label: 'MV Field', shortcut: 'F6', action: () => {} },
        { id: 'reference', label: 'Reference Frames', shortcut: 'F7', action: () => {} },
        { id: 'sep1', label: '', separator: true, action: () => {} },
        { id: 'ext-modes', label: 'Extended Modes', action: () => {} },
      ]
    },
    {
      id: 'yuvdiff',
      label: 'YUVDiff',
      items: [
        { id: 'open-debug', label: 'Open debug YUV...', shortcut: 'Ctrl+Y', action: () => {} },
        { id: 'recent-yuv', label: 'Recent YUV files', action: () => {} },
        { id: 'close-debug', label: 'Close debug YUV', action: () => {} },
        { id: 'sep1', label: '', separator: true, action: () => {} },
        {
          id: 'subsampling',
          label: 'Subsampling',
          items: [
            { id: 'subsampling-planar', label: 'Planar (YUV420)', action: () => {} },
            { id: 'subsampling-interleaved', label: 'Interleaved (YUV422)', action: () => {} },
          ]
        },
        { id: 'display-order', label: 'Display order', action: () => {} },
        { id: 'decode-order', label: 'Decode order', action: () => {} },
        { id: 'sep2', label: '', separator: true, action: () => {} },
        { id: 'stream-crop', label: 'Use stream crop values', action: () => {} },
        { id: 'picture-offset', label: 'Set picture offset here', action: () => {} },
        { id: 'sep3', label: '', separator: true, action: () => {} },
        { id: 'bitdepth-stream', label: 'Use stream bitdepth', action: () => {} },
        { id: 'bitdepth-max', label: 'Use max stream bitdepth', action: () => {} },
        { id: 'bitdepth-16', label: 'Use 16 bit bitdepth', action: () => {} },
        { id: 'sep4', label: '', separator: true, action: () => {} },
        { id: 'check-file-changes', label: 'Check for file changes', action: () => {} },
        { id: 'sep5', label: '', separator: true, action: () => {} },
        { id: 'show-psnr', label: 'Show PSNR Map', action: () => {} },
        { id: 'show-ssim', label: 'Show SSIM Map', action: () => {} },
        { id: 'show-delta', label: 'Show Delta Image', action: () => {} },
        { id: 'sep6', label: '', separator: true, action: () => {} },
        { id: 'export-all-frames', label: 'Export All Frames', action: () => {} },
        { id: 'export-metrics', label: 'Export Metrics CSV...', action: () => {} },
      ]
    },
    {
      id: 'options',
      label: 'Options',
      items: [
        {
          id: 'color-space',
          label: 'Color Space',
          items: [
            { id: 'color-bt601', label: 'ITU Rec. 601', action: () => {} },
            { id: 'color-bt709', label: 'ITU Rec. 709', action: () => {} },
            { id: 'color-bt2020', label: 'ITU Rec. 2020', action: () => {} },
            { id: 'sep1', label: '', separator: true, action: () => {} },
            { id: 'color-yuv-rgb', label: 'YUV as RGB', action: () => {} },
            { id: 'color-yuv-gbr', label: 'YUV as GBR', action: () => {} },
          ]
        },
        {
          id: 'cpu-perf',
          label: 'CPU & Performance',
          items: [
            { id: 'cpu-avx2', label: 'Enable CPU optimizations [avx2]', action: () => {} },
            { id: 'sep1', label: '', separator: true, action: () => {} },
            { id: 'loop-playback', label: 'Loop playback', action: () => {} },
          ]
        },
        {
          id: 'codec-settings',
          label: 'Codec Settings',
          items: [
            { id: 'codec-hevc-ext', label: 'HEVC: Enable extensions', action: () => {} },
            { id: 'codec-hevc-index', label: 'HEVC: Enable stream index', action: () => {} },
            { id: 'codec-hevc-mv', label: 'HEVC: Show only visible CTB MV', action: () => {} },
            { id: 'sep1', label: '', separator: true, action: () => {} },
            { id: 'codec-vvc-dynamic', label: 'VVC: Dynamic selection info', action: () => {} },
            { id: 'codec-vvc-details', label: 'VVC: Details popup window', action: () => {} },
            { id: 'sep2', label: '', separator: true, action: () => {} },
            { id: 'digest-force', label: 'Digest: Force digest', action: () => {} },
            { id: 'digest-none', label: 'Digest: No digest', action: () => {} },
            { id: 'digest-stream', label: 'Digest: As in bitstream', action: () => {} },
          ]
        },
        { id: 'sep1', label: '', separator: true, action: () => {} },
        { id: 'theme-dark', label: 'Dark Theme', action: () => {} },
        { id: 'theme-light', label: 'Light Theme', action: () => {} },
        { id: 'sep2', label: '', separator: true, action: () => {} },
        { id: 'save-layout', label: 'Save Layout...', action: () => {} },
        { id: 'load-layout', label: 'Load Layout...', action: () => {} },
        { id: 'reset-layout', label: 'Reset Layout', action: () => {} },
        { id: 'auto-save-layout', label: 'Auto-save on exit', action: () => {} },
      ]
    },
    {
      id: 'export',
      label: 'Export',
      items: [
        {
          id: 'export-data',
          label: 'Data Export',
          items: [
            { id: 'export-frame-sizes', label: 'Frame Sizes (CSV)', action: () => {} },
            { id: 'export-unit-tree', label: 'Unit Tree (JSON)', action: () => {} },
            { id: 'export-syntax-tree', label: 'Syntax Tree (JSON)', action: () => {} },
          ]
        },
        { id: 'sep1', label: '', separator: true, action: () => {} },
        { id: 'export-evidence', label: 'Evidence Bundle...', action: () => {} },
      ]
    },
    {
      id: 'view',
      label: 'View',
      items: [
        { id: 'reset-layout', label: 'Reset Layout', action: () => {} },
        { id: 'sep1', label: '', separator: true, action: () => {} },
        { id: 'show-stream-tree', label: 'Stream Tree', action: () => {} },
        { id: 'show-player', label: 'Player', action: () => {} },
        { id: 'show-diagnostics', label: 'Diagnostics', action: () => {} },
      ]
    },
    {
      id: 'help',
      label: 'Help',
      items: [
        { id: 'docs', label: 'Documentation', action: () => {} },
        { id: 'shortcuts', label: 'Keyboard Shortcuts', action: () => {} },
        { id: 'sep1', label: '', separator: true, action: () => {} },
        { id: 'about', label: 'About Bitvue', action: () => {} },
      ]
    },
  ];

  const handleMenuEnter = useCallback((menuId: string) => {
    setActiveMenu(menuId);
  }, []);

  const handleMenuLeave = useCallback(() => {
    setActiveMenu(null);
    setActiveSubmenu(null);
  }, []);

  const handleSubmenuEnter = useCallback((itemId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setActiveSubmenu(itemId);
  }, []);

  const handleSubmenuLeave = useCallback(() => {
    setActiveSubmenu(null);
  }, []);

  const handleMenuItemClick = useCallback((item: MenuItem, isSubmenu: boolean) => {
    if (item.action) {
      item.action();
    }
    if (!isSubmenu) {
      setActiveMenu(null);
    }
    setActiveSubmenu(null);
  }, []);

  const renderMenuItems = (items: MenuItem[], isSubmenu = false) => {
    return items.map((item) => {
      if (item.separator) {
        return <div key={item.id} className="menu-separator" />;
      }

      // Submenu (has nested items)
      if (item.items && item.items.length > 0) {
        return (
          <div
            key={item.id}
            className="menu-item menu-item-has-submenu"
            onMouseEnter={(e) => handleSubmenuEnter(item.id, e)}
            onMouseLeave={handleSubmenuLeave}
          >
            <span className="menu-item-label">{item.label}</span>
            <span className="menu-item-arrow">▶</span>

            {activeSubmenu === item.id && (
              <div className="menu-submenu">
                {renderMenuItems(item.items, true)}
              </div>
            )}
          </div>
        );
      }

      // Regular menu item
      return (
        <div
          key={item.id}
          className="menu-item"
          onClick={() => handleMenuItemClick(item, isSubmenu)}
        >
          <span className="menu-item-label">{item.label}</span>
          {item.shortcut && (
            <span className="menu-item-shortcut">{item.shortcut}</span>
          )}
        </div>
      );
    });
  };

  return (
    <div className="title-bar">
      {/* Left side: Menus */}
      <div className="title-bar-menus">
        {menuItems.map(menu => (
          <div
            key={menu.id}
            className="title-bar-menu"
            onMouseEnter={() => handleMenuEnter(menu.id)}
            onMouseLeave={handleMenuLeave}
          >
            <span className="menu-label">{menu.label}</span>

            {activeMenu === menu.id && (
              <div className="menu-dropdown">
                {renderMenuItems(menu.items)}
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Right side: Window controls (Windows style) */}
      <div className="title-bar-controls">
        <button className="title-bar-button" title="Minimize">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <rect x="0" y="5" width="12" height="2" fill="currentColor"/>
          </svg>
        </button>
        <button className="title-bar-button" title="Maximize">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <rect x="1" y="1" width="10" height="10" fill="none" stroke="currentColor" strokeWidth="1"/>
          </svg>
        </button>
        <button className="title-bar-button title-bar-close" title="Close">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" strokeWidth="1.5"/>
          </svg>
        </button>
      </div>
    </div>
  );
});
