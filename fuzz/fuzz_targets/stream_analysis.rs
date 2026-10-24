#![no_main]

use bitvue_av1_codec::overlay_extraction::StreamAnalyzer;
use libfuzzer_sys::fuzz_target;

/// Frames decoded per input: enough to carry reference state (slots, motion fields, CDFs, film
/// grain, sizes) across several frames, few enough to keep one run fast.
const FRAMES: usize = 12;

// A whole IVF file through the stateful decode path: every frame's header is parsed with the
// reference-slot state of the frames before it, then its tiles are decoded with the saved CDFs and
// projected motion fields. Seeded with the clips in test_data/, so multi-tile, tile-group, superres
// (also per-frame denominators), global-motion and film grain streams are all reachable by small
// mutations. Tiny checkpoint and cache sizes force the rewind-from-checkpoint path too.
fuzz_target!(|data: &[u8]| {
    let Ok(mut analyzer) = StreamAnalyzer::with_limits(data, 2, 2) else {
        return;
    };
    let count = analyzer.frame_count().min(FRAMES);
    for index in 0..count {
        let _ = analyzer.analyze(index);
    }
    // Back to the start (restores a checkpoint), then once more past the end.
    let _ = analyzer.analyze(0);
    let _ = analyzer.analyze(count);
});
