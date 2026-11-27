/**
 * ApsTab / ProbsTab / QmTab Tests
 *
 * These tabs have no data source (bitvue-sidecar's get_codec_extended_info is AV1-only and carries
 * no vvc_aps/vp9_probs/hevc_qm), so without a `data` prop they must say "not available" and must
 * never call the sidecar or any Tauri runtime.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@/test/test-utils";
import { ApsTab } from "../../components/panels/SyntaxDetailPanel/ApsTab";
import { ProbsTab } from "../../components/panels/SyntaxDetailPanel/ProbsTab";
import { QmTab } from "../../components/panels/SyntaxDetailPanel/QmTab";

describe("codec-specific syntax tabs", () => {
  it("show 'No file loaded' without a file path", () => {
    render(<ApsTab />);
    expect(screen.getByText("No file loaded")).toBeInTheDocument();
  });

  it("ApsTab shows the unavailable state when no data is provided", () => {
    render(<ApsTab filePath="/clip.vvc" />);
    expect(
      screen.getByText("APS data not available for this codec."),
    ).toBeInTheDocument();
  });

  it("ProbsTab shows the unavailable state when no data is provided", () => {
    render(<ProbsTab filePath="/clip.ivf" />);
    expect(
      screen.getByText("Probability data not available for this codec."),
    ).toBeInTheDocument();
  });

  it("QmTab shows the unavailable state when no data is provided", () => {
    render(<QmTab filePath="/clip.hevc" />);
    expect(
      screen.getByText("QM data not available for this codec/frame."),
    ).toBeInTheDocument();
  });

  it("ApsTab renders provided entries grouped by type", () => {
    render(
      <ApsTab
        filePath="/clip.vvc"
        data={{
          aps_list: [
            { aps_id: 3, aps_type: "ALF", enabled: true, summary: "luma ALF" },
          ],
        }}
      />,
    );
    expect(screen.getByText("APS ID 3")).toBeInTheDocument();
    expect(screen.getByText("luma ALF")).toBeInTheDocument();
  });
});
