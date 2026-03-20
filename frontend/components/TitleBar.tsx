/**
 * TitleBar - Style Menu Bar
 *
 * Complete menu implementation matching egui reference with full submenu structure.
 *
 * This component is shown on Windows/Linux only. On macOS, the native
 * system menu is used instead (see utils/menu/setup.ts).
 */

import { useState, memo, useCallback, useMemo } from "react";
import { MODES, type VisualizationMode } from "../contexts/ModeContext";
import "./TitleBar.css";

interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  /** Explicitly mark as disabled; if omitted, items without action are auto-disabled */
  disabled?: boolean;
  action?: () => void;
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
  onOpenDependentFile?: () => void;
  onCloseFile?: () => void;
  onQuit?: () => void;
  onShowShortcuts?: () => void;
  onModeChange?: (mode: VisualizationMode) => void;
}

export const TitleBar = memo(function TitleBar({
  onOpenFile,
  onOpenDependentFile,
  onCloseFile,
  onQuit,
  onShowShortcuts,
  onModeChange,
}: TitleBarProps) {
  const [activeMenu, setActiveMenu] = useState<string | null>(null);
  const [activeSubmenu, setActiveSubmenu] = useState<string | null>(null);

  const menuItems: MenuConfig[] = useMemo(
    () => [
      {
        id: "file",
        label: "File",
        items: [
          {
            id: "open",
            label: "Open bitstream...",
            shortcut: "Ctrl+O",
            action: onOpenFile,
          },
          {
            id: "open-as",
            label: "Open bitstream as...",
            items: [
              { id: "open-av1", label: "AV1" },
              { id: "open-hevc", label: "HEVC" },
              { id: "open-avc", label: "AVC/H.264" },
              { id: "open-vp9", label: "VP9" },
              { id: "open-vvc", label: "VVC/H.266" },
              { id: "open-mpeg2", label: "MPEG-2" },
            ],
          },
          {
            id: "open-dependent",
            label: "Open dependent bitstream...",
            action: onOpenDependentFile,
          },
          { id: "sep1", label: "", separator: true },
          {
            id: "close",
            label: "Close bitstream",
            shortcut: "Ctrl+W",
            action: onCloseFile,
          },
          { id: "sep2", label: "", separator: true },
          {
            id: "extract",
            label: "Extract...",
            items: [
              { id: "extract-yuv", label: "YUV frames" },
              { id: "extract-pred", label: "Prediction frames" },
              { id: "extract-recon", label: "Reconstruction frames" },
              { id: "extract-transform", label: "Transform coefficients" },
            ],
          },
          { id: "sep3", label: "", separator: true },
          { id: "recent", label: "Recent Files" },
          { id: "sep4", label: "", separator: true },
          { id: "quit", label: "Quit", shortcut: "Ctrl+Q", action: onQuit },
        ],
      },
      {
        id: "mode",
        label: "Mode",
        items: [
          // Generated from MODES — uses correct VisualizationMode keys (e.g. "coding-flow", "qp-map")
          ...MODES.map((mode) => ({
            id: mode.key,
            label: mode.label,
            shortcut: mode.shortcut,
            action: () => onModeChange?.(mode.key),
          })),
          { id: "sep-modes", label: "", separator: true },
          { id: "ext-modes", label: "Extended Modes" },
        ],
      },
      {
        id: "yuvdiff",
        label: "YUVDiff",
        items: [
          { id: "open-debug", label: "Open debug YUV...", shortcut: "Ctrl+Y" },
          { id: "recent-yuv", label: "Recent YUV files" },
          { id: "close-debug", label: "Close debug YUV" },
          { id: "sep1", label: "", separator: true },
          {
            id: "subsampling",
            label: "Subsampling",
            items: [
              { id: "subsampling-planar", label: "Planar (YUV420)" },
              { id: "subsampling-interleaved", label: "Interleaved (YUV422)" },
            ],
          },
          { id: "display-order", label: "Display order" },
          { id: "decode-order", label: "Decode order" },
          { id: "sep2", label: "", separator: true },
          { id: "stream-crop", label: "Use stream crop values" },
          { id: "picture-offset", label: "Set picture offset here" },
          { id: "sep3", label: "", separator: true },
          { id: "bitdepth-stream", label: "Use stream bitdepth" },
          { id: "bitdepth-max", label: "Use max stream bitdepth" },
          { id: "bitdepth-16", label: "Use 16 bit bitdepth" },
          { id: "sep4", label: "", separator: true },
          { id: "check-file-changes", label: "Check for file changes" },
          { id: "sep5", label: "", separator: true },
          { id: "show-psnr", label: "Show PSNR Map" },
          { id: "show-ssim", label: "Show SSIM Map" },
          { id: "show-delta", label: "Show Delta Image" },
          { id: "sep6", label: "", separator: true },
          { id: "export-all-frames", label: "Export All Frames" },
          { id: "export-metrics", label: "Export Metrics CSV..." },
        ],
      },
      {
        id: "options",
        label: "Options",
        items: [
          {
            id: "color-space",
            label: "Color Space",
            items: [
              { id: "color-bt601", label: "ITU Rec. 601" },
              { id: "color-bt709", label: "ITU Rec. 709" },
              { id: "color-bt2020", label: "ITU Rec. 2020" },
              { id: "sep1", label: "", separator: true },
              { id: "color-yuv-rgb", label: "YUV as RGB" },
              { id: "color-yuv-gbr", label: "YUV as GBR" },
            ],
          },
          {
            id: "cpu-perf",
            label: "CPU & Performance",
            items: [
              { id: "cpu-avx2", label: "Enable CPU optimizations [avx2]" },
              { id: "sep1", label: "", separator: true },
              { id: "loop-playback", label: "Loop playback" },
            ],
          },
          {
            id: "codec-settings",
            label: "Codec Settings",
            items: [
              { id: "codec-hevc-ext", label: "HEVC: Enable extensions" },
              { id: "codec-hevc-index", label: "HEVC: Enable stream index" },
              { id: "codec-hevc-mv", label: "HEVC: Show only visible CTB MV" },
              { id: "sep1", label: "", separator: true },
              { id: "codec-vvc-dynamic", label: "VVC: Dynamic selection info" },
              { id: "codec-vvc-details", label: "VVC: Details popup window" },
              { id: "sep2", label: "", separator: true },
              { id: "digest-force", label: "Digest: Force digest" },
              { id: "digest-none", label: "Digest: No digest" },
              { id: "digest-stream", label: "Digest: As in bitstream" },
            ],
          },
          { id: "sep1", label: "", separator: true },
          { id: "theme-dark", label: "Dark Theme" },
          { id: "theme-light", label: "Light Theme" },
          { id: "sep2", label: "", separator: true },
          { id: "save-layout", label: "Save Layout..." },
          { id: "load-layout", label: "Load Layout..." },
          { id: "reset-layout", label: "Reset Layout" },
          { id: "auto-save-layout", label: "Auto-save on exit" },
        ],
      },
      {
        id: "export",
        label: "Export",
        items: [
          {
            id: "export-data",
            label: "Data Export",
            items: [
              { id: "export-frame-sizes", label: "Frame Sizes (CSV)" },
              { id: "export-unit-tree", label: "Unit Tree (JSON)" },
              { id: "export-syntax-tree", label: "Syntax Tree (JSON)" },
            ],
          },
          { id: "sep1", label: "", separator: true },
          { id: "export-evidence", label: "Evidence Bundle..." },
        ],
      },
      {
        id: "view",
        label: "View",
        items: [
          { id: "reset-layout", label: "Reset Layout" },
          { id: "sep1", label: "", separator: true },
          { id: "show-stream-tree", label: "Stream Tree" },
          { id: "show-player", label: "Player" },
          { id: "show-diagnostics", label: "Diagnostics" },
        ],
      },
      {
        id: "help",
        label: "Help",
        items: [
          { id: "docs", label: "Documentation" },
          {
            id: "shortcuts",
            label: "Keyboard Shortcuts",
            action: onShowShortcuts,
          },
          { id: "sep1", label: "", separator: true },
          { id: "about", label: "About Bitvue" },
        ],
      },
    ],
    [
      onOpenFile,
      onOpenDependentFile,
      onCloseFile,
      onQuit,
      onShowShortcuts,
      onModeChange,
    ],
  );

  const handleMenuEnter = useCallback((menuId: string) => {
    setActiveMenu(menuId);
  }, []);

  const handleMenuLeave = useCallback(() => {
    setActiveMenu(null);
    setActiveSubmenu(null);
  }, []);

  const handleSubmenuEnter = useCallback(
    (itemId: string, e: React.MouseEvent) => {
      e.stopPropagation();
      setActiveSubmenu(itemId);
    },
    [],
  );

  const handleSubmenuLeave = useCallback(() => {
    setActiveSubmenu(null);
  }, []);

  const handleMenuItemClick = useCallback(
    (item: MenuItem, isSubmenu: boolean) => {
      if (item.action) {
        item.action();
      }
      if (!isSubmenu) {
        setActiveMenu(null);
      }
      setActiveSubmenu(null);
    },
    [],
  );

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

      // Regular menu item — disabled when no action (or explicitly flagged)
      const isDisabled = item.disabled ?? !item.action;
      return (
        <div
          key={item.id}
          className={`menu-item${isDisabled ? " menu-item-disabled" : ""}`}
          onClick={
            isDisabled ? undefined : () => handleMenuItemClick(item, isSubmenu)
          }
          title={isDisabled ? "Not yet implemented" : undefined}
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
        {menuItems.map((menu) => (
          <div
            key={menu.id}
            className="title-bar-menu"
            onMouseEnter={() => handleMenuEnter(menu.id)}
            onMouseLeave={handleMenuLeave}
          >
            <span className="menu-label">{menu.label}</span>

            {activeMenu === menu.id && (
              <div className="menu-dropdown">{renderMenuItems(menu.items)}</div>
            )}
          </div>
        ))}
      </div>

      {/* Right side: Window controls (Windows style) */}
      <div className="title-bar-controls">
        <button className="title-bar-button" title="Minimize">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <rect x="0" y="5" width="12" height="2" fill="currentColor" />
          </svg>
        </button>
        <button className="title-bar-button" title="Maximize">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <rect
              x="1"
              y="1"
              width="10"
              height="10"
              fill="none"
              stroke="currentColor"
              strokeWidth="1"
            />
          </svg>
        </button>
        <button className="title-bar-button title-bar-close" title="Close">
          <svg width="12" height="12" viewBox="0 0 12 12">
            <path
              d="M2 2l8 8M10 2l-8 8"
              stroke="currentColor"
              strokeWidth="1.5"
            />
          </svg>
        </button>
      </div>
    </div>
  );
});
