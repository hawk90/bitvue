import { describe, it, expect } from "vitest";
import { unitNodeToFrameInfo } from "@/contexts/FileStateContext";
import type { BridgeUnitNode } from "@/services/bridge/stream";

const unit = (over: Partial<BridgeUnitNode> = {}): BridgeUnitNode =>
  ({
    key: null,
    unit_type: "Frame",
    offset: 0,
    size: 100,
    frame_index: 0,
    frame_type: "I",
    pts: 0,
    dts: null,
    display_name: "Frame",
    children: [],
    qp_avg: null,
    ...over,
  }) as BridgeUnitNode;

describe("unitNodeToFrameInfo qp_avg (#614)", () => {
  it("carries the indexer's base_q_idx through to FrameInfo.qp_avg", () => {
    expect(unitNodeToFrameInfo(unit({ qp_avg: 85 })).qp_avg).toBe(85);
  });

  it("keeps a QP of 0 (a real base_q_idx) and leaves it absent when there is none", () => {
    expect(unitNodeToFrameInfo(unit({ qp_avg: 0 })).qp_avg).toBe(0);
    expect(unitNodeToFrameInfo(unit({ qp_avg: null })).qp_avg).toBeUndefined();
  });
});
