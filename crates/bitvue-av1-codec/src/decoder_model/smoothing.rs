//! The smoothing buffer of the decoder model (spec Annex E.4): when each decodable frame group
//! (DFG) starts and finishes arriving, when it is removed, and the buffer's occupancy in between.
//! Pure functions over numbers, so each formula can be checked against the spec on its own.

/// What the arrival and removal times depend on besides the DFGs themselves.
#[derive(Debug, Clone, Copy)]
pub(super) struct SmoothingParams {
    /// `BitRate`, bits per second (spec E.2)
    pub(super) bit_rate: f64,
    pub(super) encoder_buffer_delay: u32,
    pub(super) decoder_buffer_delay: u32,
    /// `low_delay_mode_flag[ op ]`
    pub(super) low_delay: bool,
    /// `DecCT`, seconds; needed to defer a removal in low-delay mode
    pub(super) dec_ct: Option<f64>,
}

/// The times of one DFG, seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DfgTimes {
    pub(super) first_bit_arrival: f64,
    pub(super) last_bit_arrival: f64,
    pub(super) scheduled_removal: f64,
    pub(super) removal: f64,
}

/// `ScheduledRemovalTiming[ i ]` of every DFG in decoding schedule mode (spec E.4.4):
///
/// ```text
/// ScheduledRemovalTiming[ 0 ] = decoder_buffer_delay ÷ 90 000
/// ScheduledRemovalTiming[ i ] = ScheduledRemovalTiming[ PrevRap ] + buffer_removal_time[ i ] * DecCT
/// ```
///
/// `PrevRap` is the most recent random access point before DFG `i`: a random access point is
/// timed from the one before it, any other frame from the latest one. `buffer_removal_time` is the
/// remainder of a counter modulo `modulus` (spec 5.9.2), so each value is lifted to the smallest
/// tick count that keeps the removal times from running backwards.
pub(super) fn scheduled_removal_timing(
    buffer_removal_time: &[u64],
    is_random_access_point: &[bool],
    modulus: u64,
    dec_ct: f64,
    decoder_buffer_delay: u32,
) -> Vec<f64> {
    let mut times: Vec<f64> = Vec::with_capacity(buffer_removal_time.len());
    let mut last_rap = f64::from(decoder_buffer_delay) / 90_000.0;
    for (i, &removal_time) in buffer_removal_time.iter().enumerate() {
        let scheduled = if i == 0 {
            last_rap
        } else {
            let previous = times[i - 1];
            let min_ticks = ((previous - last_rap) / dec_ct - 1e-9).ceil().max(0.0) as u64;
            let mut ticks = removal_time;
            if ticks < min_ticks {
                ticks += (min_ticks - ticks).div_ceil(modulus.max(1)) * modulus.max(1);
            }
            last_rap + ticks as f64 * dec_ct
        };
        times.push(scheduled);
        if is_random_access_point[i] {
            last_rap = scheduled;
        }
    }
    times
}

/// Arrival and removal times of every DFG (spec E.4.1 to E.4.4):
///
/// ```text
/// LatestArrivalTime[ i ] = ScheduledRemoval[ i ] - ( encoder_buffer_delay + decoder_buffer_delay ) ÷ 90 000
/// FirstBitArrival[ i ]   = max( LastBitArrival[ i - 1 ], LatestArrivalTime[ i ] )
/// LastBitArrival[ i ]    = FirstBitArrival[ i ] + CodedBits[ i ] ÷ BitRate
/// ```
///
/// with `FirstBitArrival[ 0 ] = 0`. The DFG is removed at its scheduled time; in low-delay mode a
/// DFG that is not complete by then is removed at `ceil( LastBitArrival[ i ] ÷ DecCT ) * DecCT`.
pub(super) fn arrival_and_removal(
    params: &SmoothingParams,
    coded_bits: &[u64],
    scheduled_removal: &[f64],
) -> Vec<DfgTimes> {
    let delay = (f64::from(params.encoder_buffer_delay) + f64::from(params.decoder_buffer_delay))
        / 90_000.0;
    let mut times: Vec<DfgTimes> = Vec::with_capacity(coded_bits.len());
    for (i, (&bits, &scheduled)) in coded_bits.iter().zip(scheduled_removal).enumerate() {
        let first_bit_arrival = if i == 0 {
            0.0
        } else {
            times[i - 1].last_bit_arrival.max(scheduled - delay)
        };
        let last_bit_arrival = first_bit_arrival + bits as f64 / params.bit_rate;
        let removal = match params.dec_ct {
            Some(dec_ct) if params.low_delay && scheduled < last_bit_arrival => {
                (last_bit_arrival / dec_ct).ceil() * dec_ct
            }
            _ => scheduled,
        };
        times.push(DfgTimes {
            first_bit_arrival,
            last_bit_arrival,
            scheduled_removal: scheduled,
            removal,
        });
    }
    times
}

/// One point of the occupancy curve: bits in the smoothing buffer at `time` seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OccupancyPoint {
    pub time: f64,
    pub bits: f64,
    /// The DFG whose arrival or removal makes this a breakpoint of the curve
    pub dfg: usize,
}

/// The smoothing buffer's occupancy: bits that have arrived minus bits of the DFGs already
/// removed. Bits arrive at a constant `BitRate` while a DFG arrives, so the curve is piecewise
/// linear with breakpoints at every first and last arrival and every removal; a removal is two
/// points at the same time (the level before and after it).
pub(super) fn occupancy_curve(
    coded_bits: &[u64],
    times: &[DfgTimes],
    bit_rate: f64,
) -> Vec<OccupancyPoint> {
    let mut breakpoints: Vec<(f64, usize)> = Vec::with_capacity(times.len() * 3);
    for (i, t) in times.iter().enumerate() {
        breakpoints.push((t.first_bit_arrival, i));
        breakpoints.push((t.last_bit_arrival, i));
        breakpoints.push((t.removal, i));
    }
    breakpoints.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    breakpoints.dedup_by(|a, b| a.0 == b.0);

    // removals by time, with the bits they take out
    let mut removals: Vec<(f64, u64)> = times
        .iter()
        .zip(coded_bits)
        .map(|(t, &bits)| (t.removal, bits))
        .collect();
    removals.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut curve = Vec::with_capacity(breakpoints.len() + times.len());
    let (mut arrived_dfgs, mut arrived_bits) = (0usize, 0u64);
    let (mut removed_before, mut removed_through) = (0usize, 0usize);
    let (mut bits_removed_before, mut bits_removed_through) = (0u64, 0u64);
    for &(time, dfg) in &breakpoints {
        // DFGs that have finished arriving by `time`
        while arrived_dfgs < times.len() && times[arrived_dfgs].last_bit_arrival <= time {
            arrived_bits += coded_bits[arrived_dfgs];
            arrived_dfgs += 1;
        }
        let partial = match times.get(arrived_dfgs) {
            Some(t) if t.first_bit_arrival < time && t.last_bit_arrival > time => {
                (time - t.first_bit_arrival) * bit_rate
            }
            _ => 0.0,
        };
        let arrived = arrived_bits as f64 + partial;
        while removed_before < removals.len() && removals[removed_before].0 < time {
            bits_removed_before += removals[removed_before].1;
            removed_before += 1;
        }
        while removed_through < removals.len() && removals[removed_through].0 <= time {
            bits_removed_through += removals[removed_through].1;
            removed_through += 1;
        }
        curve.push(OccupancyPoint {
            time,
            bits: arrived - bits_removed_before as f64,
            dfg,
        });
        if bits_removed_through != bits_removed_before {
            curve.push(OccupancyPoint {
                time,
                bits: arrived - bits_removed_through as f64,
                dfg,
            });
        }
    }
    curve
}

#[cfg(test)]
#[path = "smoothing_test.rs"]
mod smoothing_test;
