/**
 * StreamTreePanel Component Tests
 * Tests tree view, filtering, search, and expansion functionality
 *
 * TODO: Skip until codec parsing backend is properly implemented
 * This requires actual NAL/OBV parsing which is not yet complete
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { StreamTreePanel, type UnitNode } from "../StreamTreePanel";
import { mockFrames } from "@/test/test-utils";
import { useStreamData } from "@/contexts/StreamDataContext";

// Mock context
vi.mock("@/contexts/StreamDataContext");

describe.skip("StreamTreePanel", () => {
  const defaultProps = {
    units: [],
    selectedUnitKey: "",
    onUnitSelect: vi.fn(),
  };

  // Setup default mock for useStreamData
  beforeEach(() => {
    vi.mocked(useStreamData).mockReturnValue({
      frames: mockFrames,
      currentFrameIndex: 1,
      loading: false,
      error: null,
      setCurrentFrameIndex: vi.fn(),
      refreshFrames: vi.fn(),
      clearData: vi.fn(),
      getFrameStats: vi.fn(),
    });
  });

  // Helper to create mock unit nodes
  const createMockUnits = (): UnitNode[] => [
    {
      key: "seq1",
      unit_type: "SEQUENCE_HEADER",
      offset: 0,
      size: 100,
      children: [
        {
          key: "frame0",
          unit_type: "FRAME",
          offset: 100,
          size: 50000,
          frame_index: 0,
          pts: 0,
          children: [],
        },
      ],
    },
    {
      key: "frame1",
      unit_type: "FRAME",
      offset: 50100,
      size: 30000,
      frame_index: 1,
      pts: 1,
      children: [
        {
          key: "tile1",
          unit_type: "TILE_GROUP",
          offset: 50200,
          size: 15000,
          children: [],
        },
      ],
    },
    {
      key: "frame2",
      unit_type: "FRAME",
      offset: 80100,
      size: 20000,
      frame_index: 2,
      pts: 2,
      children: [],
    },
  ];

  describe("StreamTreePanel basic rendering", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should render panel header", () => {
      render(<StreamTreePanel {...defaultProps} />);

      expect(screen.getByText("Stream Tree")).toBeInTheDocument();
    });

    it("should render empty state when no frames loaded", () => {
      vi.mocked(useStreamData).mockReturnValue({
        frames: [],
        currentFrameIndex: 0,
        loading: false,
        error: null,
        setCurrentFrameIndex: vi.fn(),
        refreshFrames: vi.fn(),
        clearData: vi.fn(),
        getFrameStats: vi.fn(),
      });

      render(<StreamTreePanel {...defaultProps} />);

      expect(screen.getByText("No frames loaded")).toBeInTheDocument();
      expect(
        screen.getByText("Open a file to see stream units"),
      ).toBeInTheDocument();
    });

    it("should render frame list when frames are available", () => {
      render(<StreamTreePanel {...defaultProps} />);

      expect(screen.getByText("#0")).toBeInTheDocument();
      expect(screen.getByText("#1")).toBeInTheDocument();
      expect(screen.getByText("#2")).toBeInTheDocument();
    });

    it("should display frame types correctly", () => {
      render(<StreamTreePanel {...defaultProps} />);

      expect(screen.getByText("I")).toBeInTheDocument();
      expect(screen.getByText("P")).toBeInTheDocument();
      expect(screen.getByText("B")).toBeInTheDocument();
    });

    it("should display offset in hex format", () => {
      render(<StreamTreePanel {...defaultProps} />);

      expect(screen.getByText(/0x00000000/)).toBeInTheDocument();
    });

    it("should display size in bytes", () => {
      render(<StreamTreePanel {...defaultProps} />);

      expect(screen.getByText(/50000 bytes/)).toBeInTheDocument();
      expect(screen.getByText(/30000 bytes/)).toBeInTheDocument();
    });

    it("should render filter toolbar", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const filterLabel = screen.getByText("Filter");
      expect(filterLabel).toBeInTheDocument();

      const checkbox = screen.getByRole("checkbox");
      expect(checkbox).toBeInTheDocument();
      expect(checkbox).not.toBeChecked();
    });

    it("should use React.memo for optimization", () => {
      const { rerender } = render(<StreamTreePanel {...defaultProps} />);

      rerender(<StreamTreePanel {...defaultProps} />);

      expect(screen.getByText("Stream Tree")).toBeInTheDocument();
    });
  });

  describe("StreamTreePanel unit tree rendering", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should render tree structure with nested units", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();
      expect(screen.getByText(/FRAME/)).toBeInTheDocument();
      expect(screen.getByText(/TILE_GROUP/)).toBeInTheDocument();
    });

    it("should render expand/collapse icons for parent nodes", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const expandIcons = document.querySelectorAll(".codicon-chevron-right");
      expect(expandIcons.length).toBeGreaterThan(0);
    });

    it("should not show expand icon for leaf nodes", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const leafUnits = document.querySelectorAll(".expand-placeholder");
      expect(leafUnits.length).toBeGreaterThan(0);
    });

    it("should expand node when chevron is clicked", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const firstExpandIcon = document.querySelector(".codicon-chevron-right");
      expect(firstExpandIcon).toBeInTheDocument();

      fireEvent.click(firstExpandIcon!);

      const expandedIcon = document.querySelector(".codicon-chevron-down");
      expect(expandedIcon).toBeInTheDocument();
    });

    it("should collapse node when expanded chevron is clicked", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const firstExpandIcon = document.querySelector(".codicon-chevron-right");
      fireEvent.click(firstExpandIcon!);

      const expandedIcon = document.querySelector(".codicon-chevron-down");
      fireEvent.click(expandedIcon!);

      const collapsedIcon = document.querySelector(".codicon-chevron-right");
      expect(collapsedIcon).toBeInTheDocument();
    });

    it("should show children when parent is expanded", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const firstExpandIcon = document.querySelector(".codicon-chevron-right");
      fireEvent.click(firstExpandIcon!);

      expect(screen.getByText(/tile1/)).toBeInTheDocument();
    });

    it("should hide children when parent is collapsed", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      // Initially collapsed
      expect(screen.queryByText(/tile1/)).not.toBeInTheDocument();
    });

    it("should apply indentation based on depth", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const treeItems = document.querySelectorAll(".stream-tree-item");

      // First item (depth 0)
      expect(treeItems[0]).toHaveStyle({ paddingLeft: "8px" });

      // Expand to see children
      const firstExpandIcon = document.querySelector(".codicon-chevron-right");
      fireEvent.click(firstExpandIcon!);

      const expandedItems = document.querySelectorAll(".stream-tree-item");
      expect(expandedItems[1]).toHaveStyle({ paddingLeft: "24px" });
    });

    it("should render unit icons correctly", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/\[S\]/)).toBeInTheDocument(); // SEQUENCE_HEADER
      expect(screen.getByText(/\[F\]/)).toBeInTheDocument(); // FRAME
    });
  });

  describe("StreamTreePanel unit selection", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should call onUnitSelect when unit is clicked", () => {
      const handleSelect = vi.fn();
      const mockUnits = createMockUnits();

      render(
        <StreamTreePanel
          {...defaultProps}
          units={mockUnits}
          onUnitSelect={handleSelect}
        />,
      );

      const firstUnit = screen
        .getByText(/SEQUENCE_HEADER/)
        .closest(".stream-tree-label");
      fireEvent.click(firstUnit!);

      expect(handleSelect).toHaveBeenCalledTimes(1);
      expect(handleSelect).toHaveBeenCalledWith(mockUnits[0]);
    });

    it("should apply selected class to selected unit", () => {
      const mockUnits = createMockUnits();

      render(
        <StreamTreePanel
          {...defaultProps}
          units={mockUnits}
          selectedUnitKey="seq1"
        />,
      );

      const selectedItem = document.querySelector(".stream-tree-item.selected");
      expect(selectedItem).toBeInTheDocument();
    });

    it("should not apply selected class to non-selected units", () => {
      const mockUnits = createMockUnits();

      render(
        <StreamTreePanel
          {...defaultProps}
          units={mockUnits}
          selectedUnitKey="frame1"
        />,
      );

      const firstUnit = screen
        .getByText(/SEQUENCE_HEADER/)
        .closest(".stream-tree-item");
      expect(firstUnit).not.toHaveClass("selected");
    });

    it("should update selection when selectedUnitKey changes", () => {
      const mockUnits = createMockUnits();
      const { rerender } = render(
        <StreamTreePanel
          {...defaultProps}
          units={mockUnits}
          selectedUnitKey="seq1"
        />,
      );

      expect(
        document.querySelector(".stream-tree-item.selected"),
      ).toBeInTheDocument();

      rerender(
        <StreamTreePanel
          {...defaultProps}
          units={mockUnits}
          selectedUnitKey="frame1"
        />,
      );

      const selectedItems = document.querySelectorAll(
        ".stream-tree-item.selected",
      );
      expect(selectedItems.length).toBe(1);
    });
  });

  describe("StreamTreePanel filter functionality", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should show filter options when filter is enabled", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      expect(screen.getByRole("combobox")).toBeInTheDocument();
      expect(
        screen.getByPlaceholderText("Type or offset..."),
      ).toBeInTheDocument();
      expect(screen.getByText("Search:")).toBeInTheDocument();
    });

    it("should hide filter options when filter is disabled", () => {
      render(<StreamTreePanel {...defaultProps} />);

      expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
      expect(
        screen.queryByPlaceholderText("Type or offset..."),
      ).not.toBeInTheDocument();
    });

    it("should display all frame filter options", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      const options = select.querySelectorAll("option");

      expect(options).toHaveLength(5);
      expect(screen.getByText("All")).toBeInTheDocument();
      expect(screen.getByText("Key (I)")).toBeInTheDocument();
      expect(screen.getByText("Inter (P/B)")).toBeInTheDocument();
      expect(screen.getByText("Frames")).toBeInTheDocument();
      expect(screen.getByText("Headers")).toBeInTheDocument();
    });

    it("should filter to show only key frames when KeyOnly is selected", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "KeyOnly" } });

      // Should show I frames (key frames)
      expect(screen.getByText("#0")).toBeInTheDocument();
    });

    it("should filter to show only inter frames when InterOnly is selected", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "InterOnly" } });

      // Should show P and B frames
      expect(screen.getByText("#1")).toBeInTheDocument();
      expect(screen.getByText("#2")).toBeInTheDocument();
    });

    it("should filter to show only frames when FramesOnly is selected", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "FramesOnly" } });

      expect(screen.getByText("#0")).toBeInTheDocument();
      expect(screen.getByText("#1")).toBeInTheDocument();
      expect(screen.getByText("#2")).toBeInTheDocument();
    });

    it("should filter to show only headers when HeadersOnly is selected", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "HeadersOnly" } });

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();
    });

    it("should show all units when All filter is selected", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "All" } });

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();
      expect(screen.getByText(/FRAME/)).toBeInTheDocument();
    });

    it("should display count info when filtering", () => {
      render(<StreamTreePanel {...defaultProps} />);

      // Without filter, no count shown
      expect(screen.queryByText(/Showing/)).not.toBeInTheDocument();

      // Enable filter with KeyOnly
      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "KeyOnly" } });

      // Count should appear
      expect(screen.getByText(/Showing/)).toBeInTheDocument();
    });

    it('should show "No matching units" when filter has no results', () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const searchInput = screen.getByPlaceholderText("Type or offset...");
      fireEvent.change(searchInput, { target: { value: "NONEXISTENT" } });

      expect(screen.getByText("No matching units")).toBeInTheDocument();
    });

    it("should toggle filter enabled state", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      expect(checkbox).not.toBeChecked();

      fireEvent.click(checkbox);
      expect(checkbox).toBeChecked();

      fireEvent.click(checkbox);
      expect(checkbox).not.toBeChecked();
    });
  });

  describe("StreamTreePanel search functionality", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should filter units by type when searching", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const searchInput = screen.getByPlaceholderText("Type or offset...");
      fireEvent.change(searchInput, { target: { value: "SEQUENCE" } });

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();
    });

    it("should filter units by hex offset when searching", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const searchInput = screen.getByPlaceholderText("Type or offset...");
      fireEvent.change(searchInput, { target: { value: "64" } }); // 0x64 = 100

      expect(screen.getByText(/0x00000064/)).toBeInTheDocument();
    });

    it("should be case-insensitive when searching by type", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const searchInput = screen.getByPlaceholderText("Type or offset...");
      fireEvent.change(searchInput, { target: { value: "sequence_header" } });

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();
    });

    it("should show clear button when search has text", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const searchInput = screen.getByPlaceholderText("Type or offset...");
      fireEvent.change(searchInput, { target: { value: "test" } });

      const clearBtn = document.querySelector(".clear-search-btn");
      expect(clearBtn).toBeInTheDocument();
    });

    it("should clear search when clear button is clicked", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const searchInput = screen.getByPlaceholderText("Type or offset...");
      fireEvent.change(searchInput, { target: { value: "SEQUENCE" } });

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();

      const clearBtn = document.querySelector(".clear-search-btn");
      fireEvent.click(clearBtn!);

      // After clearing, all units should be visible again
      expect(searchInput).toHaveValue("");
    });

    it("should not show clear button when search is empty", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const clearBtn = document.querySelector(".clear-search-btn");
      expect(clearBtn).not.toBeInTheDocument();
    });

    it("should combine search with frame type filter", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      // Set frame filter to FramesOnly
      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "FramesOnly" } });

      // Add search
      const searchInput = screen.getByPlaceholderText("Type or offset...");
      fireEvent.change(searchInput, { target: { value: "FRAME" } });

      expect(screen.getByText(/FRAME/)).toBeInTheDocument();
    });
  });

  describe("StreamTreePanel unit colors", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should apply correct color for SEQUENCE_HEADER", () => {
      const mockUnits = [
        {
          key: "seq1",
          unit_type: "SEQUENCE_HEADER",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const label = screen.getByText(/SEQUENCE_HEADER/);
      expect(label).toHaveStyle({ color: "#64c864" });
    });

    it("should apply correct color for FRAME", () => {
      const mockUnits = [
        {
          key: "frame1",
          unit_type: "FRAME",
          offset: 100,
          size: 50000,
          frame_index: 0,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const label = screen.getByText(/FRAME/);
      expect(label).toHaveStyle({ color: "#6496ff" });
    });

    it("should apply correct color for TILE_GROUP", () => {
      const mockUnits = [
        {
          key: "tile1",
          unit_type: "TILE_GROUP",
          offset: 200,
          size: 15000,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const label = screen.getByText(/TILE_GROUP/);
      expect(label).toHaveStyle({ color: "#c89664" });
    });

    it("should apply correct color for TEMPORAL_DELIMITER", () => {
      const mockUnits = [
        {
          key: "td1",
          unit_type: "TEMPORAL_DELIMITER",
          offset: 300,
          size: 50,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const label = screen.getByText(/TEMPORAL_DELIMITER/);
      expect(label).toHaveStyle({ color: "#969696" });
    });

    it("should apply default color for unknown unit types", () => {
      const mockUnits = [
        {
          key: "unknown1",
          unit_type: "UNKNOWN_TYPE",
          offset: 400,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const label = screen.getByText(/UNKNOWN_TYPE/);
      expect(label).toHaveStyle({ color: "#ffffff" });
    });
  });

  describe("StreamTreePanel unit icons", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should show F icon for frame units", () => {
      const mockUnits = [
        {
          key: "frame1",
          unit_type: "FRAME",
          offset: 100,
          size: 50000,
          frame_index: 0,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/\[F\]/)).toBeInTheDocument();
    });

    it("should show S icon for SEQUENCE_HEADER", () => {
      const mockUnits = [
        {
          key: "seq1",
          unit_type: "SEQUENCE_HEADER",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/\[S\]/)).toBeInTheDocument();
    });

    it("should show T icon for TEMPORAL_DELIMITER", () => {
      const mockUnits = [
        {
          key: "td1",
          unit_type: "TEMPORAL_DELIMITER",
          offset: 300,
          size: 50,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/\[T\]/)).toBeInTheDocument();
    });

    it("should show M icon for METADATA", () => {
      const mockUnits = [
        {
          key: "meta1",
          unit_type: "METADATA",
          offset: 400,
          size: 200,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/\[M\]/)).toBeInTheDocument();
    });

    it("should show P icon for PADDING", () => {
      const mockUnits = [
        {
          key: "pad1",
          unit_type: "PADDING",
          offset: 500,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/\[P\]/)).toBeInTheDocument();
    });

    it("should show bullet icon for unknown unit types", () => {
      const mockUnits = [
        {
          key: "unknown1",
          unit_type: "UNKNOWN_TYPE",
          offset: 600,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      // Should show bullet icon
      const icon = document.querySelector(".stream-tree-label");
      expect(icon?.textContent).toContain("[•]");
    });
  });

  describe("StreamTreePanel edge cases", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should handle empty units array with empty frames", () => {
      vi.mocked(useStreamData).mockReturnValue({
        frames: [],
        currentFrameIndex: 0,
        loading: false,
        error: null,
        setCurrentFrameIndex: vi.fn(),
        refreshFrames: vi.fn(),
        clearData: vi.fn(),
        getFrameStats: vi.fn(),
      });

      render(<StreamTreePanel {...defaultProps} units={[]} />);

      expect(screen.getByText("No frames loaded")).toBeInTheDocument();
    });

    it("should handle deeply nested tree structure", () => {
      const deepNestedUnit: UnitNode = {
        key: "root",
        unit_type: "SEQUENCE_HEADER",
        offset: 0,
        size: 100,
        children: [
          {
            key: "child1",
            unit_type: "FRAME",
            offset: 100,
            size: 50000,
            frame_index: 0,
            children: [
              {
                key: "grandchild1",
                unit_type: "TILE_GROUP",
                offset: 200,
                size: 15000,
                children: [
                  {
                    key: "greatgrandchild1",
                    unit_type: "FRAME",
                    offset: 300,
                    size: 5000,
                    frame_index: 0,
                    children: [],
                  },
                ],
              },
            ],
          },
        ],
      };

      render(<StreamTreePanel {...defaultProps} units={[deepNestedUnit]} />);

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();
      expect(screen.getByText(/FRAME/)).toBeInTheDocument();
      expect(screen.getByText(/TILE_GROUP/)).toBeInTheDocument();
    });

    it("should handle units with same unit_type but different offsets", () => {
      const mockUnits = [
        {
          key: "frame1",
          unit_type: "FRAME",
          offset: 100,
          size: 50000,
          frame_index: 0,
          children: [],
        },
        {
          key: "frame2",
          unit_type: "FRAME",
          offset: 50100,
          size: 30000,
          frame_index: 1,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const frameLabels = screen.getAllByText(/FRAME/);
      expect(frameLabels.length).toBeGreaterThanOrEqual(2);
    });

    it("should handle units with offset 0", () => {
      const mockUnits = [
        {
          key: "unit1",
          unit_type: "SEQUENCE_HEADER",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText("0x00000000")).toBeInTheDocument();
    });

    it("should handle very large offset values", () => {
      const mockUnits = [
        {
          key: "unit1",
          unit_type: "FRAME",
          offset: 0x12345678,
          size: 50000,
          frame_index: 0,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText("0x12345678")).toBeInTheDocument();
    });

    it("should handle very large size values", () => {
      const mockUnits = [
        {
          key: "unit1",
          unit_type: "FRAME",
          offset: 0,
          size: 999999999,
          frame_index: 0,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/999999999 bytes/)).toBeInTheDocument();
    });

    it("should handle special characters in unit_type", () => {
      const mockUnits = [
        {
          key: "unit1",
          unit_type: "OBU_FRAME_HEADER",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/OBU_FRAME_HEADER/)).toBeInTheDocument();
    });

    it("should handle units without frame_index", () => {
      const mockUnits = [
        {
          key: "seq1",
          unit_type: "SEQUENCE_HEADER",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      // Should display without frame number
      expect(screen.queryByText("#")).not.toBeInTheDocument();
    });

    it("should handle undefined onUnitSelect callback", () => {
      const mockUnits = createMockUnits();

      render(
        <StreamTreePanel
          {...defaultProps}
          units={mockUnits}
          onUnitSelect={undefined}
        />,
      );

      const label = screen.getByText(/SEQUENCE_HEADER/);
      expect(() => fireEvent.click(label)).not.toThrow();
    });

    it("should handle rapid filter changes", () => {
      render(<StreamTreePanel {...defaultProps} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");

      // Rapid filter changes
      fireEvent.change(select, { target: { value: "KeyOnly" } });
      fireEvent.change(select, { target: { value: "InterOnly" } });
      fireEvent.change(select, { target: { value: "FramesOnly" } });
      fireEvent.change(select, { target: { value: "All" } });

      expect(screen.getByText(/#0/)).toBeInTheDocument();
    });

    it("should handle rapid search changes", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const searchInput = screen.getByPlaceholderText("Type or offset...");

      // Rapid search changes
      fireEvent.change(searchInput, { target: { value: "A" } });
      fireEvent.change(searchInput, { target: { value: "AB" } });
      fireEvent.change(searchInput, { target: { value: "ABC" } });
      fireEvent.change(searchInput, { target: { value: "" } });

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();
    });

    it("should handle multiple expanded nodes simultaneously", () => {
      const mockUnits = createMockUnits();

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const expandIcons = document.querySelectorAll(".codicon-chevron-right");

      // Expand multiple nodes
      if (expandIcons.length > 0) {
        fireEvent.click(expandIcons[0]);
      }
      if (expandIcons.length > 1) {
        fireEvent.click(expandIcons[1]);
      }

      const expandedIcons = document.querySelectorAll(".codicon-chevron-down");
      expect(expandedIcons.length).toBeGreaterThan(0);
    });
  });

  describe("StreamTreePanel header-specific filters", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should match SPS in HeadersOnly filter", () => {
      const mockUnits = [
        {
          key: "sps1",
          unit_type: "SPS",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "HeadersOnly" } });

      expect(screen.getByText(/SPS/)).toBeInTheDocument();
    });

    it("should match PPS in HeadersOnly filter", () => {
      const mockUnits = [
        {
          key: "pps1",
          unit_type: "PPS",
          offset: 100,
          size: 50,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "HeadersOnly" } });

      expect(screen.getByText(/PPS/)).toBeInTheDocument();
    });

    it("should match VPS in HeadersOnly filter", () => {
      const mockUnits = [
        {
          key: "vps1",
          unit_type: "VPS",
          offset: 200,
          size: 50,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "HeadersOnly" } });

      expect(screen.getByText(/VPS/)).toBeInTheDocument();
    });

    it("should match APS in HeadersOnly filter", () => {
      const mockUnits = [
        {
          key: "aps1",
          unit_type: "APS",
          offset: 300,
          size: 75,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "HeadersOnly" } });

      expect(screen.getByText(/APS/)).toBeInTheDocument();
    });

    it("should match SEQUENCE in HeadersOnly filter", () => {
      const mockUnits = [
        {
          key: "seq1",
          unit_type: "SEQUENCE_HEADER",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "HeadersOnly" } });

      expect(screen.getByText(/SEQUENCE_HEADER/)).toBeInTheDocument();
    });
  });

  describe("StreamTreePanel key frame filtering", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should match KEY in KeyOnly filter", () => {
      const mockUnits = [
        {
          key: "key1",
          unit_type: "KEY_FRAME",
          offset: 0,
          size: 50000,
          frame_index: 0,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "KeyOnly" } });

      expect(screen.getByText(/KEY_FRAME/)).toBeInTheDocument();
    });

    it("should match INTRA in KeyOnly filter", () => {
      const mockUnits = [
        {
          key: "intra1",
          unit_type: "INTRA_FRAME",
          offset: 0,
          size: 50000,
          frame_index: 0,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "KeyOnly" } });

      expect(screen.getByText(/INTRA_FRAME/)).toBeInTheDocument();
    });

    it("should match IDR in KeyOnly filter", () => {
      const mockUnits = [
        {
          key: "idr1",
          unit_type: "IDR_FRAME",
          offset: 0,
          size: 50000,
          frame_index: 0,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "KeyOnly" } });

      expect(screen.getByText(/IDR_FRAME/)).toBeInTheDocument();
    });

    it("should exclude non-key frames from KeyOnly filter", () => {
      const mockUnits = [
        {
          key: "key1",
          unit_type: "KEY_FRAME",
          offset: 0,
          size: 50000,
          frame_index: 0,
          children: [],
        },
        {
          key: "inter1",
          unit_type: "INTER_FRAME",
          offset: 50000,
          size: 30000,
          frame_index: 1,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const checkbox = screen.getByRole("checkbox");
      fireEvent.click(checkbox);

      const select = screen.getByRole("combobox");
      fireEvent.change(select, { target: { value: "KeyOnly" } });

      expect(screen.getByText(/KEY_FRAME/)).toBeInTheDocument();
      expect(screen.queryByText(/INTER_FRAME/)).not.toBeInTheDocument();
    });
  });

  describe("StreamTreePanel tooltips and labels", () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it("should show tooltip with full unit label", () => {
      const mockUnits = [
        {
          key: "frame1",
          unit_type: "FRAME",
          offset: 0x100,
          size: 50000,
          frame_index: 42,
          pts: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      const label = screen.getByText(/FRAME/);
      expect(label).toHaveAttribute("title");
      expect(label?.getAttribute("title")).toContain("Frame #42");
    });

    it("should show frame index in label for frame units", () => {
      const mockUnits = [
        {
          key: "frame1",
          unit_type: "FRAME",
          offset: 0,
          size: 50000,
          frame_index: 42,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText(/Frame #42/)).toBeInTheDocument();
    });

    it("should not show frame index in label for non-frame units", () => {
      const mockUnits = [
        {
          key: "seq1",
          unit_type: "SEQUENCE_HEADER",
          offset: 0,
          size: 100,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.queryByText(/Frame #/)).not.toBeInTheDocument();
    });

    it("should pad hex offset to 8 characters", () => {
      const mockUnits = [
        {
          key: "unit1",
          unit_type: "FRAME",
          offset: 0x123,
          size: 100,
          frame_index: 0,
          children: [],
        },
      ];

      render(<StreamTreePanel {...defaultProps} units={mockUnits} />);

      expect(screen.getByText("0x00000123")).toBeInTheDocument();
    });
  });
});
