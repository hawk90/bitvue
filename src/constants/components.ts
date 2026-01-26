/**
 * Component Constants
 *
 * Centralized constants for components to avoid magic strings and numbers
 */

/**
 * Display view modes for filmstrip and related components
 */
export const DISPLAY_VIEWS = {
  THUMBNAILS: 'thumbnails',
  FRAME_SIZES: 'frame-sizes',
  B_PYRAMID: 'b-pyramid',
  HRD_BUFFER: 'hrd-buffer',
  ENHANCED: 'enhanced',
} as const;

export type DisplayView = typeof DISPLAY_VIEWS[keyof typeof DISPLAY_VIEWS];

/**
 * Frame types for video analysis
 */
export const FRAME_TYPES = {
  I: 'I',
  P: 'P',
  B: 'B',
  KEY: 'KEY',
  INTRA: 'INTRA',
  INTER: 'INTER',
  UNKNOWN: 'UNKNOWN',
} as const;

export type FrameType = typeof FRAME_TYPES[keyof typeof FRAME_TYPES];

/**
 * Analysis modes for video decoder
 */
export const ANALYSIS_MODES = {
  OVERVIEW: 'overview',
  CODING_FLOW: 'coding-flow',
  PREDICTION: 'prediction',
  TRANSFORM: 'transform',
  QP_MAP: 'qp-map',
  MV_FIELD: 'mv-field',
  REFERENCE_FRAMES: 'reference-frames',
} as const;

export type AnalysisMode = typeof ANALYSIS_MODES[keyof typeof ANALYSIS_MODES];

/**
 * Panel identifiers
 */
export const PANELS = {
  SYNTAX_DETAIL: 'syntax-detail',
  UNIT_HEX: 'unit-hex',
  STREAM_TREE: 'stream-tree',
  STATISTICS: 'statistics',
  INFO: 'info',
  DETAILS: 'details',
  REFERENCE_GRAPH: 'reference-graph',
  BITRATE_GRAPH: 'bitrate-graph',
  QUALITY_METRICS: 'quality-metrics',
  BIT_VIEW: 'bit-view',
  SELECTION_INFO: 'selection-info',
  DIAGNOSTICS: 'diagnostics',
  BOOKMARKS: 'bookmarks',
  YUV_VIEWER: 'yuv-viewer',
} as const;

export type PanelId = typeof PANELS[keyof typeof PANELS];

/**
 * Tab identifiers for panels
 */
export const SYNTAX_TABS = {
  FRAME_SYNTAX: 'frame-syntax',
  REFERENCES: 'references',
  STATISTICS: 'statistics',
  SEARCH: 'search',
} as const;

export const HEX_TABS = {
  FRAME_VIEW: 'frame-view',
  HEX_VIEW: 'hex-view',
  DPB_VIEW: 'dpb-view',
} as const;

/**
 * Size metrics for frame sizes view
 */
export const SIZE_METRICS = {
  SHOW_BITRATE_BAR: 'showBitrateBar',
  SHOW_BITRATE_CURVE: 'showBitrateCurve',
  SHOW_AVG_SIZE: 'showAvgSize',
  SHOW_MIN_SIZE: 'showMinSize',
  SHOW_MAX_SIZE: 'showMaxSize',
  SHOW_MOVING_AVG: 'showMovingAvg',
  SHOW_BLOCK_MIN_QP: 'showBlockMinQP',
  SHOW_BLOCK_MAX_QP: 'showBlockMaxQP',
} as const;

/**
 * Keyboard shortcuts
 */
export const KEYBOARD_SHORTCUTS = {
  NAV_FIRST_FRAME: 'Home',
  NAV_LAST_FRAME: 'End',
  NAV_PREV_FRAME: 'ArrowLeft',
  NAV_NEXT_FRAME: 'ArrowRight',
  NAV_PREV_KEY_FRAME: 'Ctrl+ArrowLeft',
  NAV_NEXT_KEY_FRAME: 'Ctrl+ArrowRight',
  NAV_PREV_I_FRAME: 'Alt+ArrowLeft',
  NAV_NEXT_I_FRAME: 'Alt+ArrowRight',
  PLAY_PAUSE: 'Space',
  ZOOM_IN: '+',
  ZOOM_OUT: '-',
  ZOOM_RESET: '0',
  TOGGLE_FULLSCREEN: 'F',
  TOGGLE_MINIMAP: 'M',
  TOGGLE_REFERENCE_LINES: 'R',
  BOOKMARK_FRAME: 'B',
  PREV_BOOKMARK: 'Shift+B',
  NEXT_BOOKMARK: 'Ctrl+B',
  SEARCH: 'Ctrl+F',
  QUICK_JUMP: 'J',
  QUICK_JUMP_BACK: 'Shift+J',
} as const;

/**
 * Animation durations (in ms)
 */
export const ANIMATION_DURATION = {
  FAST: 150,
  NORMAL: 300,
  SLOW: 500,
} as const;

/**
 * Size variants
 */
export const SIZES = {
  SM: 'sm',
  MD: 'md',
  LG: 'lg',
} as const;

export type SizeVariant = typeof SIZES[keyof typeof SIZES];

/**
 * Common CSS class names
 */
export const CSS_CLASSES = {
  HIDDEN: 'hidden',
  DISABLED: 'disabled',
  ACTIVE: 'active',
  SELECTED: 'selected',
  LOADING: 'loading',
  ERROR: 'error',
  SUCCESS: 'success',
} as const;

/**
 * File extensions for supported video formats
 */
export const SUPPORTED_FORMATS = {
  HEVC: '.hevc',
  H265: '.h265',
  AVC: '.avc',
  H264: '.h264',
  AV1: '.av1',
  VP9: '.vp9',
  IVF: '.ivf',
} as const;

/**
 * Frame type colors (CSS variable names)
 */
export const FRAME_TYPE_COLORS = {
  I: '--frame-i',
  P: '--frame-p',
  B: '--frame-b',
  KEY: '--frame-i',
  INTRA: '--frame-i',
  INTER: '--frame-p',
} as const;

/**
 * Chart constants
 */
export const CHART = {
  MOVING_AVERAGE_WINDOW: 21,
  DEFAULT_BAR_HEIGHT: 24,
  MIN_BAR_WIDTH_PX: 2,
} as const;

/**
 * Filmstrip constants
 */
export const FILMSTRIP = {
  DEFAULT_THUMBNAIL_WIDTH: 100,
  DEFAULT_THUMBNAIL_HEIGHT: 56,
  THUMBNAIL_ASPECT_RATIO: 16 / 9,
  MIN_THUMBNAIL_SIZE: 50,
  MAX_THUMBNAIL_SIZE: 200,
  PRELOAD_RANGE: 10,
} as const;

/**
 * View mode labels
 */
export const VIEW_MODE_LABELS: Record<DisplayView, string> = {
  [DISPLAY_VIEWS.THUMBNAILS]: 'Thumbnails',
  [DISPLAY_VIEWS.FRAME_SIZES]: 'Frame Sizes',
  [DISPLAY_VIEWS.B_PYRAMID]: 'B-Pyramid',
  [DISPLAY_VIEWS.HRD_BUFFER]: 'HRD Buffer',
  [DISPLAY_VIEWS.ENHANCED]: 'Enhanced',
} as const;

/**
 * Mode labels for F1-F7 keys
 */
export const ANALYSIS_MODE_LABELS: Record<AnalysisMode, string> = {
  [ANALYSIS_MODES.OVERVIEW]: 'Overview (F1)',
  [ANALYSIS_MODES.CODING_FLOW]: 'Coding Flow (F2)',
  [ANALYSIS_MODES.PREDICTION]: 'Prediction (F3)',
  [ANALYSIS_MODES.TRANSFORM]: 'Transform (F4)',
  [ANALYSIS_MODES.QP_MAP]: 'QP Map (F5)',
  [ANALYSIS_MODES.MV_FIELD]: 'MV Field (F6)',
  [ANALYSIS_MODES.REFERENCE_FRAMES]: 'Reference Frames (F7)',
} as const;
