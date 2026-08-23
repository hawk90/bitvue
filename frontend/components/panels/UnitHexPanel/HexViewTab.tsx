/**
 * Hex View Tab Component
 *
 * Displays hex dump of frame bytes with highlighting
 */

import { memo, useCallback, useState, useEffect, useRef } from "react";
import {
  getHexRange,
  getContextMenuItems,
  type ContextMenuItemWire,
} from "../../../services/electronBridgeService";
import { useExportEvidenceBundle } from "../../../hooks/useExportEvidenceBundle";
import { ContextMenu } from "../../ContextMenu";
import { createLogger } from "../../../utils/logger";
import { useSelection } from "../../../contexts/SelectionContext";

const logger = createLogger("HexViewTab");
const BYTES_PER_LINE = 16;
const MAX_HEX_BYTES = 2048;

interface HexViewTabProps {
  frameIndex: number;
  frames: Array<{
    frame_index: number;
    size: number;
    /** Real on-disk unit offset (see FrameInfo.offset) -- required to fetch this frame's raw
     *  bytes. Frames sourced from anywhere but FileStateContext's real bridge data won't have
     *  this, so it's optional and the fetch below handles its absence honestly. */
    offset?: number;
  }>;
}

export const HexViewTab = memo(function HexViewTab({
  frameIndex,
  frames,
}: HexViewTabProps) {
  // INT-03: selectedByte/selectionEnd together represent a [min,max] byte range -- a plain click
  // sets both to the same byte (the pre-existing single-byte behavior), a real drag extends
  // selectionEnd live as the pointer moves. Kept as two values (not one {start,end} object) so
  // the single-byte case stays a trivial, cheap comparison in the hot getByteStyle path below.
  const [selectedByte, setSelectedByte] = useState<number | null>(null);
  const [selectionEnd, setSelectionEnd] = useState<number | null>(null);
  const [hexData, setHexData] = useState<Uint8Array>(new Uint8Array(0));
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [totalSize, setTotalSize] = useState<number>(0);
  const [truncated, setTruncated] = useState<boolean>(false);
  const { selection, setBitRangeSelection } = useSelection();
  const containerRef = useRef<HTMLDivElement>(null);

  // Real [start, end] byte range (inclusive), derived from the two raw state values above.
  const rangeStart =
    selectedByte !== null && selectionEnd !== null
      ? Math.min(selectedByte, selectionEnd)
      : null;
  const rangeEndInclusive =
    selectedByte !== null && selectionEnd !== null
      ? Math.max(selectedByte, selectionEnd)
      : null;
  const isMultiByteRange =
    rangeStart !== null &&
    rangeEndInclusive !== null &&
    rangeStart !== rangeEndInclusive;

  // Right-click context menu (Phase 7.6, "HexView" scope) -- see ContextMenu component doc.
  const exportEvidence = useExportEvidenceBundle();
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    items: ContextMenuItemWire[];
  } | null>(null);

  const handleHexContextMenu = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      const hasByteRange = rangeStart !== null;
      const x = event.clientX;
      const y = event.clientY;
      getContextMenuItems("HexView", false, hasByteRange)
        .then((items) => setContextMenu({ x, y, items }))
        .catch(() => setContextMenu(null));
    },
    [rangeStart],
  );

  const handleContextMenuSelect = useCallback(
    (command: string) => {
      if (command === "Export.EvidenceBundle") {
        void exportEvidence();
      } else if (
        command === "Copy.Bytes" &&
        rangeStart !== null &&
        rangeEndInclusive !== null
      ) {
        // INT-03: copies the whole selected range, not just one byte -- clamped to what's
        // actually loaded (a selection can technically extend past hexData when it arrived via
        // a Syntax->Hex jump into a truncated frame's untruncated tail).
        const clampedEnd = Math.min(rangeEndInclusive, hexData.length - 1);
        const hex = Array.from(hexData.slice(rangeStart, clampedEnd + 1))
          .map((b) => b.toString(16).padStart(2, "0").toUpperCase())
          .join(" ");
        void navigator.clipboard.writeText(hex);
      } else if (command === "Copy.Offset" && rangeStart !== null) {
        void navigator.clipboard.writeText(`0x${rangeStart.toString(16)}`);
      } else if (
        command === "Copy.BitRange" &&
        rangeStart !== null &&
        rangeEndInclusive !== null
      ) {
        // Derived directly from the local byte range (not `selection.bitRange`, which only
        // reflects the last range that finished its async `select_bit_range` round trip and can
        // be stale for a selection made moments ago) -- same [startBit, endBit) convention the
        // Syntax->Hex sync effect above already assumes (endBit is one past the last included
        // bit).
        const startBit = rangeStart * 8;
        const endBit = (rangeEndInclusive + 1) * 8;
        void navigator.clipboard.writeText(`${startBit}-${endBit}`);
      }
    },
    [exportEvidence, rangeStart, rangeEndInclusive, hexData],
  );

  const currentFrame = frames[frameIndex];

  // Load hex data when frame changes
  useEffect(() => {
    if (!currentFrame) return;

    let cancelled = false;

    const loadHexData = async () => {
      setLoading(true);
      setError(null);

      try {
        if (currentFrame.offset === undefined) {
          setError("No on-disk offset available for this frame");
          return;
        }
        const len = Math.min(currentFrame.size, MAX_HEX_BYTES);
        const result = await getHexRange("A", currentFrame.offset, len);

        if (cancelled) return;

        setHexData(result.bytes);
        setTotalSize(currentFrame.size);
        setTruncated(currentFrame.size > MAX_HEX_BYTES);
        logger.info(
          `Loaded ${result.bytes.length} bytes for frame ${frameIndex} (total: ${currentFrame.size})`,
        );
      } catch (err) {
        if (!cancelled) {
          const errorMsg = err instanceof Error ? err.message : String(err);
          setError(errorMsg);
          logger.error("Failed to load hex data:", err);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    loadHexData();

    return () => {
      cancelled = true;
    };
  }, [frameIndex, currentFrame]);

  // Syntax -> Hex: scroll to and select the byte when a real selection.bitRange arrives (from
  // FrameSyntaxTab's jump-to-hex click, or a byte selected here re-confirming itself -- see this
  // component's module context in the axis-2 Tri-Sync completion plan). bitRange is in bits;
  // this view works in bytes.
  const resolvedBitRange = selection?.bitRange;
  useEffect(() => {
    if (!resolvedBitRange || !containerRef.current) return;
    const startByte = Math.floor(resolvedBitRange.startBit / 8);
    // endBit is exclusive (matches setBitRangeSelection's own convention below); a syntax node
    // can span multiple bytes, so highlight its whole span, not just its first byte.
    const endByte = Math.max(
      startByte,
      Math.floor((resolvedBitRange.endBit - 1) / 8),
    );
    setSelectedByte(startByte);
    setSelectionEnd(endByte);
    // Scroll: each hex line is ~20px tall
    const lineIdx = Math.floor(startByte / BYTES_PER_LINE);
    const lineHeight = 20;
    containerRef.current.scrollTop = Math.max(0, lineIdx * lineHeight - 40);
  }, [resolvedBitRange]);

  // INT-03: real multi-byte drag-select. mousedown on a byte starts a drag and commits an
  // immediate 1-byte selection (so a plain click, with no following mousemove, still works
  // exactly as before); mouseenter on a later byte while dragging live-extends the visual
  // range (no backend call per pixel of drag -- cheap); mouseup (global, since the drag can end
  // outside any byte span or even outside the window) commits the final range with exactly one
  // real select_bit_range round trip, mirroring VideoCanvas.tsx's click-vs-drag pattern (INT-01).
  const isDraggingRef = useRef(false);
  const dragStartByteRef = useRef<number | null>(null);
  const dragCurrentByteRef = useRef<number | null>(null);

  const handleByteMouseDown = useCallback((byteOffset: number) => {
    isDraggingRef.current = true;
    dragStartByteRef.current = byteOffset;
    dragCurrentByteRef.current = byteOffset;
    setSelectedByte(byteOffset);
    setSelectionEnd(byteOffset);
  }, []);

  const handleByteMouseEnter = useCallback((byteOffset: number) => {
    if (!isDraggingRef.current || dragStartByteRef.current === null) return;
    dragCurrentByteRef.current = byteOffset;
    setSelectedByte(Math.min(dragStartByteRef.current, byteOffset));
    setSelectionEnd(Math.max(dragStartByteRef.current, byteOffset));
  }, []);

  useEffect(() => {
    const handleGlobalMouseUp = () => {
      if (!isDraggingRef.current || dragStartByteRef.current === null) return;
      isDraggingRef.current = false;
      const start = dragStartByteRef.current;
      const end = dragCurrentByteRef.current ?? start;
      dragStartByteRef.current = null;
      dragCurrentByteRef.current = null;
      const finalStart = Math.min(start, end);
      const finalEnd = Math.max(start, end);
      // Hex -> Syntax: drives the real select_bit_range round trip so FrameSyntaxTab can
      // expand/scroll to whatever syntax node(s) this byte range overlaps.
      setBitRangeSelection(
        { startBit: finalStart * 8, endBit: (finalEnd + 1) * 8 },
        "hex",
      );
    };
    window.addEventListener("mouseup", handleGlobalMouseUp);
    return () => window.removeEventListener("mouseup", handleGlobalMouseUp);
  }, [setBitRangeSelection]);

  // Convert byte to ASCII character
  const byteToAscii = useCallback((byte: number): string => {
    if (byte >= 0x20 && byte <= 0x7e) {
      return String.fromCharCode(byte);
    }
    return ".";
  }, []);

  // Check if byte is part of start code (00 00 01)
  const isStartCode = useCallback(
    (offset: number): boolean => {
      if (offset < 3) return false;
      // Check for 00 00 01 pattern (AV1 OBU start code)
      if (
        hexData[offset] === 0x01 &&
        hexData[offset - 1] === 0x00 &&
        hexData[offset - 2] === 0x00
      ) {
        return true;
      }
      return false;
    },
    [hexData],
  );

  // Check if byte is OBU header
  const isObuHeader = useCallback(
    (offset: number): boolean => {
      if (offset === 0) return hexData[0] !== 0x00; // First byte might be OBU if no start code
      // After start code, first byte is OBU header
      if (offset >= 3 && isStartCode(offset - 1)) {
        return true;
      }
      return false;
    },
    [hexData, isStartCode],
  );

  // Get byte style based on position and value
  const getByteStyle = useCallback(
    (offset: number): React.CSSProperties => {
      if (
        rangeStart !== null &&
        rangeEndInclusive !== null &&
        offset >= rangeStart &&
        offset <= rangeEndInclusive
      ) {
        return { color: "#ffb450", backgroundColor: "rgba(255, 180, 80, 0.2)" };
      }
      if (isStartCode(offset)) {
        return { color: "#ff6464", fontWeight: "500" };
      }
      if (isObuHeader(offset)) {
        return { color: "#4a9eff", fontWeight: "500" };
      }
      if (offset < 3) {
        return { color: "#ff6464", fontWeight: "500" };
      }
      return { color: "var(--text-primary)" };
    },
    [rangeStart, rangeEndInclusive, isStartCode, isObuHeader],
  );

  if (!currentFrame) {
    return (
      <div className="hex-empty">
        <span className="codicon codicon-file-code"></span>
        <span>No frame selected</span>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="hex-empty">
        <span className="codicon codicon-loading codicon-spin"></span>
        <span>Loading hex data...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="hex-empty">
        <span className="codicon codicon-error"></span>
        <span>{error}</span>
      </div>
    );
  }

  if (hexData.length === 0) {
    return (
      <div className="hex-empty">
        <span className="codicon codicon-file-code"></span>
        <span>No hex data available</span>
      </div>
    );
  }

  const lines = Math.ceil(hexData.length / BYTES_PER_LINE);

  return (
    <div
      className="hex-dump-content"
      ref={containerRef}
      onContextMenu={handleHexContextMenu}
    >
      <div className="hex-info-bar">
        <span className="hex-info-item">
          <span className="hex-info-label">Data:</span>
          <span className="hex-info-value">
            {truncated
              ? `First ${hexData.length} bytes`
              : `All ${hexData.length} bytes`}
            {truncated && ` (of ${totalSize} total)`}
          </span>
        </span>
        <span className="hex-info-item">
          <span className="hex-info-label">Frame:</span>
          <span className="hex-info-value">{frameIndex}</span>
        </span>
      </div>

      {Array.from({ length: lines }, (_, lineIdx) => {
        const offset = lineIdx * BYTES_PER_LINE;
        const end = Math.min(offset + BYTES_PER_LINE, hexData.length);
        const lineBytes = Array.from(
          { length: end - offset },
          (_, i) => hexData[offset + i],
        );
        const ascii = lineBytes.map(byteToAscii).join("");

        return (
          <div key={offset} className="hex-line">
            <span className="hex-offset">
              {offset.toString(16).padStart(8, "0").toUpperCase()}
            </span>
            <span className="hex-separator"></span>

            <span className="hex-bytes">
              {lineBytes.map((byte, i) => {
                const byteOffset = offset + i;
                const style = getByteStyle(byteOffset);

                return (
                  <span
                    key={i}
                    className="hex-byte"
                    style={style}
                    onMouseDown={() => handleByteMouseDown(byteOffset)}
                    onMouseEnter={() => handleByteMouseEnter(byteOffset)}
                    title={`Offset: 0x${byteOffset.toString(16).toUpperCase()}, Value: 0x${byte.toString(16).toUpperCase()}`}
                  >
                    {byte.toString(16).padStart(2, "0").toUpperCase()}
                    {i === 7 && <span className="hex-gap"></span>}
                  </span>
                );
              })}
              {/* Pad remaining bytes */}
              {Array.from(
                { length: BYTES_PER_LINE - lineBytes.length },
                (_, i) => (
                  <span key={`pad-${i}`} className="hex-byte hex-padding">
                    {" "}
                  </span>
                ),
              )}
            </span>

            <span className="hex-separator"></span>

            <span className="hex-ascii">{ascii}</span>
          </div>
        );
      })}

      {truncated && (
        <div className="hex-truncated">
          ... ({totalSize - hexData.length} more bytes)
        </div>
      )}

      {/* Byte info panel -- a real multi-byte range (INT-03) shows a range summary instead of
          the single-byte Value/ASCII/Binary breakdown, which only makes sense for one byte. */}
      {rangeStart !== null &&
        rangeEndInclusive !== null &&
        rangeStart < hexData.length &&
        (isMultiByteRange ? (
          <div className="hex-byte-info">
            <div className="hex-byte-info-row">
              <span className="hex-byte-info-label">Range:</span>
              <span className="hex-byte-info-value">
                0x{rangeStart.toString(16).toUpperCase()} - 0x
                {Math.min(rangeEndInclusive, hexData.length - 1)
                  .toString(16)
                  .toUpperCase()}
              </span>
            </div>
            <div className="hex-byte-info-row">
              <span className="hex-byte-info-label">Length:</span>
              <span className="hex-byte-info-value">
                {Math.min(rangeEndInclusive, hexData.length - 1) -
                  rangeStart +
                  1}{" "}
                bytes
              </span>
            </div>
          </div>
        ) : (
          <div className="hex-byte-info">
            <div className="hex-byte-info-row">
              <span className="hex-byte-info-label">Offset:</span>
              <span className="hex-byte-info-value">
                0x{rangeStart.toString(16).toUpperCase()} ({rangeStart})
              </span>
            </div>
            <div className="hex-byte-info-row">
              <span className="hex-byte-info-label">Value:</span>
              <span className="hex-byte-info-value">
                0x{hexData[rangeStart].toString(16).toUpperCase()} (
                {hexData[rangeStart]})
              </span>
            </div>
            <div className="hex-byte-info-row">
              <span className="hex-byte-info-label">ASCII:</span>
              <span className="hex-byte-info-value">
                {byteToAscii(hexData[rangeStart])}
              </span>
            </div>
            <div className="hex-byte-info-row">
              <span className="hex-byte-info-label">Binary:</span>
              <span className="hex-byte-info-value">
                {hexData[rangeStart].toString(2).padStart(8, "0")}
              </span>
            </div>
          </div>
        ))}

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenu.items}
          onSelect={handleContextMenuSelect}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
});
