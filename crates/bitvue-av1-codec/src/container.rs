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

/// The three `extract_obu_data_from_*` wrappers must return every AV1 sample of the container, concatenated in order.
#[cfg(test)]
mod tests {
    use super::*;

    const S1: [u8; 2] = [0x12, 0x00]; // temporal delimiter OBU
    const S2: [u8; 4] = [0x0A, 0x02, 0xAA, 0xBB];
    const S3: [u8; 1] = [0xEE];

    fn bx(t: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut v = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(t);
        v.extend_from_slice(payload);
        v
    }

    fn full(t: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut p = vec![0, 0, 0, 0];
        p.extend_from_slice(payload);
        bx(t, &p)
    }

    fn be(vals: &[u32]) -> Vec<u8> {
        vals.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// One-track MP4 with one chunk per sample.
    fn mp4(codec: &[u8; 4], samples: &[&[u8]]) -> Vec<u8> {
        let build = |offsets: &[u32]| {
            let mut stsd = be(&[1]);
            stsd.extend(bx(codec, &[0u8; 78]));
            let sizes: Vec<u32> = samples.iter().map(|s| s.len() as u32).collect();
            let mut stbl = full(b"stsd", &stsd);
            stbl.extend(full(
                b"stsz",
                &[be(&[0, sizes.len() as u32]), be(&sizes)].concat(),
            ));
            stbl.extend(full(
                b"stco",
                &[be(&[offsets.len() as u32]), be(offsets)].concat(),
            ));
            bx(
                b"moov",
                &bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &stbl)))),
            )
        };
        let ftyp = bx(b"ftyp", &[b"isom".as_slice(), &be(&[0])].concat());
        let start = (ftyp.len() + build(&vec![0; samples.len()]).len() + 8) as u32;
        let mut at = start;
        let offsets: Vec<u32> = samples
            .iter()
            .map(|s| {
                let o = at;
                at += s.len() as u32;
                o
            })
            .collect();
        let mut f = ftyp;
        f.extend(build(&offsets));
        f.extend(bx(b"mdat", &samples.concat()));
        f
    }

    #[test]
    fn mp4_samples_are_concatenated_in_order() {
        let f = mp4(b"av01", &[&S1, &S2, &S3]);
        assert_eq!(
            extract_obu_data_from_mp4(&f).unwrap(),
            [S1.as_slice(), &S2, &S3].concat()
        );
        // a single sample, and the same bytes through parse_av1's container dispatch
        let one = mp4(b"av01", &[&S1]);
        assert_eq!(extract_obu_data_from_mp4(&one).unwrap(), S1);
        assert!(is_mp4(&f) && !is_mkv(&f) && !is_ts(&f));
    }

    #[test]
    fn mp4_errors_are_propagated() {
        // another codec in the container
        let e = extract_obu_data_from_mp4(&mp4(b"avc1", &[&S1])).unwrap_err();
        assert!(e.to_string().contains("Not an AV1 file"), "{e}");
        assert!(extract_obu_data_from_mp4(&[]).is_err());
        assert!(extract_obu_data_from_mp4(b"not an mp4 file at all").is_err());
        // a truncated file
        let f = mp4(b"av01", &[&S1, &S2]);
        assert!(extract_obu_data_from_mp4(&f[..f.len() - 3]).is_err());
    }

    #[test]
    fn mkv_errors_are_propagated() {
        assert!(extract_obu_data_from_mkv(&[]).is_err());
        assert!(extract_obu_data_from_mkv(b"RIFF....AVI ").is_err());
        assert!(extract_obu_data_from_mkv(&[0x1A, 0x45, 0xDF, 0xA3, 0x80]).is_err());
    }

    fn el(id: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        assert!(payload.len() < 0x7F);
        v.push(0x80 | payload.len() as u8);
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn mkv_samples_are_concatenated_in_order() {
        let mut track = el(&[0xD7], &[1]);
        track.extend(el(&[0x83], &[1]));
        track.extend(el(&[0x86], b"V_AV1"));
        let tracks = el(&[0x16, 0x54, 0xAE, 0x6B], &el(&[0xAE], &track));
        let block = |frame: &[u8]| el(&[0xA3], &[&[0x81, 0, 0, 0x80][..], frame].concat());
        let mut cluster = el(&[0xE7], &[0]);
        for s in [&S1[..], &S2, &S3] {
            cluster.extend(block(s));
        }
        let mut seg = tracks;
        seg.extend(el(&[0x1F, 0x43, 0xB6, 0x75], &cluster));
        let mut f = el(&[0x1A, 0x45, 0xDF, 0xA3], &[]);
        f.extend(el(&[0x18, 0x53, 0x80, 0x67], &seg));
        assert_eq!(
            extract_obu_data_from_mkv(&f).unwrap(),
            [S1.as_slice(), &S2, &S3].concat()
        );
    }

    fn ts_packet(pid: u16, pusi: bool, payload: &[u8]) -> Vec<u8> {
        assert!(payload.len() <= 183);
        let stuffing = 183 - payload.len();
        let mut p = vec![
            0x47,
            ((pusi as u8) << 6) | (pid >> 8) as u8,
            pid as u8,
            0x30,
        ];
        p.push(stuffing as u8);
        if stuffing > 0 {
            p.push(0);
            p.extend(vec![0xFF; stuffing - 1]);
        }
        p.extend_from_slice(payload);
        p
    }

    /// PAT + PMT (PCR_PID present, as in a real stream) + one PES per sample on PID 0x101 with stream type 0x06.
    fn ts(samples: &[&[u8]]) -> Vec<u8> {
        let pat = [
            0x00, 0x00, 0xB0, 0x0D, 0, 1, 0xC1, 0, 0, 0, 1, 0xF0, 0x00, 0, 0, 0, 0,
        ];
        let pmt = [
            0x00, 0x02, 0xB0, 0x12, 0, 1, 0xC1, 0, 0, 0xE1, 0x01, 0xF0, 0x00, 0x06, 0xE1, 0x01,
            0xF0, 0x00, 0, 0, 0, 0,
        ];
        let mut d = ts_packet(0, true, &pat);
        d.extend(ts_packet(0x1000, true, &pmt));
        for s in samples {
            let mut pes = vec![0, 0, 1, 0xE0, 0, 0, 0x80, 0x80, 5, 0x21, 0, 1, 0, 1];
            pes.extend_from_slice(s);
            d.extend(ts_packet(0x101, true, &pes));
        }
        d
    }

    #[test]
    fn ts_samples_are_concatenated_in_order() {
        let f = ts(&[&S1, &S2, &S3]);
        assert_eq!(
            extract_obu_data_from_ts(&f).unwrap(),
            [S1.as_slice(), &S2, &S3].concat()
        );
    }

    #[test]
    fn ts_without_an_av1_stream_gives_no_data_and_a_bad_packet_is_an_error() {
        // PAT/PMT only, no PES: nothing to concatenate
        assert!(extract_obu_data_from_ts(&ts(&[])).unwrap().is_empty());
        assert!(extract_obu_data_from_ts(&[]).unwrap().is_empty());
        let mut bad = ts(&[&S1]);
        bad[0] = 0x00;
        assert!(extract_obu_data_from_ts(&bad).is_err());
        assert!(is_ts(&ts(&[&S1])) && !is_mp4(&ts(&[&S1])));
    }
}
