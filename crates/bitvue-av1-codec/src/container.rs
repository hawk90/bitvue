//! Recognising the container around an AV1 stream (MP4/MOV, MKV/WebM, TS) and pulling its OBU
//! data out.

/// Check if data is an MP4 file
pub fn is_mp4(data: &[u8]) -> bool {
    // MP4 files start with a box (typically ftyp)
    // First 4 bytes are size, next 4 bytes are type
    data.get(4..8)
        .is_some_and(|box_type| matches!(box_type, b"ftyp" | b"moov" | b"mdat" | b"free" | b"skip"))
}

/// Check if data is an MKV file
pub fn is_mkv(data: &[u8]) -> bool {
    // MKV files start with EBML header
    // First byte is typically 0x1A, and EBML element ID is 0x1A45DFA3
    data.get(0..4)
        .is_some_and(|sig| sig == [0x1A, 0x45, 0xDF, 0xA3])
}

/// Check if data is a WebM file
/// WebM is a subset of MKV with DocType "webm"
pub fn is_webm(data: &[u8]) -> bool {
    // WebM uses the same EBML header as MKV
    // For simplicity, we treat WebM as MKV (they use the same parser)
    is_mkv(data)
}

/// Check if data is a MOV (QuickTime) file
/// MOV uses the same ISO Base Media format as MP4
pub fn is_mov(data: &[u8]) -> bool {
    // MOV files use the same box structure as MP4
    // Check for 'qt  ' or 'moov' box type
    data.get(4..8)
        .is_some_and(|box_type| matches!(box_type, b"ftyp" | b"moov" | b"mdat" | b"wide" | b"free"))
}

/// Check if data is a TS (MPEG-2 Transport Stream) file
pub fn is_ts(data: &[u8]) -> bool {
    bitvue_formats::ts::is_ts(data)
}

/// Extract OBU data from MP4 container
pub fn extract_obu_data_from_mp4(data: &[u8]) -> bitvue_engine::Result<Vec<u8>> {
    use bitvue_formats::mp4;

    // Extract AV1 samples from MP4
    let samples = mp4::extract_av1_samples(data)?;

    // Concatenate all samples into a single OBU stream
    let mut obu_data = Vec::new();
    for sample in samples {
        obu_data.extend_from_slice(&sample);
    }

    Ok(obu_data)
}

/// Extract OBU data from MKV container
pub fn extract_obu_data_from_mkv(data: &[u8]) -> bitvue_engine::Result<Vec<u8>> {
    use bitvue_formats::mkv;

    // Extract AV1 samples from MKV
    let samples = mkv::extract_av1_samples(data)?;

    // Concatenate all samples into a single OBU stream
    let mut obu_data = Vec::new();
    for sample in samples {
        obu_data.extend_from_slice(&sample);
    }

    Ok(obu_data)
}

/// Extract OBU data from TS container
pub fn extract_obu_data_from_ts(data: &[u8]) -> bitvue_engine::Result<Vec<u8>> {
    use bitvue_formats::ts;

    // Extract AV1 samples from TS
    let samples = ts::extract_av1_samples(data)?;

    // Concatenate all samples into a single OBU stream
    let mut obu_data = Vec::new();
    for sample in samples {
        obu_data.extend_from_slice(&sample);
    }

    Ok(obu_data)
}
