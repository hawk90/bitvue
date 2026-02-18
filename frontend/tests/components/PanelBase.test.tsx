/**
 * PanelBase Component Tests
 * Tests base panel component and sub-components
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import {
  PanelBase,
  PanelSection,
  PanelInfoRow,
  PanelEmpty,
  PanelLoading,
} from "../PanelBase";

describe("PanelBase", () => {
  it("should render panel with title and content", () => {
    render(
      <PanelBase title="Test Panel" onClose={vi.fn()}>
        <div>Panel Content</div>
      </PanelBase>,
    );

    expect(screen.getByText("Test Panel")).toBeInTheDocument();
    expect(screen.getByText("Panel Content")).toBeInTheDocument();
  });

  it("should render panel with icon", () => {
    render(
      <PanelBase title="Test Panel" icon="codicon-search" onClose={vi.fn()}>
        <div>Content</div>
      </PanelBase>,
    );

    const icon = document.querySelector(".codicon-search");
    expect(icon).toBeInTheDocument();
  });

  it("should render close button and trigger onClose", () => {
    const handleClose = vi.fn();
    render(
      <PanelBase title="Test Panel" onClose={handleClose}>
        <div>Content</div>
      </PanelBase>,
    );

    const closeButton = screen.getByRole("button", { name: "Close" });
    fireEvent.click(closeButton);

    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it("should not render when visible is false", () => {
    const { container } = render(
      <PanelBase title="Test Panel" visible={false} onClose={vi.fn()}>
        <div>Content</div>
      </PanelBase>,
    );

    expect(container.firstChild).toBe(null);
  });

  it("should render footer when provided", () => {
    render(
      <PanelBase
        title="Test Panel"
        onClose={vi.fn()}
        footer={<div>Footer Content</div>}
      >
        <div>Content</div>
      </PanelBase>,
    );

    expect(screen.getByText("Footer Content")).toBeInTheDocument();
  });

  it("should apply size variant classes", () => {
    const { container: smContainer } = render(
      <PanelBase title="Small" size="sm" onClose={vi.fn()}>
        <div>Content</div>
      </PanelBase>,
    );
    const { container: lgContainer } = render(
      <PanelBase title="Large" size="lg" onClose={vi.fn()}>
        <div>Content</div>
      </PanelBase>,
    );

    expect(smContainer.querySelector(".panel--sm")).toBeInTheDocument();
    expect(lgContainer.querySelector(".panel--lg")).toBeInTheDocument();
  });

  it("should apply custom className", () => {
    const { container } = render(
      <PanelBase title="Test Panel" className="custom-class" onClose={vi.fn()}>
        <div>Content</div>
      </PanelBase>,
    );

    const panel = container.querySelector(".panel-container");
    expect(panel).toHaveClass("custom-class");
  });

  it("should render headerExtra content", () => {
    render(
      <PanelBase
        title="Test Panel"
        headerExtra={<span data-testid="extra">Extra</span>}
        onClose={vi.fn()}
      >
        <div>Content</div>
      </PanelBase>,
    );

    expect(screen.getByTestId("extra")).toBeInTheDocument();
  });

  it("should hide close button when showCloseButton is false", () => {
    render(
      <PanelBase title="Test Panel" showCloseButton={false} onClose={vi.fn()}>
        <div>Content</div>
      </PanelBase>,
    );

    const closeButton = screen.queryByRole("button", { name: "Close" });
    expect(closeButton).not.toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <PanelBase title="Test Panel" onClose={vi.fn()}>
        <div>Content</div>
      </PanelBase>,
    );

    rerender(
      <PanelBase title="Test Panel" onClose={vi.fn()}>
        <div>Content</div>
      </PanelBase>,
    );

    expect(screen.getByText("Test Panel")).toBeInTheDocument();
  });
});

describe("PanelSection", () => {
  it("should render section with title", () => {
    render(
      <PanelSection title="Section Title">
        <div>Section Content</div>
      </PanelSection>,
    );

    expect(screen.getByText("Section Title")).toBeInTheDocument();
    expect(screen.getByText("Section Content")).toBeInTheDocument();
  });

  it("should render section without title", () => {
    render(
      <PanelSection>
        <div>Content</div>
      </PanelSection>,
    );

    expect(screen.getByText("Content")).toBeInTheDocument();
    expect(screen.queryByText("Section Title")).not.toBeInTheDocument();
  });

  it("should apply custom className", () => {
    const { container } = render(
      <PanelSection className="custom-section">
        <div>Content</div>
      </PanelSection>,
    );

    const section = container.querySelector(".panel-section");
    expect(section).toHaveClass("custom-section");
  });
});

describe("PanelInfoRow", () => {
  it("should render label and value", () => {
    render(<PanelInfoRow label="Test Label" value="Test Value" />);

    expect(screen.getByText("Test Label")).toBeInTheDocument();
    expect(screen.getByText("Test Value")).toBeInTheDocument();
  });

  it("should render complex value as ReactNode", () => {
    render(
      <PanelInfoRow
        label="Status"
        value={<span className="badge">Active</span>}
      />,
    );

    expect(screen.getByText("Status")).toBeInTheDocument();
    expect(screen.getByText("Active")).toBeInTheDocument();
    expect(document.querySelector(".badge")).toBeInTheDocument();
  });

  it("should apply custom className", () => {
    const { container } = render(
      <PanelInfoRow label="Test" value="Value" className="custom-row" />,
    );

    const row = container.querySelector(".panel-info-row");
    expect(row).toHaveClass("custom-row");
  });
});

describe("PanelEmpty", () => {
  it("should render empty state with message", () => {
    render(<PanelEmpty message="No data available" />);

    expect(screen.getByText("No data available")).toBeInTheDocument();
  });

  it("should render with default icon", () => {
    const { container } = render(<PanelEmpty message="Empty" />);

    const icon = container.querySelector(".codicon-circle-slash");
    expect(icon).toBeInTheDocument();
  });

  it("should render with custom icon", () => {
    const { container } = render(
      <PanelEmpty message="Empty" icon="codicon-search" />,
    );

    const icon = container.querySelector(".codicon-search");
    expect(icon).toBeInTheDocument();
  });

  it("should apply custom className", () => {
    const { container } = render(
      <PanelEmpty message="Empty" className="custom-empty" />,
    );

    const empty = container.querySelector(".panel-empty");
    expect(empty).toHaveClass("custom-empty");
  });
});

describe("PanelLoading", () => {
  it("should render loading state with default message", () => {
    render(<PanelLoading />);

    expect(screen.getByText("Loading...")).toBeInTheDocument();
  });

  it("should render loading state with custom message", () => {
    render(<PanelLoading message="Please wait..." />);

    expect(screen.getByText("Please wait...")).toBeInTheDocument();
  });

  it("should render loading icon", () => {
    const { container } = render(<PanelLoading />);

    const icon = container.querySelector(".codicon-loading");
    expect(icon).toBeInTheDocument();
  });

  it("should apply custom className", () => {
    const { container } = render(<PanelLoading className="custom-loading" />);

    const loading = container.querySelector(".panel-loading");
    expect(loading).toHaveClass("custom-loading");
  });
});
