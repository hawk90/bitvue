/**
 * Component Constants Tests
 */

import { describe, it, expect } from 'vitest';
import {
  DISPLAY_VIEWS,
  FRAME_TYPES,
  ANALYSIS_MODES,
  PANELS,
  SYNTAX_TABS,
  HEX_TABS,
  SIZE_METRICS,
  KEYBOARD_SHORTCUTS,
  ANIMATION_DURATION,
  SIZES,
  CSS_CLASSES,
  SUPPORTED_FORMATS,
  FRAME_TYPE_COLORS,
  CHART,
  FILMSTRIP,
  VIEW_MODE_LABELS,
  ANALYSIS_MODE_LABELS,
  type DisplayView,
  type FrameType,
  type AnalysisMode,
  type PanelId,
  type SizeVariant,
} from '../components';

describe('DISPLAY_VIEWS', () => {
  it('should have all expected view modes', () => {
    expect(DISPLAY_VIEWS.THUMBNAILS).toBe('thumbnails');
    expect(DISPLAY_VIEWS.FRAME_SIZES).toBe('frame-sizes');
    expect(DISPLAY_VIEWS.B_PYRAMID).toBe('b-pyramid');
    expect(DISPLAY_VIEWS.HRD_BUFFER).toBe('hrd-buffer');
    expect(DISPLAY_VIEWS.ENHANCED).toBe('enhanced');
  });

  it('should be readonly', () => {
    // @ts-expect-error - Testing immutability
    expect(() => { DISPLAY_VIEWS.THUMBNAILS = 'other'; }).toThrow();
  });
});

describe('FRAME_TYPES', () => {
  it('should have all expected frame types', () => {
    expect(FRAME_TYPES.I).toBe('I');
    expect(FRAME_TYPES.P).toBe('P');
    expect(FRAME_TYPES.B).toBe('B');
    expect(FRAME_TYPES.KEY).toBe('KEY');
    expect(FRAME_TYPES.INTRA).toBe('INTRA');
    expect(FRAME_TYPES.INTER).toBe('INTER');
    expect(FRAME_TYPES.UNKNOWN).toBe('UNKNOWN');
  });
});

describe('ANALYSIS_MODES', () => {
  it('should have all expected analysis modes', () => {
    expect(ANALYSIS_MODES.OVERVIEW).toBe('overview');
    expect(ANALYSIS_MODES.CODING_FLOW).toBe('coding-flow');
    expect(ANALYSIS_MODES.PREDICTION).toBe('prediction');
    expect(ANALYSIS_MODES.TRANSFORM).toBe('transform');
    expect(ANALYSIS_MODES.QP_MAP).toBe('qp-map');
    expect(ANALYSIS_MODES.MV_FIELD).toBe('mv-field');
    expect(ANALYSIS_MODES.REFERENCE_FRAMES).toBe('reference-frames');
  });
});

describe('PANELS', () => {
  it('should have all expected panel IDs', () => {
    expect(PANELS.SYNTAX_DETAIL).toBe('syntax-detail');
    expect(PANELS.UNIT_HEX).toBe('unit-hex');
    expect(PANELS.STREAM_TREE).toBe('stream-tree');
    expect(PANELS.STATISTICS).toBe('statistics');
    expect(PANELS.INFO).toBe('info');
    expect(PANELS.DETAILS).toBe('details');
    expect(PANELS.REFERENCE_GRAPH).toBe('reference-graph');
    expect(PANELS.BITRATE_GRAPH).toBe('bitrate-graph');
    expect(PANELS.QUALITY_METRICS).toBe('quality-metrics');
    expect(PANELS.BIT_VIEW).toBe('bit-view');
    expect(PANELS.SELECTION_INFO).toBe('selection-info');
    expect(PANELS.DIAGNOSTICS).toBe('diagnostics');
    expect(PANELS.BOOKMARKS).toBe('bookmarks');
    expect(PANELS.YUV_VIEWER).toBe('yuv-viewer');
  });
});

describe('SYNTAX_TABS', () => {
  it('should have all expected syntax tabs', () => {
    expect(SYNTAX_TABS.FRAME_SYNTAX).toBe('frame-syntax');
    expect(SYNTAX_TABS.REFERENCES).toBe('references');
    expect(SYNTAX_TABS.STATISTICS).toBe('statistics');
    expect(SYNTAX_TABS.SEARCH).toBe('search');
  });
});

describe('HEX_TABS', () => {
  it('should have all expected hex tabs', () => {
    expect(HEX_TABS.FRAME_VIEW).toBe('frame-view');
    expect(HEX_TABS.HEX_VIEW).toBe('hex-view');
    expect(HEX_TABS.DPB_VIEW).toBe('dpb-view');
  });
});

describe('SIZE_METRICS', () => {
  it('should have all expected size metrics', () => {
    expect(SIZE_METRICS.SHOW_BITRATE_BAR).toBe('showBitrateBar');
    expect(SIZE_METRICS.SHOW_BITRATE_CURVE).toBe('showBitrateCurve');
    expect(SIZE_METRICS.SHOW_AVG_SIZE).toBe('showAvgSize');
    expect(SIZE_METRICS.SHOW_MIN_SIZE).toBe('showMinSize');
    expect(SIZE_METRICS.SHOW_MAX_SIZE).toBe('showMaxSize');
    expect(SIZE_METRICS.SHOW_MOVING_AVG).toBe('showMovingAvg');
    expect(SIZE_METRICS.SHOW_BLOCK_MIN_QP).toBe('showBlockMinQP');
    expect(SIZE_METRICS.SHOW_BLOCK_MAX_QP).toBe('showBlockMaxQP');
  });
});

describe('KEYBOARD_SHORTCUTS', () => {
  it('should have all expected keyboard shortcuts', () => {
    expect(KEYBOARD_SHORTCUTS.NAV_FIRST_FRAME).toBe('Home');
    expect(KEYBOARD_SHORTCUTS.NAV_LAST_FRAME).toBe('End');
    expect(KEYBOARD_SHORTCUTS.NAV_PREV_FRAME).toBe('ArrowLeft');
    expect(KEYBOARD_SHORTCUTS.NAV_NEXT_FRAME).toBe('ArrowRight');
    expect(KEYBOARD_SHORTCUTS.PLAY_PAUSE).toBe('Space');
    expect(KEYBOARD_SHORTCUTS.ZOOM_IN).toBe('+');
    expect(KEYBOARD_SHORTCUTS.ZOOM_OUT).toBe('-');
    expect(KEYBOARD_SHORTCUTS.ZOOM_RESET).toBe('0');
    expect(KEYBOARD_SHORTCUTS.TOGGLE_FULLSCREEN).toBe('F');
  });
});

describe('ANIMATION_DURATION', () => {
  it('should have all expected animation durations', () => {
    expect(ANIMATION_DURATION.FAST).toBe(150);
    expect(ANIMATION_DURATION.NORMAL).toBe(300);
    expect(ANIMATION_DURATION.SLOW).toBe(500);
  });
});

describe('SIZES', () => {
  it('should have all expected sizes', () => {
    expect(SIZES.SM).toBe('sm');
    expect(SIZES.MD).toBe('md');
    expect(SIZES.LG).toBe('lg');
  });
});

describe('CSS_CLASSES', () => {
  it('should have all expected CSS classes', () => {
    expect(CSS_CLASSES.HIDDEN).toBe('hidden');
    expect(CSS_CLASSES.DISABLED).toBe('disabled');
    expect(CSS_CLASSES.ACTIVE).toBe('active');
    expect(CSS_CLASSES.SELECTED).toBe('selected');
    expect(CSS_CLASSES.LOADING).toBe('loading');
    expect(CSS_CLASSES.ERROR).toBe('error');
    expect(CSS_CLASSES.SUCCESS).toBe('success');
  });
});

describe('SUPPORTED_FORMATS', () => {
  it('should have all expected supported formats', () => {
    expect(SUPPORTED_FORMATS.HEVC).toBe('.hevc');
    expect(SUPPORTED_FORMATS.H265).toBe('.h265');
    expect(SUPPORTED_FORMATS.AVC).toBe('.avc');
    expect(SUPPORTED_FORMATS.H264).toBe('.h264');
    expect(SUPPORTED_FORMATS.AV1).toBe('.av1');
    expect(SUPPORTED_FORMATS.VP9).toBe('.vp9');
    expect(SUPPORTED_FORMATS.IVF).toBe('.ivf');
  });
});

describe('FRAME_TYPE_COLORS', () => {
  it('should have all expected frame type colors', () => {
    expect(FRAME_TYPE_COLORS.I).toBe('--frame-i');
    expect(FRAME_TYPE_COLORS.P).toBe('--frame-p');
    expect(FRAME_TYPE_COLORS.B).toBe('--frame-b');
    expect(FRAME_TYPE_COLORS.KEY).toBe('--frame-i');
    expect(FRAME_TYPE_COLORS.INTRA).toBe('--frame-i');
    expect(FRAME_TYPE_COLORS.INTER).toBe('--frame-p');
  });
});

describe('CHART', () => {
  it('should have all expected chart constants', () => {
    expect(CHART.MOVING_AVERAGE_WINDOW).toBe(21);
    expect(CHART.DEFAULT_BAR_HEIGHT).toBe(24);
    expect(CHART.MIN_BAR_WIDTH_PX).toBe(2);
  });
});

describe('FILMSTRIP', () => {
  it('should have all expected filmstrip constants', () => {
    expect(FILMSTRIP.DEFAULT_THUMBNAIL_WIDTH).toBe(100);
    expect(FILMSTRIP.DEFAULT_THUMBNAIL_HEIGHT).toBe(56);
    expect(FILMSTRIP.THUMBNAIL_ASPECT_RATIO).toBe(16 / 9);
    expect(FILMSTRIP.MIN_THUMBNAIL_SIZE).toBe(50);
    expect(FILMSTRIP.MAX_THUMBNAIL_SIZE).toBe(200);
    expect(FILMSTRIP.PRELOAD_RANGE).toBe(10);
  });
});

describe('VIEW_MODE_LABELS', () => {
  it('should have labels for all view modes', () => {
    expect(VIEW_MODE_LABELS[DISPLAY_VIEWS.THUMBNAILS]).toBe('Thumbnails');
    expect(VIEW_MODE_LABELS[DISPLAY_VIEWS.FRAME_SIZES]).toBe('Frame Sizes');
    expect(VIEW_MODE_LABELS[DISPLAY_VIEWS.B_PYRAMID]).toBe('B-Pyramid');
    expect(VIEW_MODE_LABELS[DISPLAY_VIEWS.HRD_BUFFER]).toBe('HRD Buffer');
    expect(VIEW_MODE_LABELS[DISPLAY_VIEWS.ENHANCED]).toBe('Enhanced');
  });
});

describe('ANALYSIS_MODE_LABELS', () => {
  it('should have labels for all analysis modes', () => {
    expect(ANALYSIS_MODE_LABELS[ANALYSIS_MODES.OVERVIEW]).toBe('Overview (F1)');
    expect(ANALYSIS_MODE_LABELS[ANALYSIS_MODES.CODING_FLOW]).toBe('Coding Flow (F2)');
    expect(ANALYSIS_MODE_LABELS[ANALYSIS_MODES.PREDICTION]).toBe('Prediction (F3)');
    expect(ANALYSIS_MODE_LABELS[ANALYSIS_MODES.TRANSFORM]).toBe('Transform (F4)');
    expect(ANALYSIS_MODE_LABELS[ANALYSIS_MODES.QP_MAP]).toBe('QP Map (F5)');
    expect(ANALYSIS_MODE_LABELS[ANALYSIS_MODES.MV_FIELD]).toBe('MV Field (F6)');
    expect(ANALYSIS_MODE_LABELS[ANALYSIS_MODES.REFERENCE_FRAMES]).toBe('Reference Frames (F7)');
  });
});

describe('Type Guards', () => {
  it('should correctly type DisplayView', () => {
    const view: DisplayView = DISPLAY_VIEWS.THUMBNAILS;
    expect(view).toBe('thumbnails');
  });

  it('should correctly type FrameType', () => {
    const frameType: FrameType = FRAME_TYPES.I;
    expect(frameType).toBe('I');
  });

  it('should correctly type AnalysisMode', () => {
    const mode: AnalysisMode = ANALYSIS_MODES.OVERVIEW;
    expect(mode).toBe('overview');
  });

  it('should correctly type PanelId', () => {
    const panel: PanelId = PANELS.SYNTAX_DETAIL;
    expect(panel).toBe('syntax-detail');
  });

  it('should correctly type SizeVariant', () => {
    const size: SizeVariant = SIZES.MD;
    expect(size).toBe('md');
  });
});
