//! `decode_process()` (spec E.5.2) on hand-made frame lists.

use super::*;

fn frame(frame_type: FrameType, show_frame: bool, refresh: u8) -> FrameRecord {
    FrameRecord {
        packet: 0,
        show_existing_frame: false,
        show_frame,
        frame_type,
        frame_to_show_map_idx: 0,
        refresh_frame_flags: refresh,
        luma_samples: 100,
        buffer_removal_time: None,
        frame_presentation_time: None,
        coded_bits: 0,
    }
}

fn show_existing(slot: u8) -> FrameRecord {
    FrameRecord {
        show_existing_frame: true,
        show_frame: true,
        frame_to_show_map_idx: slot,
        luma_samples: 0,
        ..frame(FrameType::Key, true, 0)
    }
}

/// Decoding takes 100 luma samples / 10 000 per second = 10 ms a frame.
fn config(presentation: Option<&[f64]>) -> DecodeConfig<'_> {
    DecodeConfig {
        max_decode_rate: 10_000.0,
        initial_display_delay: 1,
        presentation,
    }
}

fn kinds(outcome: &DecodeOutcome) -> Vec<ViolationKind> {
    outcome.violations.iter().map(|v| v.0).collect()
}

/// Resource availability mode: the first DFG goes at `decoder_buffer_delay ÷ 90 000`, each next one
/// as soon as the decoder is done with the one before.
#[test]
fn resource_availability_removals_follow_the_decoder_back_to_back() {
    let frames = [
        frame(FrameType::Key, true, 0xFF),
        frame(FrameType::Inter, true, 0x01),
        frame(FrameType::Inter, true, 0x02),
    ];
    let presentation = [0.0, 1.0, 2.0];
    let outcome = decode_process(
        &frames,
        &config(Some(&presentation)),
        &Removal::Resource { first_removal: 0.5 },
    );
    assert_eq!(outcome.removal.len(), 3);
    for (got, want) in outcome.removal.iter().zip([0.5, 0.51, 0.52]) {
        assert!((got - want).abs() < 1e-9, "{:?}", outcome.removal);
    }
    assert!(outcome.violations.is_empty());
}

/// Decoding schedule mode takes the removal times as given and reports a frame removed after its
/// presentation time (`DECODE_BUFFER_AVAILABLE_LATE`) and one whose decode ends after it
/// (`DISPLAY_FRAME_LATE`).
#[test]
fn a_frame_removed_or_decoded_after_its_presentation_time_is_late() {
    let frames = [
        frame(FrameType::Key, true, 0xFF),
        frame(FrameType::Inter, true, 0x01),
        frame(FrameType::Inter, true, 0x02),
    ];
    // the first frame is decoded 0.5..0.51, which is the initial presentation delay: the others
    // are due at 0.51 + 5 ms and 0.51 + 20 ms
    let removal = [0.5, 0.52, 0.7];
    let presentation = [0.0, 0.005, 0.02];
    let outcome = decode_process(
        &frames,
        &config(Some(&presentation)),
        &Removal::Schedule(&removal),
    );
    // frame 1: removed at 0.52, after 0.515; decoded by 0.53. Frame 2: removed at 0.7, after 0.53
    assert_eq!(
        kinds(&outcome),
        vec![
            ViolationKind::DecodeBufferAvailableLate,
            ViolationKind::DisplayFrameLate,
            ViolationKind::DecodeBufferAvailableLate,
            ViolationKind::DisplayFrameLate
        ]
    );
    assert_eq!(
        outcome.violations.iter().map(|v| v.1).collect::<Vec<_>>(),
        vec![1, 1, 2, 2]
    );

    // a frame removed in time but too slow to decode is only a late display: 20 ms of work, due
    // 5 ms after the first
    let slow = [FrameRecord {
        luma_samples: 200,
        ..frame(FrameType::Inter, true, 0x01)
    }];
    let frames = [frame(FrameType::Key, true, 0xFF), slow[0].clone()];
    let outcome = decode_process(
        &frames,
        &config(Some(&[0.0, 0.015])),
        &Removal::Schedule(&[0.5, 0.51]),
    );
    assert_eq!(kinds(&outcome), vec![ViolationKind::DisplayFrameLate]);

    // no presentation times known: nothing to be late for
    let outcome = decode_process(&frames, &config(None), &Removal::Schedule(&removal));
    assert!(outcome.violations.is_empty());
}

#[test]
fn showing_an_empty_slot_is_reported() {
    let frames = [frame(FrameType::Key, true, 0xFF), show_existing(0)];
    let outcome = decode_process(&frames, &config(None), &Removal::Schedule(&[0.5]));
    assert!(outcome.violations.is_empty(), "slot 0 holds the key frame");

    let frames = [frame(FrameType::Key, true, 0x01), show_existing(3)];
    let outcome = decode_process(&frames, &config(None), &Removal::Schedule(&[0.5]));
    assert_eq!(
        kinds(&outcome),
        vec![ViolationKind::DecodeExistingFrameBufEmpty]
    );
    assert_eq!(outcome.violations[0].1, 1);
}

/// Every buffer in the pool held by a slot or a frame waiting for presentation: no frame can be
/// decoded. Eight slots hold eight buffers; shown frames presented far in the future keep two more.
#[test]
fn a_full_pool_leaves_no_buffer_for_the_next_frame() {
    // ten frames, each refreshing its own slot (8 slots: the last two displace the first two, but
    // their buffers stay with the player until presented)
    let mut frames = vec![frame(FrameType::Key, true, 0xFF)];
    for slot in 0..9u8 {
        frames.push(frame(FrameType::Inter, true, 1 << (slot % 8)));
    }
    frames.push(frame(FrameType::Inter, true, 0x01));
    let presentation: Vec<f64> = (0..frames.len()).map(|j| 100.0 + j as f64).collect();
    let removal: Vec<f64> = (0..frames.len()).map(|i| 0.5 + 0.02 * i as f64).collect();
    let outcome = decode_process(
        &frames,
        &config(Some(&presentation)),
        &Removal::Schedule(&removal),
    );
    assert!(
        kinds(&outcome).contains(&ViolationKind::DecodeFrameBufUnavailable),
        "{:?}",
        outcome.violations
    );
}

/// Resource availability: when the pool is full of frames waiting for presentation, the next
/// decode waits for the first of them to be presented.
#[test]
fn resource_availability_waits_for_a_buffer_held_only_for_presentation() {
    // a key frame fills every slot with one buffer; ten more shown frames each refresh nothing, so
    // each is held only until presented
    let mut frames = vec![frame(FrameType::Key, true, 0xFF)];
    for _ in 0..10 {
        frames.push(frame(FrameType::Inter, true, 0));
    }
    // frame j is presented at 1 + j seconds; the pool holds the key frame buffer and nine frames
    let presentation: Vec<f64> = (0..frames.len()).map(|j| j as f64).collect();
    let outcome = decode_process(
        &frames,
        &config(Some(&presentation)),
        &Removal::Resource { first_removal: 0.5 },
    );
    // frame 10 finds buffer 0 (key frame, referenced by the slots) and 1..=9 (presented at
    // 0.51 + 1..9): the earliest presentation frees one at 0.51 + 1
    let last = *outcome.removal.last().unwrap();
    assert!(
        last > 1.4,
        "the 11th frame waits for a presented buffer: {last}"
    );
    assert!(outcome.removal.windows(2).all(|w| w[0] <= w[1]));
}
