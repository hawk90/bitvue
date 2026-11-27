//! Annex E.4 formulas, each checked against numbers worked out by hand from the spec text.

use super::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// `ScheduledRemovalTiming[ 0 ] = decoder_buffer_delay ÷ 90 000`, then each frame is
/// `buffer_removal_time * DecCT` after the latest random access point.
#[test]
fn removal_times_count_decoding_ticks_from_the_latest_random_access_point() {
    // DecCT 0.1 s, decoder_buffer_delay 9000 -> 0.1 s
    let times = scheduled_removal_timing(
        &[0, 2, 4, 1, 3],
        &[true, false, false, true, false],
        1 << 8,
        0.1,
        9000,
    );
    // 0.1; 0.1+0.2; 0.1+0.4; the second random access point is timed from the first (0.1+0.1 =
    // 0.2 would run backwards, so its counter wraps: 0.1 + 257 ticks); the last from the second
    let expected = [0.1, 0.3, 0.5, 0.1 + 25.7, 0.1 + 25.7 + 0.3];
    for (got, want) in times.iter().zip(expected) {
        assert!(close(*got, want), "{times:?}");
    }
}

/// `buffer_removal_time` is a counter modulo `1 << length`: a value that would put a frame before
/// the one decoded just before it is taken one or more laps later.
#[test]
fn a_wrapped_removal_counter_is_unwrapped_to_keep_time_running_forward() {
    // 3-bit counter: 6, 7, 0 (=8), 1 (=9), 5 (=13), 2 (=18)
    let times = scheduled_removal_timing(
        &[6, 7, 0, 1, 5, 2],
        &[true, false, false, false, false, false],
        8,
        1.0,
        90_000,
    );
    // frame 0 at 1 s; the others are `lifted` ticks after it
    let ticks: Vec<f64> = times.iter().map(|t| t - 1.0).collect();
    assert_eq!(ticks, vec![0.0, 7.0, 8.0, 9.0, 13.0, 18.0]);
}

/// The worked example: 1000 bit/s, delays of 0.1 s each, three DFGs of 50, 100 and 20 bits removed
/// at 0.2, 0.3 and 0.35 s.
#[test]
fn arrival_times_follow_the_latest_arrival_and_the_previous_dfg() {
    let params = SmoothingParams {
        bit_rate: 1000.0,
        encoder_buffer_delay: 9000,
        decoder_buffer_delay: 9000,
        low_delay: false,
        dec_ct: Some(0.05),
    };
    let times = arrival_and_removal(&params, &[50, 100, 20], &[0.2, 0.3, 0.35]);
    // DFG 0: first bit at 0 whatever LatestArrivalTime (0.2 - 0.2 = 0); 50 bits take 0.05 s
    assert!(close(times[0].first_bit_arrival, 0.0));
    assert!(close(times[0].last_bit_arrival, 0.05));
    // DFG 1: LatestArrivalTime 0.3 - 0.2 = 0.1 is later than 0.05 -> waits for it; 0.1 s long
    assert!(close(times[1].first_bit_arrival, 0.1));
    assert!(close(times[1].last_bit_arrival, 0.2));
    // DFG 2: LatestArrivalTime 0.15 is before the end of DFG 1 -> follows it at once
    assert!(close(times[2].first_bit_arrival, 0.2));
    assert!(close(times[2].last_bit_arrival, 0.22));
    for (t, scheduled) in times.iter().zip([0.2, 0.3, 0.35]) {
        assert!(close(t.scheduled_removal, scheduled));
        assert!(
            close(t.removal, scheduled),
            "strict mode removes on schedule"
        );
    }
}

/// Low-delay mode: a DFG that is not complete at its scheduled time is removed at the first
/// decoding tick after its last bit.
#[test]
fn low_delay_mode_defers_an_incomplete_dfg_to_the_next_decoding_tick() {
    let mut params = SmoothingParams {
        bit_rate: 1000.0,
        encoder_buffer_delay: 0,
        decoder_buffer_delay: 0,
        low_delay: true,
        dec_ct: Some(0.05),
    };
    // delay 0: the DFG starts arriving at its removal time, so it is never complete by then
    let times = arrival_and_removal(&params, &[120, 10], &[0.0, 0.1]);
    // DFG 0 ends at 0.12: ceil(0.12 / 0.05) * 0.05 = 0.15
    assert!(close(times[0].last_bit_arrival, 0.12));
    assert!(close(times[0].removal, 0.15));
    // DFG 1: LatestArrivalTime 0.1 < 0.12, arrives 0.12 -> 0.13, due at 0.1 -> 0.15
    assert!(close(times[1].removal, 0.15));
    // a complete DFG is removed on time, and strict mode never defers
    let times = arrival_and_removal(&params, &[10], &[0.5]);
    assert!(close(times[0].removal, 0.5));
    params.low_delay = false;
    let times = arrival_and_removal(&params, &[120, 10], &[0.0, 0.1]);
    assert!(close(times[0].removal, 0.0));
}

/// Bits arrive at `BitRate` while a DFG arrives, and leave all at once at removal.
#[test]
fn occupancy_rises_with_arrival_and_drops_at_removal() {
    let times = [
        DfgTimes {
            first_bit_arrival: 0.0,
            last_bit_arrival: 0.05,
            scheduled_removal: 0.2,
            removal: 0.2,
        },
        DfgTimes {
            first_bit_arrival: 0.1,
            last_bit_arrival: 0.2,
            scheduled_removal: 0.3,
            removal: 0.3,
        },
    ];
    let curve = occupancy_curve(&[50, 100], &times, 1000.0);
    let at = |time: f64, index: usize| {
        curve
            .iter()
            .filter(|p| close(p.time, time))
            .nth(index)
            .map(|p| p.bits)
    };
    assert_eq!(at(0.0, 0), Some(0.0));
    assert_eq!(at(0.05, 0), Some(50.0));
    assert_eq!(at(0.1, 0), Some(50.0));
    // halfway through DFG 1 nothing is a breakpoint; at its end 50 + 100 bits are in
    assert_eq!(at(0.2, 0), Some(150.0), "before DFG 0 is removed");
    assert_eq!(at(0.2, 1), Some(100.0), "after it");
    assert_eq!(at(0.3, 0), Some(100.0));
    assert_eq!(at(0.3, 1), Some(0.0));
    // times never run backwards
    assert!(curve.windows(2).all(|w| w[0].time <= w[1].time));
}

/// A removal before the DFG has fully arrived leaves the buffer short of bits: the curve goes
/// below zero, which is what an underflow is.
#[test]
fn removing_a_dfg_before_it_has_arrived_gives_a_negative_occupancy() {
    let times = [DfgTimes {
        first_bit_arrival: 0.0,
        last_bit_arrival: 0.1,
        scheduled_removal: 0.05,
        removal: 0.05,
    }];
    let curve = occupancy_curve(&[100], &times, 1000.0);
    let after = curve.iter().rfind(|p| close(p.time, 0.05)).unwrap();
    assert!(close(after.bits, -50.0), "{curve:?}");
}
