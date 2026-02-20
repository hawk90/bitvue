/**
 * EmptyState Component Tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@/test/test-utils";
import {
  EmptyState,
  NoFileLoaded,
  NoFramesSelected,
  NoSearchResults,
  NoReferenceFrames,
  PanelEmptyState,
} from "@/components/common/EmptyState";

describe("EmptyState", () => {
  it("should render with all props", () => {
    render(
      <EmptyState
        icon="codicon-search"
        title="Test Title"
        description="Test Description"
        size="md"
      />,
    );

    expect(screen.getByText("Test Title")).toBeInTheDocument();
    expect(screen.getByText("Test Description")).toBeInTheDocument();
  });

  it("should render without description", () => {
    render(<EmptyState icon="codicon-search" title="Test Title" size="sm" />);

    expect(screen.getByText("Test Title")).toBeInTheDocument();
    expect(screen.queryByText("Test Description")).not.toBeInTheDocument();
  });

  it("should render with action button", () => {
    const handleClick = vi.fn();
    render(
      <EmptyState
        icon="codicon-search"
        title="Test Title"
        action={{ label: "Click Me", onClick: handleClick }}
        size="lg"
      />,
    );

    const button = screen.getByRole("button", { name: "Click Me" });
    expect(button).toBeInTheDocument();

    button.click();
    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it("should apply correct size class", () => {
    const { container } = render(
      <EmptyState icon="codicon-search" title="Test Title" size="lg" />,
    );

    const emptyState = container.querySelector(".empty-state");
    expect(emptyState).toHaveClass("gap-4"); // lg size uses gap-4
  });
});

describe("NoFileLoaded", () => {
  it("should render with open file action", () => {
    const handleClick = vi.fn();
    render(<NoFileLoaded onOpenFile={handleClick} />);

    expect(screen.getByText("No file loaded")).toBeInTheDocument();
    expect(
      screen.getByText("Open a video bitstream file to begin analysis"),
    ).toBeInTheDocument();

    const button = screen.getByRole("button", { name: "Open File" });
    button.click();
    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it("should render without action when onOpenFile is not provided", () => {
    render(<NoFileLoaded />);

    expect(screen.getByText("No file loaded")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });
});

describe("NoFramesSelected", () => {
  it("should render correctly", () => {
    render(<NoFramesSelected />);

    expect(screen.getByText("No frame selected")).toBeInTheDocument();
    expect(
      screen.getByText("Select a frame to view detailed information"),
    ).toBeInTheDocument();
  });
});

describe("NoSearchResults", () => {
  it("should render correctly", () => {
    render(<NoSearchResults />);

    expect(screen.getByText("No results found")).toBeInTheDocument();
    expect(
      screen.getByText("Try adjusting your search criteria"),
    ).toBeInTheDocument();
  });
});

describe("NoReferenceFrames", () => {
  it("should render correctly", () => {
    render(<NoReferenceFrames />);

    expect(screen.getByText("No reference frames")).toBeInTheDocument();
    expect(
      screen.getByText("This frame does not reference any other frames"),
    ).toBeInTheDocument();
  });
});

describe("PanelEmptyState", () => {
  it("should render with panel name", () => {
    render(<PanelEmptyState panelName="Test Panel" />);

    expect(screen.getByText("Test Panel unavailable")).toBeInTheDocument();
    expect(
      screen.getByText("Load a file to view this panel"),
    ).toBeInTheDocument();
  });
});
