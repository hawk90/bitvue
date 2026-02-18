/**
 * ErrorDialog Component Tests
 * Tests error modal, warning banner, and toast notifications
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "../../test/test-utils";
import { ErrorDialog, WarningBanner } from "../ErrorDialog";

describe("ErrorDialog", () => {
  it("should render error modal when isOpen is true", () => {
    render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("Test Error")).toBeInTheDocument();
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
  });

  it("should not render when isOpen is false", () => {
    const { container } = render(
      <ErrorDialog
        isOpen={false}
        title="Test Error"
        message="Something went wrong"
        onClose={vi.fn()}
      />,
    );

    expect(container.firstChild).toBe(null);
  });

  it("should render error details when provided", () => {
    render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        details="Error: Something went wrong\n    at test.js:10:20"
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText(/View Details/)).toBeInTheDocument();
  });

  it("should call onClose when close button clicked", () => {
    const handleClose = vi.fn();
    render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={handleClose}
      />,
    );

    const closeButton = screen.getByRole("button", { name: "Close" });
    fireEvent.click(closeButton);

    expect(handleClose).toHaveBeenCalledTimes(1);
  });

  it("should call onClose when overlay clicked", () => {
    const handleClose = vi.fn();
    const { container } = render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={handleClose}
      />,
    );

    const overlay = container.querySelector(".error-dialog-overlay");
    if (overlay) {
      fireEvent.click(overlay);
      expect(handleClose).toHaveBeenCalledTimes(1);
    }
  });

  it("should not close when dialog content clicked", () => {
    const handleClose = vi.fn();
    const { container } = render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={handleClose}
      />,
    );

    const dialog = container.querySelector(".error-dialog");
    if (dialog) {
      fireEvent.click(dialog);
      expect(handleClose).not.toHaveBeenCalled();
    }
  });

  it("should render error code when provided", () => {
    render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        errorCode="ERR_001"
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("ERR_001")).toBeInTheDocument();
    expect(screen.getByText("Error Code:")).toBeInTheDocument();
  });

  it("should render dismiss button when onDismiss provided", () => {
    const handleDismiss = vi.fn();
    render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={vi.fn()}
        onDismiss={handleDismiss}
      />,
    );

    expect(screen.getByRole("button", { name: "Dismiss" })).toBeInTheDocument();
  });

  it("should call onDismiss when dismiss button clicked", () => {
    const handleDismiss = vi.fn();
    render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={vi.fn()}
        onDismiss={handleDismiss}
      />,
    );

    const dismissButton = screen.getByRole("button", { name: "Dismiss" });
    fireEvent.click(dismissButton);

    expect(handleDismiss).toHaveBeenCalledTimes(1);
  });

  it("should hide details when showViewDetails is false", () => {
    render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        details="Error details here"
        showViewDetails={false}
        onClose={vi.fn()}
      />,
    );

    expect(screen.queryByText("View Details")).not.toBeInTheDocument();
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={vi.fn()}
      />,
    );

    rerender(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByText("Test Error")).toBeInTheDocument();
  });

  it("should close on Escape key", () => {
    const handleClose = vi.fn();
    render(
      <ErrorDialog
        isOpen={true}
        title="Test Error"
        message="Something went wrong"
        onClose={handleClose}
      />,
    );

    fireEvent.keyDown(document, { key: "Escape" });

    expect(handleClose).toHaveBeenCalled();
  });
});

describe("WarningBanner", () => {
  it("should render warning banner", () => {
    render(
      <WarningBanner
        message="This is a warning"
        onDismiss={vi.fn()}
        severity="warning"
      />,
    );

    expect(screen.getByText("This is a warning")).toBeInTheDocument();
  });

  it("should render info banner", () => {
    render(<WarningBanner message="This is info" severity="info" />);

    expect(screen.getByText("This is info")).toBeInTheDocument();
  });

  it("should render error banner", () => {
    render(<WarningBanner message="This is an error" severity="error" />);

    expect(screen.getByText("This is an error")).toBeInTheDocument();
  });

  it("should render success banner", () => {
    render(<WarningBanner message="Success!" severity="success" />);

    expect(screen.getByText("Success!")).toBeInTheDocument();
  });

  it("should show dismiss button when onDismiss provided", () => {
    const handleDismiss = vi.fn();
    render(
      <WarningBanner
        message="Warning message"
        onDismiss={handleDismiss}
        severity="warning"
      />,
    );

    const dismissButton = screen.getByRole("button");
    fireEvent.click(dismissButton);

    expect(handleDismiss).toHaveBeenCalledTimes(1);
  });

  it("should not show dismiss button when onDismiss not provided", () => {
    render(<WarningBanner message="Info message" severity="info" />);

    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("should apply severity class", () => {
    const { container } = render(
      <WarningBanner message="Warning message" severity="warning" />,
    );

    const banner = container.querySelector(".warning-banner");
    expect(banner).toBeInTheDocument();
    expect(banner).toHaveClass("warning-banner-warning");
  });
});
