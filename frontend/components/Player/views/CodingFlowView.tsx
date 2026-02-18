/**
 * Coding Flow View Component
 *
 * Visualizes the encoder/decoder pipeline flow showing:
 * - Input frame -> Prediction -> Transform -> Quantization -> Entropy Coding
 * - Decoder path: Entropy Decoding -> Inverse Quant -> Inverse Transform -> Reconstruction
 * - Current stage highlighting based on analysis mode
 */

import { memo, useMemo } from "react";
import type { FrameInfo } from "../../../types/video";

interface CodingFlowViewProps {
  frame: FrameInfo | null;
  currentStage?:
    | "input"
    | "prediction"
    | "transform"
    | "quantization"
    | "entropy"
    | "reconstruction";
  codec?: string;
}

interface Stage {
  id: string;
  label: string;
  description: string;
  encoderPath: boolean;
  decoderPath: boolean;
}

const ENCODER_STAGES: Stage[] = [
  {
    id: "input",
    label: "Input",
    description: "Original Frame",
    encoderPath: true,
    decoderPath: false,
  },
  {
    id: "prediction",
    label: "Prediction",
    description: "Intra/Inter Prediction",
    encoderPath: true,
    decoderPath: true,
  },
  {
    id: "transform",
    label: "Transform",
    description: "Frequency Transform",
    encoderPath: true,
    decoderPath: true,
  },
  {
    id: "quantization",
    label: "Quantization",
    description: "Quantization of Coefficients",
    encoderPath: true,
    decoderPath: true,
  },
  {
    id: "entropy",
    label: "Entropy Coding",
    description: "CABAC/CAVLC Encoding",
    encoderPath: true,
    decoderPath: true,
  },
  {
    id: "reconstruction",
    label: "Reconstruction",
    description: "Reconstructed Frame",
    encoderPath: true,
    decoderPath: true,
  },
];

const CODEC_FEATURES: Record<string, string[]> = {
  AV1: [
    "Directional Intra Pred",
    "Compound Prediction",
    "Transform Type (DCT/ADST/FLIPADST)",
  ],
  HEVC: [
    "35 Intra Modes",
    "Advanced Motion Vector Pred",
    "Transform Units (4x4 to 32x32)",
  ],
  VVC: ["67 Intra Modes", "GPM/Combine Pred", "MTS: Multiple Transform Sets"],
  AVC: ["9 Intra Modes", "Skip/Direct Modes", "4x4 to 8x8 Transform"],
  VP9: ["10 Intra Modes", "Compound Prediction", "Transform Type Selection"],
  AV3: ["Enhanced Intra", "Super-Resolution", "Adaptive Transform Size"],
};

export const CodingFlowView = memo(function CodingFlowView({
  frame,
  currentStage = "prediction",
  codec = "Unknown",
}: CodingFlowViewProps) {
  const codecFeatures = useMemo(() => {
    return CODEC_FEATURES[codec.toUpperCase()] || ["Standard Features"];
  }, [codec]);

  const getStageClass = (stageId: string) => {
    const isActive = stageId === currentStage;
    const isEncoderPath = ENCODER_STAGES.find(
      (s) => s.id === stageId,
    )?.encoderPath;
    const isDecoderPath = ENCODER_STAGES.find(
      (s) => s.id === stageId,
    )?.decoderPath;

    return [
      "coding-flow-stage",
      isActive ? "coding-flow-stage-active" : "",
      isEncoderPath ? "coding-flow-stage-encoder" : "",
      isDecoderPath ? "coding-flow-stage-decoder" : "",
    ]
      .filter(Boolean)
      .join(" ");
  };

  const getConnectorClass = (index: number) => {
    const currentStageIndex = ENCODER_STAGES.findIndex(
      (s) => s.id === currentStage,
    );
    const isPast = index < currentStageIndex;
    return [
      "coding-flow-connector",
      isPast ? "coding-flow-connector-active" : "",
    ].join(" ");
  };

  return (
    <div className="coding-flow-view">
      <div className="coding-flow-header">
        <h3>Coding Flow - {codec}</h3>
        {frame && (
          <div className="coding-flow-frame-info">
            <span>Frame {frame.frame_index}</span>
            <span className={frame.frame_type.toLowerCase()}>
              {frame.frame_type}
            </span>
          </div>
        )}
      </div>

      {/* Pipeline Visualization */}
      <div className="coding-flow-pipeline">
        <div className="coding-flow-path-label">Encoder Path →</div>
        <div className="coding-flow-stages">
          {ENCODER_STAGES.map((stage, index) => (
            <div key={stage.id} className="coding-flow-stage-wrapper">
              {index > 0 && (
                <div className={getConnectorClass(index)}>
                  <div className="coding-flow-arrow">→</div>
                </div>
              )}
              <div className={getStageClass(stage.id)}>
                <div className="coding-flow-stage-label">{stage.label}</div>
                <div className="coding-flow-stage-desc">
                  {stage.description}
                </div>
                {stage.encoderPath && !stage.decoderPath && (
                  <div className="coding-flow-stage-badge">ENC</div>
                )}
                {stage.decoderPath && !stage.encoderPath && (
                  <div className="coding-flow-stage-badge">DEC</div>
                )}
                {stage.encoderPath && stage.decoderPath && (
                  <div className="coding-flow-stage-badge">BOTH</div>
                )}
              </div>
            </div>
          ))}
        </div>
        <div className="coding-flow-path-label">← Decoder Path</div>
      </div>

      {/* Codec-Specific Features */}
      <div className="coding-flow-features">
        <h4>Codec Features</h4>
        <div className="coding-flow-features-list">
          {codecFeatures.map((feature, idx) => (
            <div key={idx} className="coding-flow-feature-item">
              <span className="coding-flow-feature-bullet">•</span>
              <span>{feature}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Data Flow Legend */}
      <div className="coding-flow-legend">
        <div className="coding-flow-legend-item">
          <div className="coding-flow-legend-box coding-flow-stage-active"></div>
          <span>Current Stage</span>
        </div>
        <div className="coding-flow-legend-item">
          <div className="coding-flow-legend-box coding-flow-connector-active"></div>
          <span>Completed Flow</span>
        </div>
        <div className="coding-flow-legend-item">
          <div className="coding-flow-legend-box coding-flow-stage-encoder"></div>
          <span>Encoder Only</span>
        </div>
        <div className="coding-flow-legend-item">
          <div className="coding-flow-legend-box coding-flow-stage-decoder"></div>
          <span>Decoder Path</span>
        </div>
      </div>
    </div>
  );
});
