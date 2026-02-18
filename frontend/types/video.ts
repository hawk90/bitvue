/**
 * Video Type Definitions
 *
 * Centralized type definitions for video stream analysis.
 * These types are used across the application for consistency.
 */

/**
 * File information returned by the open_file command
 */
export interface FileInfo {
  success: boolean;
  path?: string;
  codec?: string;
  error?: string;
  frameCount?: number;
  width?: number;
  height?: number;
  bitrate?: number;
  duration?: number;
  fps?: number;
  profile?: string;
  level?: string;
  bitDepth?: number;
  chromaFormat?: string;
  fileSize?: number;
}

/**
 * Theme change event from Tauri menu
 */
export interface ThemeChangeEvent extends Event {
  detail: "dark" | "light"; // Theme name
}

/**
 * File opened event from Tauri
 */
export interface FileOpenedEvent {
  success: boolean;
  path?: string;
  error?: string;
  [key: string]: unknown;
}

/**
 * Stream information returned by get_stream_info command
 */
export interface StreamInfo {
  frameCount: number;
  fileSize: number;
  codec?: string;
  width?: number;
  height?: number;
  fps?: number;
  duration?: number;
}

/**
 * Thumbnail result returned by get_thumbnails command
 */
export interface ThumbnailResult {
  success: boolean;
  frame_index: number;
  thumbnail_data?: string;
  error?: string;
}

/**
 * Unit tree node for export
 * Represents a single unit in the bitstream syntax tree
 */
export interface UnitTreeNode {
  key: string;
  unit_type: string;
  offset: number;
  size: number;
  children: UnitTreeNode[];
}

/**
 * Syntax tree node for export
 * Represents a node in the syntax tree hierarchy
 */
export interface SyntaxNode {
  type: string;
  name?: string;
  children?: SyntaxNode[];
  [key: string]: unknown;
}

/**
 * Frame type enumeration
 * Standard frame types used in video compression
 */
export enum FrameType {
  I = "I", // Intra-coded frame (keyframe)
  P = "P", // Predicted frame
  B = "B", // Bidirectionally predicted frame
  KEY = "KEY", // Generic keyframe (used in some codecs)
  INTER = "INTER", // Inter frame (VP9, AV1)
  INTRA = "INTRA", // Intra frame (VP9, AV1)
  SWITCH = "SWITCH", // Switch frame (AV1)
  UNKNOWN = "UNKNOWN",
}

/**
 * Reference slot information
 * Used in AV1 and other codecs with explicit reference frame slots
 */
export interface ReferenceSlot {
  index: number; // Slot index (0-7 for AV1)
  name: string; // Slot name (LAST, LAST2, LAST3, GOLDEN, ALTREF, etc.)
  frameIndex?: number; // Which frame is stored in this slot
}

/**
 * Frame information
 * Core data structure for a single frame in a video stream
 */
export interface FrameInfo {
  frame_index: number; // Sequential frame index in the stream
  frame_type: string; // Frame type (I, P, B, KEY, etc.)
  size: number; // Frame size in bytes

  // Display and coding order
  poc?: number; // Picture Order Count
  pts?: number; // Presentation Timestamp
  display_order?: number; // Display order index
  coding_order?: number; // Coding/decode order index

  // Frame properties
  key_frame?: boolean; // Whether this is a keyframe
  temporal_id?: number; // Temporal layer ID
  spatial_id?: number; // Spatial layer ID (for scalable codecs)

  // References
  ref_frames?: number[]; // List of frame indices this frame references
  ref_slots?: number[]; // Reference slot indices used
  ref_slot_info?: ReferenceSlot[]; // Detailed slot information with names

  // Thumbnail
  thumbnail?: string; // Data URL or blob URL for thumbnail image

  // Timing
  duration?: number; // Frame duration in time units

  // Analysis data (QP heatmap, MV field, partition grid, prediction mode, transform)
  qp_grid?: QPGrid; // QP grid data for heatmap visualization
  mv_grid?: MVGrid; // MV grid data for motion vector field
  partition_grid?: PartitionGrid; // Partition grid for coding flow
  prediction_mode_grid?: PredictionModeGrid; // Prediction mode grid for prediction visualization
  transform_grid?: TransformGrid; // Transform grid for transform visualization

  // Frame dimensions
  width?: number; // Frame width in pixels
  height?: number; // Frame height in pixels
}

/**
 * Stream statistics
 * Aggregated statistics for a video stream
 */
export interface StreamStats {
  totalFrames: number; // Total number of frames
  keyFrames: number; // Number of keyframes
  totalSize: number; // Total size in bytes
  avgSize: number; // Average frame size in bytes
  frameTypes: Record<string, number>; // Count by frame type
}

/**
 * Video stream metadata
 * High-level information about a video stream
 */
export interface VideoMetadata {
  codec: string; // Codec name (VVC, HEVC, AV1, VP9, AVC, etc.)
  width: number; // Frame width in pixels
  height: number; // Frame height in pixels
  frameRate: number; // Frame rate (fps)
  bitDepth: number; // Bit depth (8, 10, etc.)
  profile?: string; // Codec profile
  level?: string; // Codec level
  chromaFormat: string; // Chroma subsampling (YUV420, YUV422, etc.)
}

/**
 * QP (Quantization Parameter) data
 * Used for QP map visualization
 */
export interface QPData {
  qp: number; // QP value
  x: number; // X position in frame
  y: number; // Y position in frame
  width: number; // Block width
  height: number; // Block height
}

/**
 * Motion Vector data
 * Used for MV field visualization
 */
export interface MotionVector {
  x: number; // X position in frame
  y: number; // Y position in frame
  mvX: number; // Motion vector X component
  mvY: number; // Motion vector Y component
  refIndex: number; // Reference frame index
  blockSize: number; // Block size
}

/**
 * Transform block information
 * Used for transform visualization
 */
export interface TransformBlock {
  x: number; // X position in frame
  y: number; // Y position in frame
  width: number; // Block width
  height: number; // Block height
  type: string; // Transform type (DCT, DST, etc.)
}

/**
 * Prediction mode information
 * Used for prediction visualization
 */
export interface PredictionInfo {
  x: number; // X position in frame
  y: number; // Y position in frame
  width: number; // Block width
  height: number; // Block height
  mode: string; // Prediction mode (intra/inter)
  modeIndex: number; // Specific mode index
}

/**
 * GOP (Group of Pictures) structure
 */
export interface GOPStructure {
  gopSize: number; // GOP size in frames
  gopCount: number; // Number of GOPs in stream
  iframeDistance: number; // Distance between I-frames
}

/**
 * Codec type enumeration
 * Supported video codecs
 */
export enum CodecType {
  VVC = "VVC", // H.266/Versatile Video Coding
  HEVC = "HEVC", // H.265/High Efficiency Video Coding
  AV1 = "AV1", // AOMedia Video 1
  VP9 = "VP9", // VP9 codec
  AVC = "AVC", // H.264/Advanced Video Coding
  MPEG2 = "MPEG2", // MPEG-2
  UNKNOWN = "UNKNOWN",
}

/**
 * Visualization mode
 * Different overlay visualization modes
 */
export type VisualizationMode =
  | "overview" // No overlay, just the frame
  | "coding-flow" // CTU/block structure
  | "prediction" // Prediction mode visualization
  | "transform" // Transform block sizes
  | "qp-map" // QP heatmap
  | "mv-field" // Motion vector field
  | "reference"; // Reference frame relationships

/**
 * Filmstrip display view
 * Different ways to display frames in the filmstrip
 */
export type FilmstripView =
  | "thumbnails" // Thumbnail images
  | "sizes" // Frame size bar chart
  | "bpyramid" // B-Pyramid GOP visualization
  | "hrdbuffer" // HRD buffer visualization
  | "enhanced" // Enhanced view with additional info
  | "minimap"; // Minimap view

/**
 * Color space types
 */
export enum ColorSpace {
  BT601 = "BT601", // ITU-R BT.601
  BT709 = "BT709", // ITU-R BT.709
  BT2020 = "BT2020", // ITU-R BT.2020
  SMPTE170M = "SMPTE170M", // SMPTE 170M
}

/**
 * YUV format types
 */
export enum YUVFormat {
  YUV420 = "YUV420", // 4:2:0 chroma subsampling
  YUV422 = "YUV422", // 4:2:2 chroma subsampling
  YUV444 = "YUV444", // 4:4:4 no chroma subsampling
  NV12 = "NV12", // Semi-planar YUV420
  P010 = "P010", // 10-bit YUV420
}

/**
 * Helper function to get frame type color class
 * Returns the CSS class name for styling frame types
 */
export function getFrameTypeColorClass(frameType: string): string {
  const type = frameType.toUpperCase();
  switch (type) {
    case FrameType.I:
    case FrameType.KEY:
    case FrameType.INTRA:
      return "frame-i";
    case FrameType.P:
    case FrameType.INTER:
      return "frame-p";
    case FrameType.B:
      return "frame-b";
    default:
      return "frame-unknown";
  }
}

/**
 * Helper function to get frame type CSS color variable
 * Returns the CSS variable for styling frame types
 */
export function getFrameTypeColor(frameType: string): string {
  const type = frameType.toLowerCase();
  if (type === "i" || type === "key") return "var(--frame-i)";
  if (type === "p" || type === "inter") return "var(--frame-p)";
  if (type.startsWith("b")) return "var(--frame-b)";
  return "var(--text-secondary)";
}

/**
 * Helper function to check if a frame type is intra-coded
 */
export function isIntraFrame(frameType: string): boolean {
  const type = frameType.toUpperCase();
  return (
    type === FrameType.I || type === FrameType.KEY || type === FrameType.INTRA
  );
}

/**
 * Helper function to check if a frame type is inter-coded
 */
export function isInterFrame(frameType: string): boolean {
  const type = frameType.toUpperCase();
  return (
    type === FrameType.P || type === FrameType.B || type === FrameType.INTER
  );
}

/**
 * Helper function to check if a frame is a keyframe
 */
export function isKeyframe(frameType: string, keyFrame?: boolean): boolean {
  if (keyFrame !== undefined) return keyFrame;
  const type = frameType.toUpperCase();
  return (
    type === FrameType.I || type === FrameType.KEY || type === FrameType.INTRA
  );
}

/**
 * QP Grid data for heatmap visualization
 * Per-block quantization parameter values from the bitstream parser
 */
export interface QPGrid {
  grid_w: number; // Grid width (number of blocks horizontally)
  grid_h: number; // Grid height (number of blocks vertically)
  block_w: number; // Block width in pixels (e.g., 8 for AV1, 16 for H.264)
  block_h: number; // Block height in pixels
  qp: number[]; // QP values (row-major: grid_w * grid_h), -1 for missing
  qp_min: number; // Minimum QP value in the data
  qp_max: number; // Maximum QP value in the data
}

/**
 * Motion Vector data for a single block
 */
export interface MotionVectorBlock {
  dx_qpel: number; // Horizontal displacement in quarter-pel units
  dy_qpel: number; // Vertical displacement in quarter-pel units
}

/**
 * Block mode enumeration
 * Matches bitvue_core::mv_overlay::BlockMode
 */
export enum BlockMode {
  None = 0,
  Inter = 1,
  Intra = 2,
  Skip = 3,
}

/**
 * MV Grid data for motion vector field visualization
 * Per-block motion vectors from the bitstream parser
 */
export interface MVGrid {
  coded_width: number; // Coded frame width in pixels
  coded_height: number; // Coded frame height in pixels
  block_w: number; // Block width in pixels
  block_h: number; // Block height in pixels
  grid_w: number; // Grid width in blocks
  grid_h: number; // Grid height in blocks
  mv_l0: MotionVectorBlock[]; // L0 motion vectors (forward prediction)
  mv_l1: MotionVectorBlock[]; // L1 motion vectors (backward prediction)
  mode?: BlockMode[]; // Optional block modes
}

/**
 * Partition type for visualization
 * Matches bitvue_core::partition_grid::PartitionType
 */
export enum PartitionType {
  None = 0, // No partition (leaf block)
  Horz = 1, // Horizontal split
  Vert = 2, // Vertical split
  Split = 3, // 4-way split
  HorzA = 4, // Horizontal A (top split)
  HorzB = 5, // Horizontal B (bottom split)
  VertA = 6, // Vertical A (left split)
  VertB = 7, // Vertical B (right split)
  Horz4 = 8, // 4-way horizontal
  Vert4 = 9, // 4-way vertical
}

/**
 * Partition block in the grid
 */
export interface PartitionBlock {
  x: number; // Block X position in pixels
  y: number; // Block Y position in pixels
  width: number; // Block width in pixels
  height: number; // Block height in pixels
  partition: PartitionType; // How this block was created
  depth: number; // Nesting depth (0 = superblock)
}

/**
 * Partition Grid data for coding flow visualization
 * Hierarchical block structure from the bitstream parser
 */
export interface PartitionGrid {
  coded_width: number; // Coded frame width in pixels
  coded_height: number; // Coded frame height in pixels
  sb_size: number; // Superblock size (typically 64 or 128)
  blocks: PartitionBlock[]; // All leaf blocks (coding units)
}

/**
 * Prediction Mode Grid data for prediction visualization
 * Per-block prediction mode data (INTRA/INTER modes)
 */
export interface PredictionModeGrid {
  coded_width: number; // Coded frame width in pixels
  coded_height: number; // Coded frame height in pixels
  block_w: number; // Block width in pixels (typically 16)
  block_h: number; // Block height in pixels (typically 16)
  grid_w: number; // Grid width in blocks
  grid_h: number; // Grid height in blocks
  modes: (number | null)[]; // Prediction mode for each block (u8 value or null)
}

/**
 * Transform Grid data for transform visualization
 * Per-block transform size data (4x4, 8x8, 16x16, 32x32, 64x64)
 */
export interface TransformGrid {
  coded_width: number; // Coded frame width in pixels
  coded_height: number; // Coded frame height in pixels
  block_w: number; // Block width in pixels (typically 32)
  block_h: number; // Block height in pixels (typically 32)
  grid_w: number; // Grid width in blocks
  grid_h: number; // Grid height in blocks
  tx_sizes: (number | null)[]; // Transform size for each block (u8 value or null, 0=4x4, 1=8x8, 2=16x16, 3=32x32, 4=64x64)
}

/**
 * Decoded frame data
 * Full-resolution frame for video player display
 */
export interface DecodedFrameData {
  frame_index: number;
  width: number;
  height: number;
  frame_data: string; // Base64 encoded PNG (full resolution)
  success: boolean;
  error?: string;
}

/**
 * YUV frame data for direct rendering
 * More efficient than RGB conversion - decoder outputs YUV natively
 */
export interface YUVFrameData {
  frame_index: number;
  width: number;
  height: number;
  bit_depth: number;
  y_plane: string; // Base64 encoded Y plane
  u_plane: string | null; // Base64 encoded U plane (null for monochrome)
  v_plane: string | null; // Base64 encoded V plane (null for monochrome)
  y_stride: number;
  u_stride: number;
  v_stride: number;
  success: boolean;
  error?: string;
}

/**
 * Frame analysis data
 * QP heatmap and MV field data for overlay visualization
 */
export interface FrameAnalysisData {
  frame_index: number;
  width: number;
  height: number;
  qp_grid?: QPGrid;
  mv_grid?: MVGrid;
  partition_grid?: PartitionGrid; // Partition grid for coding flow
  prediction_mode_grid?: PredictionModeGrid; // Prediction mode grid for prediction overlay
  transform_grid?: TransformGrid; // Transform grid for transform overlay
}

/**
 * Frame index map for alignment
 * Maps display indices to PTS values
 */
export interface FrameIndexMap {
  frame_count: number; // Total number of frames
  pts_values: (number | null)[]; // PTS value for each frame (null if unavailable)
  display_indices: number[]; // Display order indices
  coding_indices: number[]; // Coding/decode order indices
}

/**
 * PTS quality level
 */
export enum PtsQuality {
  Good = "Good", // PTS values are reliable and monotonic
  Warn = "Warn", // PTS values have some issues but usable
  Bad = "Bad", // PTS values are unreliable
}

/**
 * Alignment method used
 */
export enum AlignmentMethod {
  PtsExact = "PtsExact", // PTS values match exactly (delta = 0)
  PtsNearest = "PtsNearest", // PTS-based nearest-neighbor matching
  DisplayIdx = "DisplayIdx", // Fallback to display_idx alignment
}

/**
 * Alignment confidence level
 */
export enum AlignmentConfidence {
  High = "High", // Perfect or near-perfect alignment (<5% gaps)
  Medium = "Medium", // Usable alignment with some gaps (5-20%)
  Low = "Low", // Many gaps or fallback alignment method (>20% gaps)
}

/**
 * Frame pair in alignment
 */
export interface FramePair {
  stream_a_idx: number | null; // Stream A frame index (null = gap in A)
  stream_b_idx: number | null; // Stream B frame index (null = gap in B)
  pts_delta: number | null; // PTS delta (A - B) in same units as PTS
  has_gap: boolean; // Gap indicator (one stream missing frame)
}

/**
 * Alignment engine result
 */
export interface AlignmentEngine {
  stream_a_count: number;
  stream_b_count: number;
  method: AlignmentMethod;
  confidence: AlignmentConfidence;
  frame_pairs: FramePair[];
  gap_count: number;
}

/**
 * Resolution information for both streams
 */
export interface ResolutionInfo {
  stream_a: [number, number]; // Stream A resolution (width, height)
  stream_b: [number, number]; // Stream B resolution (width, height)
  tolerance: number; // Compatibility threshold (default 0.05 = 5%)
  is_compatible: boolean;
  mismatch_percentage: number;
  is_exact_match: boolean;
  scale_indicator: string;
}

/**
 * Alignment quality for frame pair
 */
export enum AlignmentQuality {
  Exact = "Exact", // Exact PTS match (delta = 0)
  Nearest = "Nearest", // Nearest neighbor match (small delta)
  Gap = "Gap", // Gap in alignment
}

/**
 * Sync mode for compare view
 */
export enum SyncMode {
  Off = "Off", // No synchronization
  Playhead = "Playhead", // Sync playhead position only
  Full = "Full", // Full synchronization (playhead + playback)
}

/**
 * Sync controls UI state
 */
export interface SyncControls {
  mode: SyncMode;
  manual_offset_enabled: boolean;
  manual_offset: number;
  show_alignment_info: boolean;
}

/**
 * Compare workspace managing A/B streams
 */
export interface CompareWorkspace {
  stream_a: FrameIndexMap;
  stream_b: FrameIndexMap;
  alignment: AlignmentEngine;
  manual_offset: number;
  sync_mode: SyncMode;
  resolution_info: ResolutionInfo;
  diff_enabled: boolean;
  disable_reason: string | null;
}
