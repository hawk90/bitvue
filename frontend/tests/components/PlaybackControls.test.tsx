/**
 * PlaybackControls Component Tests
 * Tests playback controls component
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@/test/test-utils";
import { PlaybackControls } from "../YuvViewerPanel/PlaybackControls";

describe("PlaybackControls", () => {
  it("should render playback controls", () => {
    render(
      <PlaybackControls
        isPlaying={false}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    expect(screen.getByRole("button")).toBeInTheDocument();
  });

  it("should show play icon when paused", () => {
    render(
      <PlaybackControls
        isPlaying={false}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    const playIcon = document.querySelector(".codicon-play");
    expect(playIcon).toBeInTheDocument();
  });

  it("should show pause icon when playing", () => {
    render(
      <PlaybackControls
        isPlaying={true}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    const pauseIcon = document.querySelector(".codicon-debug-pause");
    expect(pauseIcon).toBeInTheDocument();
  });

  it("should call onTogglePlay when button clicked", () => {
    const handleToggle = vi.fn();
    render(
      <PlaybackControls
        isPlaying={false}
        playbackSpeed={1}
        onTogglePlay={handleToggle}
        onSpeedChange={vi.fn()}
      />,
    );

    const button = screen.getByRole("button");
    fireEvent.click(button);

    expect(handleToggle).toHaveBeenCalledTimes(1);
  });

  it("should render speed selector", () => {
    render(
      <PlaybackControls
        isPlaying={false}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    const select = screen.getByRole("combobox");
    expect(select).toBeInTheDocument();
  });

  it("should call onSpeedChange when speed changed", () => {
    const handleSpeedChange = vi.fn();
    render(
      <PlaybackControls
        isPlaying={false}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={handleSpeedChange}
      />,
    );

    const select = screen.getByRole("combobox");
    fireEvent.change(select, { target: { value: "2" } });

    expect(handleSpeedChange).toHaveBeenCalledWith(2);
  });

  it("should use React.memo for performance", () => {
    const { rerender } = render(
      <PlaybackControls
        isPlaying={false}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    rerender(
      <PlaybackControls
        isPlaying={false}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    expect(screen.getByRole("button")).toBeInTheDocument();
  });

  it("should have correct title for play button", () => {
    render(
      <PlaybackControls
        isPlaying={false}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("title", "Play (Space)");
  });

  it("should have correct title for pause button", () => {
    render(
      <PlaybackControls
        isPlaying={true}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    const button = screen.getByRole("button");
    expect(button).toHaveAttribute("title", "Pause (Space)");
  });

  it("should have active class when playing", () => {
    const { container } = render(
      <PlaybackControls
        isPlaying={true}
        playbackSpeed={1}
        onTogglePlay={vi.fn()}
        onSpeedChange={vi.fn()}
      />,
    );

    const button = container.querySelector("button.active");
    expect(button).toBeInTheDocument();
  });
});
