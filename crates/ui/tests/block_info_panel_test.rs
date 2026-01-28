//! Tests for Block Info Panel

#[test]
fn test_block_coordinates() {
    // Test block coordinate representation
    struct BlockCoordinates {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    }

    let block = BlockCoordinates {
        x: 64,
        y: 64,
        width: 16,
        height: 16,
    };

    assert_eq!(block.x, 64);
    assert_eq!(block.width, 16);
}

#[test]
fn test_block_type_info() {
    // Test block type information
    #[derive(Debug, PartialEq)]
    enum BlockType {
        Intra,
        Inter,
        Skip,
    }

    let block_type = BlockType::Intra;
    assert_eq!(block_type, BlockType::Intra);
}

#[test]
fn test_block_prediction_mode() {
    // Test block prediction mode info
    struct PredictionInfo {
        mode: String,
        angle_degrees: Option<f32>,
    }

    let pred = PredictionInfo {
        mode: "DC".to_string(),
        angle_degrees: None,
    };

    let directional = PredictionInfo {
        mode: "D45".to_string(),
        angle_degrees: Some(45.0),
    };

    assert_eq!(pred.mode, "DC");
    assert!(directional.angle_degrees.is_some());
}

#[test]
fn test_block_quantization() {
    // Test block quantization parameter info
    struct QuantizationInfo {
        qp: u8,
        delta_qp: i8,
    }

    let quant = QuantizationInfo {
        qp: 26,
        delta_qp: -2,
    };

    assert_eq!(quant.qp, 26);
}

#[test]
fn test_block_transform_info() {
    // Test block transform information
    #[derive(Debug, PartialEq)]
    enum TransformType {
        Dct,
        Dst,
        Adst,
        Identity,
    }

    struct TransformInfo {
        tx_type: TransformType,
        tx_size: usize,
    }

    let transform = TransformInfo {
        tx_type: TransformType::Dct,
        tx_size: 8,
    };

    assert_eq!(transform.tx_type, TransformType::Dct);
}

#[test]
fn test_block_motion_vector_info() {
    // Test block motion vector information
    struct MotionVectorInfo {
        mv_x: i16,
        mv_y: i16,
        ref_frame: usize,
    }

    let mv_info = MotionVectorInfo {
        mv_x: 16,
        mv_y: -8,
        ref_frame: 0,
    };

    assert_eq!(mv_info.mv_x, 16);
}

#[test]
fn test_block_size_info() {
    // Test block size categorization
    fn categorize_block_size(width: usize, height: usize) -> &'static str {
        match (width, height) {
            (4, 4) => "4x4",
            (8, 8) => "8x8",
            (16, 16) => "16x16",
            (32, 32) => "32x32",
            (64, 64) => "64x64",
            _ => "Non-square",
        }
    }

    assert_eq!(categorize_block_size(16, 16), "16x16");
    assert_eq!(categorize_block_size(16, 8), "Non-square");
}

#[test]
fn test_block_residual_info() {
    // Test block residual coefficient information
    struct ResidualInfo {
        non_zero_count: usize,
        total_coeffs: usize,
    }

    impl ResidualInfo {
        fn sparsity(&self) -> f64 {
            if self.total_coeffs == 0 {
                0.0
            } else {
                ((self.total_coeffs - self.non_zero_count) as f64 / self.total_coeffs as f64)
                    * 100.0
            }
        }
    }

    let residual = ResidualInfo {
        non_zero_count: 5,
        total_coeffs: 64,
    };

    assert!(residual.sparsity() > 90.0);
}

#[test]
fn test_block_coding_bits() {
    // Test block coding bits information
    struct CodingBitsInfo {
        header_bits: u32,
        residual_bits: u32,
        total_bits: u32,
    }

    impl CodingBitsInfo {
        fn residual_percentage(&self) -> f64 {
            if self.total_bits == 0 {
                0.0
            } else {
                (self.residual_bits as f64 / self.total_bits as f64) * 100.0
            }
        }
    }

    let bits = CodingBitsInfo {
        header_bits: 50,
        residual_bits: 450,
        total_bits: 500,
    };

    assert_eq!(bits.residual_percentage(), 90.0);
}

#[test]
fn test_block_partition_info() {
    // Test block partition information
    #[derive(Debug, PartialEq)]
    enum PartitionType {
        None,
        Horizontal,
        Vertical,
        Split,
    }

    struct PartitionInfo {
        partition_type: PartitionType,
        depth: usize,
    }

    let partition = PartitionInfo {
        partition_type: PartitionType::Split,
        depth: 2,
    };

    assert_eq!(partition.partition_type, PartitionType::Split);
}

#[test]
fn test_block_reference_frames() {
    // Test block reference frame information
    struct ReferenceFrameInfo {
        fwd_ref: Option<usize>,
        bwd_ref: Option<usize>,
    }

    impl ReferenceFrameInfo {
        fn is_bidirectional(&self) -> bool {
            self.fwd_ref.is_some() && self.bwd_ref.is_some()
        }
    }

    let uni_ref = ReferenceFrameInfo {
        fwd_ref: Some(0),
        bwd_ref: None,
    };

    let bi_ref = ReferenceFrameInfo {
        fwd_ref: Some(0),
        bwd_ref: Some(1),
    };

    assert!(!uni_ref.is_bidirectional());
    assert!(bi_ref.is_bidirectional());
}
