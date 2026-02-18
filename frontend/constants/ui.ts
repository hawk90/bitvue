/**
 * UI Constants
 *
 * Centralized magic numbers and configuration values
 */

// ==================== Thumbnail Display ====================

export const THUMBNAIL_ZOOM = {
  MIN: 0.5,
  MAX: 3,
  STEP: 0.1,
  DEFAULT: 1,
} as const;

export const THUMBNAIL_SIZE = {
  WIDTH: 120,
  HEIGHT: 68, // 16:9 aspect ratio
  MIN_WIDTH: 60,
  MAX_WIDTH: 240,
} as const;

export const THUMBNAIL_BATCH_SIZE = 50;

// ==================== Filmstrip Layout ====================

export const FILMSTRIP = {
  MIN_WIDTH: 800,
  DEFAULT_HEIGHT: 140,
  SCROLL_THRESHOLD: 0.8,
  INTERSECTION_ROOT_MARGIN: "200px",
  VIRTUAL_DEFAULT_OVERSCAN: 5, // Number of extra items to render outside viewport
} as const;

export const FILMSTRIP_EMPTY_HEIGHT = 140;

// ==================== Arrow Rendering ====================

export const ARROW = {
  BASE_OFFSET: 30, // Base vertical offset from source frame
  SPACING_PER_SLOT: 12, // Vertical spacing between multiple arrows
  MIN_LENGTH: 20, // Minimum arrow length in pixels
  OPACITY: 0.8, // Default arrow opacity
  HIGHLIGHT_OPACITY: 1, // Opacity when highlighted
  WIDTH: 1.5, // SVG stroke width
} as const;

// ==================== Frame Display ====================

export const FRAME = {
  I_COLOR: "var(--frame-i)",
  P_COLOR: "var(--frame-p)",
  B_COLOR: "var(--frame-b)",
  UNKNOWN_COLOR: "var(--text-tertiary)",
} as const;

export const FRAME_TYPES = {
  I: "I",
  P: "P",
  B: "B",
  KEY: "KEY",
  INTRA: "INTRA",
  INTER: "INTER",
  SWITCH: "SWITCH",
  UNKNOWN: "UNKNOWN",
} as const;

// ==================== Moving Average ====================

export const MOVING_AVERAGE_WINDOW = 21; // frames

// ==================== Timings ====================

export const TIMING = {
  THUMBNAIL_LOAD_DELAY: 100, // ms before loading next batch
  DEBOUNCE_DELAY: 150, // ms for debounce operations
  TOOLTIP_DELAY: 300, // ms before showing tooltip
  AUTO_PLAY_INTERVAL: 1000, // ms between frames at 1fps
  STORAGE_DEBOUNCE_DELAY: 500, // ms for auto-saving to localStorage
  TOOLTIP_AUTO_HIDE_DELAY: 5000, // ms before auto-hiding tooltips
} as const;

// ==================== Dimensions ====================

export const DIMENSIONS = {
  CIRCLE_SIZE: 12, // B-Pyramid frame circle diameter
  CIRCLE_GAP: 8, // Gap between circles in B-Pyramid
  MINIMAP_CELL_SIZE: 12, // Size of minimap cells
  TOOLTIP_OFFSET: 8, // Pixel offset for tooltip positioning
} as const;

// ==================== Z-Index Layers ====================

export const Z_INDEX = {
  TOOLTIP: 10000, // Interactive tooltips
  MODAL: 1000, // Modal dialogs
  DROPDOWN: 600, // Dropdown menus
  POPOVER: 500, // Popovers
} as const;

// ==================== Zoom Levels ====================

export const ZOOM = {
  MIN: 0.25,
  MAX: 4,
  STEP: 0.25,
  DEFAULT: 1,
} as const;

// ==================== Key Codes ====================

export const KEY_CODES = {
  ESCAPE: "Escape",
  SPACE: " ",
  ARROW_LEFT: "ArrowLeft",
  ARROW_RIGHT: "ArrowRight",
  ARROW_UP: "ArrowUp",
  ARROW_DOWN: "ArrowDown",
  HOME: "Home",
  END: "End",
  PLUS: "+",
  EQUAL: "=",
  MINUS: "-",
  DIGIT_0: "0",
} as const;

// ==================== File Filters ====================

export const FILE_FILTERS = [
  {
    name: "Video Files",
    extensions: [
      "ivf",
      "av1",
      "hevc",
      "h265",
      "vvc",
      "h266",
      "mp4",
      "mkv",
      "webm",
      "ts",
    ],
  },
  {
    name: "All Files",
    extensions: ["*"],
  },
] as const;

// ==================== Display Modes ====================

export const DISPLAY_MODES = {
  THUMBNAILS: "thumbnails",
  SIZES: "sizes",
  BPYRAMID: "bpyramid",
  HRDBUFFER: "hrdbuffer",
  ENHANCED: "enhanced",
  MINIMAP: "minimap",
} as const;

// ==================== Statistics ====================

export const STATISTICS = {
  BAR_MIN_HEIGHT: 1, // Minimum bar height in percentage
  BAR_CHART_HEIGHT: 40, // Bar chart height in pixels
} as const;
