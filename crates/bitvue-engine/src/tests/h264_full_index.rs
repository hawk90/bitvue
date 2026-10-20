//! `H264IndexExtractor::extract_full_index` on hand-built Annex B streams: start-code forms,
//! which NAL types become frames, frame sizes, progress reports and cancellation.

use crate::{BitvueError, H264IndexExtractor, IndexExtractor};
use std::cell::RefCell;
use std::io::Cursor;

const START4: [u8; 4] = [0, 0, 0, 1];
const START3: [u8; 3] = [0, 0, 1];
const IDR: u8 = 0x65; // nal_unit_type 5
const SLICE: u8 = 0x41; // nal_unit_type 1
const SPS: u8 = 0x67; // nal_unit_type 7

fn nal(start: &[u8], header: u8, payload_len: usize) -> Vec<u8> {
    let mut v = start.to_vec();
    v.push(header);
    v.extend(std::iter::repeat_n(0xAA, payload_len));
    v
}

fn full(data: Vec<u8>) -> Result<Vec<crate::indexing::FrameMetadata>, BitvueError> {
    H264IndexExtractor::new().extract_full_index(&mut Cursor::new(data), None, None)
}

fn parse_message(err: BitvueError) -> (usize, String) {
    match err {
        BitvueError::Parse { offset, message } => (offset as usize, message),
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[test]
fn idr_is_a_keyframe_labelled_i() {
    let frames = full(nal(&START4, IDR, 3)).unwrap();
    assert_eq!(frames.len(), 1);
    assert!(frames[0].is_keyframe);
    assert_eq!(frames[0].frame_type.as_deref(), Some("I"));
}

#[test]
fn non_idr_slice_is_a_non_keyframe_labelled_p() {
    let frames = full(nal(&START4, SLICE, 3)).unwrap();
    assert_eq!(frames.len(), 1);
    assert!(!frames[0].is_keyframe);
    assert_eq!(frames[0].frame_type.as_deref(), Some("P"));
}

#[test]
fn timestamps_are_never_filled_in() {
    let frames = full(nal(&START4, IDR, 3)).unwrap();
    assert_eq!(frames[0].pts, None);
    assert_eq!(frames[0].dts, None);
}

#[test]
fn display_and_decode_indices_count_frames_in_stream_order() {
    let mut data = nal(&START4, IDR, 2);
    data.extend(nal(&START4, SLICE, 2));
    data.extend(nal(&START4, SLICE, 2));
    let frames = full(data).unwrap();
    let idx: Vec<_> = frames
        .iter()
        .map(|f| (f.display_idx, f.decode_idx))
        .collect();
    assert_eq!(idx, vec![(0, 0), (1, 1), (2, 2)]);
}

#[test]
fn non_frame_nals_are_skipped_and_do_not_take_an_index() {
    let mut data = nal(&START4, SPS, 4);
    data.extend(nal(&START4, 0x68, 2)); // PPS
    data.extend(nal(&START4, IDR, 4));
    data.extend(nal(&START4, 0x06, 2)); // SEI
    data.extend(nal(&START4, SLICE, 4));
    let frames = full(data).unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!((frames[0].display_idx, frames[0].byte_offset), (0, 16));
    assert_eq!((frames[1].display_idx, frames[1].byte_offset), (1, 32));
}

#[test]
fn frame_size_runs_to_the_next_start_code_of_any_nal_type() {
    // IDR (5 + 3 bytes), SPS, slice: the IDR is cut at the SPS start code, not at the slice.
    let mut data = nal(&START4, IDR, 3);
    data.extend(nal(&START4, SPS, 5));
    data.extend(nal(&START4, SLICE, 1));
    let frames = full(data).unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!((frames[0].byte_offset, frames[0].size), (0, 8));
    assert_eq!((frames[1].byte_offset, frames[1].size), (18, 6));
}

#[test]
fn last_frame_runs_to_end_of_file() {
    let frames = full(nal(&START4, IDR, 10)).unwrap();
    assert_eq!(frames[0].size, 15);
}

#[test]
fn three_byte_start_code_begins_at_its_first_zero() {
    let frames = full(nal(&START3, IDR, 4)).unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!((frames[0].byte_offset, frames[0].size), (0, 8));
    assert!(frames[0].is_keyframe);
}

#[test]
fn three_byte_start_code_ends_the_previous_frame() {
    let mut data = nal(&START4, IDR, 3);
    data.extend(nal(&START3, SLICE, 3));
    let frames = full(data).unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].size, 8);
    assert_eq!(frames[1].byte_offset, 8);
    assert!(!frames[1].is_keyframe);
}

#[test]
fn extra_leading_zero_belongs_to_the_four_byte_start_code() {
    let mut data = vec![0u8];
    data.extend(nal(&START4, IDR, 2));
    let frames = full(data).unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].byte_offset, 1);
    assert_eq!(frames[0].size, 7);
}

#[test]
fn garbage_before_the_first_start_code_is_skipped() {
    let mut data = vec![0xFF, 0x12, 0x34];
    data.extend(nal(&START4, IDR, 2));
    let frames = full(data).unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].byte_offset, 3);
}

#[test]
fn only_the_low_five_bits_of_the_header_select_the_nal_type() {
    // 0xE5 = forbidden_zero_bit/nal_ref_idc set, type 5; 0x21 = type 1.
    let mut data = nal(&START4, 0xE5, 2);
    data.extend(nal(&START4, 0x21, 2));
    let frames = full(data).unwrap();
    assert_eq!(frames.len(), 2);
    assert!(frames[0].is_keyframe);
    assert!(!frames[1].is_keyframe);
}

#[test]
fn start_code_without_a_header_byte_yields_no_frames() {
    let (offset, message) = parse_message(full(START4.to_vec()).unwrap_err());
    assert_eq!(offset, 0);
    assert_eq!(message, "No frames found in H.264 stream");
}

#[test]
fn stream_shorter_than_a_start_code_yields_no_frames() {
    assert!(full(vec![0, 0, 1]).is_err());
}

#[test]
fn trailing_bytes_shorter_than_a_start_code_are_ignored() {
    let mut data = nal(&START4, IDR, 4);
    data.extend([0x00, 0x00]);
    let frames = full(data).unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].size, 11);
}

#[test]
fn a_three_byte_start_code_at_the_very_end_is_not_found_by_the_size_search() {
    // The size search needs 4 readable bytes, so a final `00 00 01 41` is found but a final
    // `00 00 01` (no header) is not: the first frame then runs to end of file.
    let mut data = nal(&START4, IDR, 3);
    data.extend(START3);
    let frames = full(data).unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].size, 11);
}

#[test]
fn empty_and_non_start_code_streams_report_no_frames() {
    for data in [vec![], vec![0xFF; 32], vec![0u8; 16]] {
        let (offset, message) = parse_message(full(data).unwrap_err());
        assert_eq!(offset, 0);
        assert_eq!(message, "No frames found in H.264 stream");
    }
}

#[test]
fn a_stream_of_only_parameter_sets_reports_no_frames() {
    let mut data = nal(&START4, SPS, 4);
    data.extend(nal(&START4, 0x68, 2));
    assert!(full(data).is_err());
}

#[test]
fn extractor_rewinds_a_reader_that_is_not_at_the_start() {
    let mut cursor = Cursor::new(nal(&START4, IDR, 4));
    cursor.set_position(3);
    let frames = H264IndexExtractor::new()
        .extract_full_index(&mut cursor, None, None)
        .unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].byte_offset, 0);
}

#[test]
fn cancellation_returns_invalid_data() {
    let err = H264IndexExtractor::new()
        .extract_full_index(&mut Cursor::new(nal(&START4, IDR, 4)), None, Some(&|| true))
        .unwrap_err();
    match err {
        BitvueError::InvalidData(m) => assert_eq!(m, "Indexing cancelled by user"),
        other => panic!("expected InvalidData, got {other:?}"),
    }
}

#[test]
fn cancellation_is_checked_before_the_empty_stream_check() {
    let err = H264IndexExtractor::new()
        .extract_full_index(&mut Cursor::new(vec![]), None, Some(&|| true))
        .unwrap_err();
    assert!(matches!(err, BitvueError::InvalidData(_)));
}

#[test]
fn cancellation_after_some_work_discards_the_frames_found_so_far() {
    let mut data = Vec::new();
    for _ in 0..5 {
        data.extend(nal(&START4, IDR, 6));
    }
    let polls = RefCell::new(0);
    let cancel = || {
        *polls.borrow_mut() += 1;
        *polls.borrow() > 2
    };
    let result =
        H264IndexExtractor::new().extract_full_index(&mut Cursor::new(data), None, Some(&cancel));
    assert!(matches!(result, Err(BitvueError::InvalidData(_))));
    assert_eq!(*polls.borrow(), 3);
}

#[test]
fn cancel_is_polled_once_per_scan_step() {
    // 4 + 1 + 3 bytes = one frame; each non-start-code byte costs a step as well.
    let polls = RefCell::new(0);
    let cancel = || {
        *polls.borrow_mut() += 1;
        false
    };
    H264IndexExtractor::new()
        .extract_full_index(&mut Cursor::new(nal(&START4, IDR, 3)), None, Some(&cancel))
        .unwrap();
    assert_eq!(*polls.borrow(), 2); // frame found, then EOF
}

#[test]
fn progress_is_scan_position_over_file_size_then_a_final_report() {
    let mut data = vec![0xFF]; // one garbage step, then a frame
    data.extend(nal(&START4, IDR, 4));
    let size = data.len() as f64;
    let reports = RefCell::new(Vec::new());
    let progress = |p: f64, m: &str| reports.borrow_mut().push((p, m.to_string()));
    H264IndexExtractor::new()
        .extract_full_index(&mut Cursor::new(data), Some(&progress), None)
        .unwrap();
    let reports = reports.borrow();
    assert_eq!(
        *reports,
        vec![
            (0.0, "Scanning frame 0".to_string()),
            (1.0 / size, "Scanning frame 0".to_string()),
            (1.0, "Indexed 1 frames".to_string()),
        ]
    );
}

#[test]
fn progress_message_carries_the_number_of_frames_found_so_far() {
    let mut data = nal(&START4, IDR, 2);
    data.extend(nal(&START4, SLICE, 2));
    let messages = RefCell::new(Vec::new());
    let progress = |_: f64, m: &str| messages.borrow_mut().push(m.to_string());
    H264IndexExtractor::new()
        .extract_full_index(&mut Cursor::new(data), Some(&progress), None)
        .unwrap();
    assert_eq!(
        *messages.borrow(),
        vec!["Scanning frame 0", "Scanning frame 1", "Indexed 2 frames"]
    );
}

#[test]
fn completion_is_reported_even_when_no_frames_were_found() {
    let reports = RefCell::new(Vec::new());
    let progress = |p: f64, m: &str| reports.borrow_mut().push((p, m.to_string()));
    let result = H264IndexExtractor::new().extract_full_index(
        &mut Cursor::new(vec![]),
        Some(&progress),
        None,
    );
    assert!(result.is_err());
    assert_eq!(
        *reports.borrow(),
        vec![(1.0, "Indexed 0 frames".to_string())]
    );
}

#[test]
fn no_progress_report_is_made_for_a_cancelled_run() {
    let reports = RefCell::new(0);
    let progress = |_: f64, _: &str| *reports.borrow_mut() += 1;
    let _ = H264IndexExtractor::new().extract_full_index(
        &mut Cursor::new(nal(&START4, IDR, 4)),
        Some(&progress),
        Some(&|| true),
    );
    assert_eq!(*reports.borrow(), 0);
}
