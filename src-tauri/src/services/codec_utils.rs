//! Codec utility functions
//!
//! Shared utility functions for codec operations like IVF wrapper creation.

/// Create minimal IVF wrapper for a single AV1 sample
///
/// This function wraps an AV1 sample (OBU data) in a minimal IVF container
/// so it can be decoded by the AV1 decoder which expects IVF format.
///
/// # Arguments
/// * `sample_data` - The AV1 sample data (OBU bytes) to wrap
///
/// # Returns
/// A complete IVF file as bytes (header + frame header + sample data)
///
/// # Note
/// The resolution values in the IVF header are placeholders (1920x1080).
/// The decoder will detect the actual resolution from the AV1 sequence header.
#[allow(dead_code)]
pub fn create_ivf_wrapper(sample_data: &[u8]) -> Vec<u8> {
    let mut ivf = Vec::with_capacity(32 + 12 + sample_data.len());

    // IVF header (32 bytes)
    ivf.extend_from_slice(b"DKIF");              // Signature (4 bytes)
    ivf.extend_from_slice(&0u16.to_le_bytes());  // Version (2 bytes)
    ivf.extend_from_slice(&1u16.to_le_bytes());  // Header length (2 bytes)
    ivf.extend_from_slice(b"AV01");              // FourCC (4 bytes)
    ivf.extend_from_slice(&1920u16.to_le_bytes()); // Width (placeholder, 2 bytes)
    ivf.extend_from_slice(&1080u16.to_le_bytes()); // Height (placeholder, 2 bytes)
    ivf.extend_from_slice(&30u32.to_le_bytes()); // Timebase denominator (4 bytes)
    ivf.extend_from_slice(&1u32.to_le_bytes());  // Timebase numerator (4 bytes)
    ivf.extend_from_slice(&1u32.to_le_bytes());  // Frame count (4 bytes)
    ivf.extend_from_slice(&[0u8; 4]);            // Reserved (4 bytes)

    // IVF frame header (12 bytes)
    ivf.extend_from_slice(&(sample_data.len() as u32).to_le_bytes()); // Frame size (4 bytes)
    ivf.extend_from_slice(&0u64.to_le_bytes()); // Timestamp (8 bytes)
    ivf.extend_from_slice(sample_data);          // Frame data

    ivf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ivf_wrapper_header() {
        let sample = b"test";
        let ivf = create_ivf_wrapper(sample);

        // Check IVF signature
        assert_eq!(&ivf[0..4], b"DKIF");

        // Check version
        assert_eq!(u16::from_le_bytes([ivf[4], ivf[5]]), 0);

        // Check header length
        assert_eq!(u16::from_le_bytes([ivf[6], ivf[7]]), 1);

        // Check FourCC
        assert_eq!(&ivf[8..12], b"AV01");

        // Check placeholder dimensions
        assert_eq!(u16::from_le_bytes([ivf[12], ivf[13]]), 1920);
        assert_eq!(u16::from_le_bytes([ivf[14], ivf[15]]), 1080);
    }

    #[test]
    fn test_create_ivf_wrapper_frame_header() {
        let sample = b"test_data";
        let ivf = create_ivf_wrapper(sample);

        // Frame header starts at byte 32
        let frame_size = u32::from_le_bytes([ivf[32], ivf[33], ivf[34], ivf[35]]);
        assert_eq!(frame_size, 9); // "test_data".len()

        let timestamp = u64::from_le_bytes([
            ivf[36], ivf[37], ivf[38], ivf[39],
            ivf[40], ivf[41], ivf[42], ivf[43],
        ]);
        assert_eq!(timestamp, 0);

        // Frame data starts at byte 44
        assert_eq!(&ivf[44..], sample);
    }

    #[test]
    fn test_create_ivf_wrapper_empty_sample() {
        let sample = b"";
        let ivf = create_ivf_wrapper(sample);

        // Should still have valid headers
        assert_eq!(&ivf[0..4], b"DKIF");
        assert_eq!(&ivf[8..12], b"AV01");

        // Frame size should be 0
        let frame_size = u32::from_le_bytes([ivf[32], ivf[33], ivf[34], ivf[35]]);
        assert_eq!(frame_size, 0);
    }

    #[test]
    fn test_create_ivf_wrapper_large_sample() {
        let large_sample = vec![0u8; 10000];
        let ivf = create_ivf_wrapper(&large_sample);

        // Check total size
        assert_eq!(ivf.len(), 32 + 12 + 10000);

        // Check frame size
        let frame_size = u32::from_le_bytes([ivf[32], ivf[33], ivf[34], ivf[35]]);
        assert_eq!(frame_size, 10000);
    }
}
