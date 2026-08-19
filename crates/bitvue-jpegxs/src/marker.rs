//! JPEG XS marker definitions (ISO 21122-1:2019).
//!
//! All markers are 2-byte values with the pattern 0xFF??. Marker segments
//! are preceded by a 2-byte length field (Lm) that includes the length field
//! itself but excludes the 2-byte marker.
//!
//! Key markers:
//!   0xFF10  SOC  Start of Codestream
//!   0xFF11  EOC  End of Codestream
//!   0xFF50  CAP  Capabilities Marker
//!   0xFF51  PIH  Picture Information Header
//!   0xFF52  CDT  Component Description Table
//!   0xFF53  WGT  Weights Table
//!   0xFF54  SLH  Slice Header
//!   0xFF55  SLI  Slice Instance
//!   0xFF56  SLD  Slice Data
//!   0xFF60  COM  Comment
//!   0xFF63  NLT  Non-Linear Transform
//!   0xFF64  CWD  Coefficient Weights for Decomposition
//!   0xFF72  MCT  Multiple Component Transform

/// JPEG XS marker constants.
pub mod markers {
    pub const SOC: u16 = 0xFF10; // Start of Codestream
    pub const EOC: u16 = 0xFF11; // End of Codestream
    pub const CAP: u16 = 0xFF50; // Capabilities
    pub const PIH: u16 = 0xFF51; // Picture Information Header
    pub const CDT: u16 = 0xFF52; // Component Description Table
    pub const WGT: u16 = 0xFF53; // Weights Table
    pub const SLH: u16 = 0xFF54; // Slice Header
    pub const SLI: u16 = 0xFF55; // Slice Instance
    pub const SLD: u16 = 0xFF56; // Slice Data
    pub const COM: u16 = 0xFF60; // Comment
    pub const NLT: u16 = 0xFF63; // Non-Linear Transform
    pub const CWD: u16 = 0xFF64; // Coefficient Weights for Decomposition
    pub const MCT: u16 = 0xFF72; // Multiple Component Transform
}

/// A raw JPEG XS marker segment extracted from the codestream.
#[derive(Debug, Clone)]
pub struct MarkerSegment<'a> {
    pub marker: u16,
    /// Marker payload bytes (does not include the 2-byte marker or length).
    pub data: &'a [u8],
    /// Byte offset of the 0xFF prefix in the codestream.
    pub offset: usize,
}

impl<'a> MarkerSegment<'a> {
    /// Human-readable name for this marker.
    pub fn name(&self) -> &'static str {
        match self.marker {
            markers::SOC => "SOC",
            markers::EOC => "EOC",
            markers::CAP => "CAP",
            markers::PIH => "PIH",
            markers::CDT => "CDT",
            markers::WGT => "WGT",
            markers::SLH => "SLH",
            markers::SLI => "SLI",
            markers::SLD => "SLD",
            markers::COM => "COM",
            markers::NLT => "NLT",
            markers::CWD => "CWD",
            markers::MCT => "MCT",
            _ => "UNK",
        }
    }
}

/// Scan a JPEG XS codestream and return all marker segments.
///
/// Slice Data (SLD) segments are returned with an empty data slice to avoid
/// copying large payloads; their `offset` and byte length can be computed
/// from the preceding SLH.
pub fn scan_markers(data: &[u8]) -> Vec<MarkerSegment<'_>> {
    let mut segments = Vec::new();
    let mut i = 0usize;

    while i + 1 < data.len() {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = u16::from_be_bytes([data[i], data[i + 1]]);
        match marker {
            // SOC and EOC are standalone (no length field)
            markers::SOC | markers::EOC => {
                segments.push(MarkerSegment {
                    marker,
                    data: &[],
                    offset: i,
                });
                i += 2;
            }
            // All other 0xFF?? markers that we recognise have a length field
            0xFF50..=0xFF7F => {
                if i + 3 >= data.len() {
                    break;
                }
                let lm = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                if lm < 2 {
                    i += 2;
                    continue;
                }
                let payload_len = lm - 2;
                let payload_start = i + 4;
                let payload_end = (payload_start + payload_len).min(data.len());
                // SLD: skip payload copy — just record position
                let payload = if marker == markers::SLD {
                    &data[payload_start..payload_start] // empty slice
                } else {
                    &data[payload_start..payload_end]
                };
                segments.push(MarkerSegment {
                    marker,
                    data: payload,
                    offset: i,
                });
                i = payload_end;
            }
            _ => {
                i += 1;
            }
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_soc_eoc() {
        let data = [0xFF, 0x10, 0xFF, 0x11];
        let segs = scan_markers(&data);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].marker, markers::SOC);
        assert_eq!(segs[1].marker, markers::EOC);
    }

    #[test]
    fn scan_empty() {
        let segs = scan_markers(&[]);
        assert!(segs.is_empty());
    }

    #[test]
    fn scan_pih_segment() {
        // PIH: marker(2) + length(2) = 4 bytes minimum; length=2 → empty payload
        let data = [0xFF, 0x51, 0x00, 0x02];
        let segs = scan_markers(&data);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].marker, markers::PIH);
        assert_eq!(segs[0].data.len(), 0);
    }
}
