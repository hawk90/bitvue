//! Inputs found by `fuzz/fuzz_targets/av1_decode.rs` that once crashed the decoder wrapper.

use bitvue_decode::decoder::Av1Decoder;
use std::path::Path;

fn clip(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_data")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Corrupt frames left dav1d holding unconsumed input (EAGAIN) while the next frame was sent,
/// which the `dav1d` crate answers with a panic ("Have pending data that needs to be handled
/// first"). Whether the decode ends in Ok or Err is not the point; ending is.
#[test]
fn corrupt_frames_that_leave_dav1d_input_pending_do_not_panic() {
    let data = clip("av1_fuzz_dav1d_pending_data.ivf");
    let mut decoder = Av1Decoder::new().unwrap();
    let _ = decoder.decode_all(&data);
}

/// A 5.6 KB file whose sequence header declares 65536x65536. dav1d sizes the picture from the
/// declared size (over 10 GB here, ~25 s to allocate and free) unless the decoder was given a
/// frame size limit. The 1 GiB test allocator cannot see dav1d's C allocations, so the check
/// is the outcome and the time, decoder drop included.
#[test]
fn a_picture_larger_than_av1_level_6_3_is_refused_not_allocated() {
    let data = clip("av1_fuzz_dav1d_oversized_picture.ivf");
    let started = std::time::Instant::now();
    let mut decoder = Av1Decoder::new().unwrap();
    let decoded = decoder.decode_all(&data).map(|frames| frames.len());
    drop(decoder);
    assert!(decoded.is_err(), "decoded {decoded:?} frame(s)");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "took {:?}",
        started.elapsed()
    );
}

/// With dav1d's frame threading on, decoding this 16 KB file hung in `dav1d_get_picture` (every
/// worker asleep on a condition variable) in a few hundred repeats, within seconds, while a fresh
/// process usually got through it once. `Av1Decoder` turns frame threading off. The repeats run
/// on a thread so a hang fails the test instead of stalling the run.
#[test]
fn repeated_decodes_of_a_corrupt_clip_never_hang_in_dav1d() {
    use std::sync::mpsc;
    let data = clip("av1_fuzz_dav1d_frame_thread_hang.ivf");
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        for _ in 0..2000 {
            let mut decoder = Av1Decoder::new().unwrap();
            let _ = decoder.decode_all(&data);
        }
        let _ = done.send(());
    });
    finished
        .recv_timeout(std::time::Duration::from_secs(90))
        .expect("2000 decodes did not finish in 90 s: dav1d hung");
}

bitvue_test_alloc::install!();
