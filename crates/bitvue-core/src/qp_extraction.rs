//! Quantization Parameter (QP) extraction utilities
//!
//! This module provides utilities for extracting QP information from
//! codec-specific slice/frame headers, enabling unified QP visualization
//! across all supported video codecs.

use serde::{Deserialize, Serialize};

/// Extracted QP information for a frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QpData {
    /// Average QP value for the frame (0-51)
    pub qp_avg: Option<u8>,
    /// QP value range (min, max)
    pub qp_range: Option<(u8, u8)>,
    /// QP delta from picture parameter set
    pub qp_delta: Option<i32>,
    /// Chroma QP delta (for codecs that support separate chroma QP)
    pub chroma_qp_delta: Option<i32>,
}

impl QpData {
    /// Calculate average QP from a slice_qp_delta and pic_init_qp
    ///
    /// # Arguments
    ///
    /// * `pic_init_qp_minus26` - PPS pic_init_qp_minus26 value
    /// * `slice_qp_delta` - Slice QP delta
    ///
    /// # Returns
    ///
    /// Calculated QP value (0-51)
    pub fn calculate_qp_from_delta(pic_init_qp_minus26: i32, slice_qp_delta: i32) -> u8 {
        let qp = (pic_init_qp_minus26 + 26 + slice_qp_delta).clamp(0, 51) as u8;
        qp
    }

    /// Calculate average QP from multiple slice QP deltas
    ///
    /// This is useful when a frame contains multiple slices with different QP values.
    ///
    /// # Arguments
    ///
    /// * `pic_init_qp_minus26` - PPS pic_init_qp_minus26 value
    /// * `slice_qp_deltas` - Slice QP deltas
    ///
    /// # Returns
    ///
    /// Average QP value (0-51)
    pub fn calculate_average_qp(pic_init_qp_minus26: i32, slice_qp_deltas: &[i32]) -> Option<u8> {
        if slice_qp_deltas.is_empty() {
            return None;
        }

        let qp_values: Vec<u8> = slice_qp_deltas
            .iter()
            .map(|&delta| Self::calculate_qp_from_delta(pic_init_qp_minus26, delta))
            .collect();

        // Calculate average, rounding to nearest integer
        let sum: u32 = qp_values.iter().map(|&v| v as u32).sum();
        let avg = (sum / qp_values.len() as u32).min(51) as u8;
        Some(avg)
    }

    /// Extract QP data from H.264/AVC slice header
    ///
    /// # Arguments
    ///
    /// * `pic_init_qp_minus26` - PPS pic_init_qp_minus26 value
    /// * `slice_qp_delta` - Slice QP delta from slice header
    ///
    /// # Returns
    ///
    /// QP data with calculated average QP
    pub fn from_avc_slice(pic_init_qp_minus26: i32, slice_qp_delta: i32) -> Self {
        let qp_avg = Some(Self::calculate_qp_from_delta(
            pic_init_qp_minus26,
            slice_qp_delta,
        ));
        Self {
            qp_avg,
            qp_range: None, // TODO: Calculate min/max from all slices
            qp_delta: Some(slice_qp_delta),
            chroma_qp_delta: None, // AVC uses separate chroma QP with slice_qs_delta
        }
    }

    /// Extract QP data from H.265/HEVC slice header
    ///
    /// # Arguments
    ///
    /// * `pic_init_qp_minus26` - PPS pic_init_qp_minus26 value
    /// * `slice_qp_delta` - Slice QP delta from slice header
    ///
    /// # Returns
    ///
    /// QP data with calculated average QP
    pub fn from_hevc_slice(pic_init_qp_minus26: i32, slice_qp_delta: i32) -> Self {
        let qp_avg = Some(Self::calculate_qp_from_delta(
            pic_init_qp_minus26,
            slice_qp_delta,
        ));
        Self {
            qp_avg,
            qp_range: None, // TODO: Calculate min/max from all CTUs
            qp_delta: Some(slice_qp_delta),
            chroma_qp_delta: None, // HEVC has separate chroma QP handling
        }
    }

    /// Extract QP data from VP9 quantization index
    ///
    /// VP9 uses a quantization index (0-255) that maps to QP values.
    ///
    /// # Arguments
    ///
    /// * `base_q_idx` - Base quantization index from frame header
    ///
    /// # Returns
    ///
    /// QP data with mapped QP value
    pub fn from_vp9_qindex(base_q_idx: u8) -> Self {
        // VP9 QP mapping (simplified - actual mapping is more complex)
        // QP = qindex / 2 for most cases
        let qp_avg = Some((base_q_idx / 2).min(51));
        Self {
            qp_avg,
            qp_range: None,
            qp_delta: None,
            chroma_qp_delta: None,
        }
    }

    /// Extract QP data from AV1 quantization index
    ///
    /// AV1 uses quantization index that maps to QP values through AC/DC quantization matrices.
    ///
    /// # Arguments
    ///
    /// * `base_q_idx` - Base quantization index
    /// * `delta_q` - Delta Q value
    ///
    /// # Returns
    ///
    /// QP data with calculated QP value
    pub fn from_av1_qindex(base_q_idx: u8, delta_q: i32) -> Self {
        // AV1 QP calculation (simplified)
        let q = (base_q_idx as i32 + delta_q).max(0).min(255) as u8;
        // Map to QP range (rough approximation)
        let qp_avg = Some((q / 4).min(51));
        Self {
            qp_avg,
            qp_range: None,
            qp_delta: Some(delta_q),
            chroma_qp_delta: None,
        }
    }

    /// Check if QP data is available
    pub fn is_available(&self) -> bool {
        self.qp_avg.is_some()
    }

    /// Get the QP average value, or a default if not available
    pub fn qp_avg_or(&self, default: u8) -> u8 {
        self.qp_avg.unwrap_or(default)
    }

    /// Get a display-friendly QP range string
    pub fn range_string(&self) -> String {
        match self.qp_range {
            Some((min, max)) => format!("{}-{}", min, max),
            None => "N/A".to_string(),
        }
    }
}

impl Default for QpData {
    fn default() -> Self {
        Self {
            qp_avg: None,
            qp_range: None,
            qp_delta: None,
            chroma_qp_delta: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_qp_from_delta() {
        // Test QP calculation: pic_init_qp_minus26 + 26 + slice_qp_delta
        assert_eq!(QpData::calculate_qp_from_delta(0, 0), 26);
        assert_eq!(QpData::calculate_qp_from_delta(0, 10), 36);
        assert_eq!(QpData::calculate_qp_from_delta(-10, 5), 21);
        assert_eq!(QpData::calculate_qp_from_delta(0, -30), 0); // Clamped to minimum
        assert_eq!(QpData::calculate_qp_from_delta(0, 30), 51); // Clamped to maximum
    }

    #[test]
    fn test_calculate_average_qp() {
        // Test average QP calculation
        let deltas = vec![0, 10, -5, 5];
        let avg = QpData::calculate_average_qp(0, &deltas);
        assert_eq!(avg, Some(26)); // (26+26+36+21+31)/5 = 140/5 = 28

        // Test empty deltas
        let empty_deltas: Vec<i32> = vec![];
        assert_eq!(QpData::calculate_average_qp(0, &empty_deltas), None);
    }

    #[test]
    fn test_qp_from_avc_slice() {
        let qp_data = QpData::from_avc_slice(0, 5);
        assert_eq!(qp_data.qp_avg, Some(31));
        assert_eq!(qp_data.qp_delta, Some(5));
        assert!(qp_data.is_available());
    }

    #[test]
    fn test_qp_from_vp9_qindex() {
        let qp_data = QpData::from_vp9_qindex(100);
        assert_eq!(qp_data.qp_avg, Some(50));
        assert!(qp_data.is_available());
    }

    #[test]
    fn test_qp_default() {
        let qp_data = QpData::default();
        assert!(!qp_data.is_available());
        assert_eq!(qp_data.qp_avg_or(30), 30);
    }
}
