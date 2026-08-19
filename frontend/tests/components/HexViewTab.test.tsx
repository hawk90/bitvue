/**
 * Hex View Tab Component Tests
 *
 * Covers the 2026-08-09 rewiring off Tauri's get_frame_hex_data invoke() to the already-proven
 * getHexRange bridge call (no new backend work needed -- HexViewTab now resolves a frame's real
 * on-disk offset via FrameInfo.offset, added in an earlier round, then reuses getHexRange
 * exactly as get_hex_range's own byte-exact tests already established it works).
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@/test/test-utils";
import { HexViewTab } from "../HexViewTab";

const { getHexRange } = vi.hoisted(() => ({
  getHexRange: vi.fn(),
}));

vi.mock("@/services/electronBridgeService", () => ({ getHexRange }));

/** Deterministic bytes varying by offset, mirroring the old Tauri mock's frame_index-based
 *  variation closely enough for the "different frames produce different bytes" test below --
 *  not a real decoder, just needs a start-code-shaped, offset-dependent pattern. */
function generateMockBytes(offset: number, len: number): Uint8Array {
  const data = new Uint8Array(len);
  data[0] = 0x00;
  data[1] = 0x00;
  data[2] = 0x01;
  for (let i = 3; i < len; i++) {
    if (i === 3) {
      data[i] = 0x10 + (offset % 8);
    } else if (i < 20) {
      data[i] = 0x20 + ((i + offset) % 64);
    } else {
      data[i] = (i * 7 + offset * 13) % 256;
    }
  }
  return data;
}

const mockFrames = [
  { frame_index: 0, size: 100, offset: 1000 },
  { frame_index: 1, size: 200, offset: 2000 },
  { frame_index: 2, size: 150, offset: 3000 },
];

describe("HexViewTab", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getHexRange.mockImplementation(
      (_stream: string, offset: number, len: number) =>
        Promise.resolve({ offset, len, bytes: generateMockBytes(offset, len) }),
    );
  });

  it("should render empty state when no frames available", () => {
    render(<HexViewTab frameIndex={0} frames={[]} />);

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });

  it("should render empty state icon when no frames", () => {
    const { container } = render(<HexViewTab frameIndex={0} frames={[]} />);

    expect(container.querySelector(".codicon-file-code")).toBeInTheDocument();
  });

  it("should render empty state when frame index out of bounds", () => {
    render(<HexViewTab frameIndex={99} frames={mockFrames} />);

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
  });

  it("fetches the frame's real on-disk byte range, not a frame index", async () => {
    render(<HexViewTab frameIndex={0} frames={mockFrames} />);

    await waitFor(() => {
      expect(getHexRange).toHaveBeenCalledWith("A", 1000, 100);
    });
  });

  it("shows an honest error when the frame has no on-disk offset", async () => {
    const frameWithoutOffset = { frame_index: 0, size: 100 };
    render(<HexViewTab frameIndex={0} frames={[frameWithoutOffset]} />);

    await waitFor(() => {
      expect(
        screen.getByText(/No on-disk offset available/),
      ).toBeInTheDocument();
    });
    expect(getHexRange).not.toHaveBeenCalled();
  });

  it("should render hex dump content for valid frame", async () => {
    render(<HexViewTab frameIndex={0} frames={mockFrames} />);

    // Wait for async data loading
    await waitFor(() => {
      expect(
        screen.getByText(/All 100 bytes|First 100 bytes/),
      ).toBeInTheDocument();
    });
  });

  it("should display frame size", async () => {
    render(<HexViewTab frameIndex={0} frames={mockFrames} />);

    await waitFor(() => {
      const text = screen.getByText(/100 bytes/);
      expect(text).toBeInTheDocument();
    });
  });

  it("should render hex lines", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const hexLines = container.querySelectorAll(".hex-line");
      expect(hexLines.length).toBeGreaterThan(0);
    });
  });

  it("should render hex offset in uppercase", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const firstOffset = container.querySelector(".hex-offset");
      expect(firstOffset?.textContent).toBe("00000000");
    });
  });

  it("should render hex bytes in uppercase", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const firstByte = container.querySelector(".hex-byte");
      expect(firstByte?.textContent).toMatch(/^[0-9A-F]{2}$/);
    });
  });

  it("should render 16 bytes per line", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const firstLine = container.querySelector(".hex-line");
      const bytes = firstLine?.querySelectorAll(".hex-byte");
      // Should have 16 bytes total (including padding)
      expect(bytes?.length).toBe(16);
    });
  });

  it("should add gap after 8th byte", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const firstLine = container.querySelector(".hex-line");
      const gaps = firstLine?.querySelectorAll(".hex-gap");
      expect(gaps?.length).toBe(1);
    });
  });

  it("should render ASCII representation", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const ascii = container.querySelector(".hex-ascii");
      expect(ascii).toBeInTheDocument();
      expect(ascii?.textContent?.length).toBeGreaterThan(0);
    });
  });

  it("should show truncated message for frames larger than MAX_HEX_BYTES", async () => {
    const largeFrame = { frame_index: 0, size: 3000, offset: 5000 };
    render(<HexViewTab frameIndex={0} frames={[largeFrame]} />);

    await waitFor(() => {
      expect(screen.getByText(/\(\d+ more bytes\)/)).toBeInTheDocument();
    });
    // MAX_HEX_BYTES caps the request length even though the frame itself is bigger.
    expect(getHexRange).toHaveBeenCalledWith("A", 5000, 2048);
  });

  it("should handle byte selection", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(async () => {
      const firstByte = await waitFor(() =>
        container.querySelector(".hex-byte"),
      );
      if (firstByte) {
        fireEvent.click(firstByte);
        // Click should not error - internal state updates
        expect(firstByte).toBeInTheDocument();
      }
    });
  });

  it("should color start code bytes red", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const bytes = container.querySelectorAll(".hex-byte");
      if (bytes.length > 0) {
        const firstByte = bytes[0] as HTMLElement;
        const firstByteStyle = firstByte.style.color;
        // Start codes should be colored (not empty)
        expect(firstByteStyle).toBeTruthy();
      }
    });
  });

  it("should highlight selected byte", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(async () => {
      const bytes = await waitFor(() =>
        container.querySelectorAll(".hex-byte"),
      );
      if (bytes.length > 0) {
        fireEvent.click(bytes[0]);
        expect(bytes[0]).toBeInTheDocument();
      }
    });
  });

  it("should render info bar with data indicator", async () => {
    render(<HexViewTab frameIndex={0} frames={mockFrames} />);

    await waitFor(() => {
      expect(screen.getByText("Data:")).toBeInTheDocument();
    });
  });

  it("should render separators between sections", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const separators = container.querySelectorAll(".hex-separator");
      expect(separators.length).toBeGreaterThan(0);
    });
  });

  it("should handle frames smaller than 512 bytes", async () => {
    const smallFrame = { frame_index: 0, size: 50, offset: 1000 };
    const { container } = render(
      <HexViewTab frameIndex={0} frames={[smallFrame]} />,
    );

    await waitFor(() => {
      const lines = container.querySelectorAll(".hex-line");
      expect(lines.length).toBeGreaterThan(0);
    });

    // Should not show truncated message
    expect(screen.queryByText(/\(\d+ more bytes\)/)).not.toBeInTheDocument();
  });

  it("should use frame offset for mock data generation", async () => {
    // Render frame 0 and capture first hex byte
    const { container: container1, unmount: unmount1 } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    let hexBytes1: string;
    await waitFor(() => {
      const bytes = container1.querySelectorAll(".hex-byte");
      expect(bytes.length).toBeGreaterThan(3);
      // Join all byte values - offset affects byte at position 3+
      hexBytes1 = Array.from(bytes)
        .map((b) => b.textContent)
        .join("");
    });

    unmount1();

    // Render frame 1 and capture hex bytes
    const { container: container2 } = render(
      <HexViewTab frameIndex={1} frames={mockFrames} />,
    );

    await waitFor(() => {
      const bytes = container2.querySelectorAll(".hex-byte");
      expect(bytes.length).toBeGreaterThan(3);
      const hexBytes2 = Array.from(bytes)
        .map((b) => b.textContent)
        .join("");
      // Different frame offsets should generate different mock data
      expect(hexBytes2).not.toBe(hexBytes1!);
    });
  });

  it("should handle frame size exactly 512 bytes", async () => {
    const exactFrame = { frame_index: 0, size: 512, offset: 1000 };
    render(<HexViewTab frameIndex={0} frames={[exactFrame]} />);

    // Should not show truncated message when exactly 512
    await waitFor(() => {
      expect(screen.queryByText(/\(\d+ more bytes\)/)).not.toBeInTheDocument();
    });
  });

  it("should render all sections for each line", async () => {
    const { container } = render(
      <HexViewTab frameIndex={0} frames={mockFrames} />,
    );

    await waitFor(() => {
      const firstLine = container.querySelector(".hex-line");
      expect(firstLine?.querySelector(".hex-offset")).toBeInTheDocument();
      expect(firstLine?.querySelectorAll(".hex-separator")).toHaveLength(2);
      expect(firstLine?.querySelector(".hex-bytes")).toBeInTheDocument();
      expect(firstLine?.querySelector(".hex-ascii")).toBeInTheDocument();
    });
  });

  it("should show loading state initially", () => {
    // Make the bridge call never resolve so we can observe the loading state
    getHexRange.mockImplementationOnce(() => new Promise(() => {}));

    render(<HexViewTab frameIndex={0} frames={mockFrames} />);

    // Should show loading initially (effect sets loading=true before awaiting the bridge call)
    expect(screen.getByText("Loading hex data...")).toBeInTheDocument();
  });
});
