//! The decode process of the decoder model (spec Annex E.5.2): frame buffers, when a free one is
//! available, how long a frame takes to decode and whether it is decoded and shown in time. In
//! resource availability mode it also produces the removal times.

use super::{FrameRecord, ViolationKind};
use bitvue_engine::FrameType;

/// `BUFFER_POOL_MAX_SIZE` (spec 3)
const BUFFER_POOL_MAX_SIZE: usize = 10;
/// Times are computed in floating point; closer than this is "at the same time".
const EPSILON: f64 = 1e-9;

/// Where a DFG's removal time comes from.
pub(super) enum Removal<'a> {
    /// Decoding schedule mode: `Removal[ i ]` from the smoothing buffer, one per DFG.
    Schedule(&'a [f64]),
    /// Resource availability mode: the earliest time a frame buffer is free
    /// (`time_next_buffer_is_free`); the first DFG is removed at this time (`decoder_buffer_delay
    /// ÷ 90 000`).
    Resource { first_removal: f64 },
}

pub(super) struct DecodeConfig<'a> {
    /// `MaxDecodeRate` of the operating point's level, luma samples per second
    pub(super) max_decode_rate: f64,
    /// `initial_display_delay_minus_1[ op ] + 1`: frames in the pool before presentation starts
    pub(super) initial_display_delay: usize,
    /// `PresentationTime[ j ] - InitialPresentationDelay` of the j-th shown frame, seconds; `None`
    /// when the stream does not give enough to compute presentation times
    pub(super) presentation: Option<&'a [f64]>,
}

pub(super) struct DecodeOutcome {
    /// `Removal[ i ]` of every DFG
    pub(super) removal: Vec<f64>,
    /// Each conformance failure with the index of the frame (in `FrameRecord` order) it is found at
    pub(super) violations: Vec<(ViolationKind, usize)>,
}

struct Pool {
    decoder_ref_count: [u32; BUFFER_POOL_MAX_SIZE],
    player_ref_count: [u32; BUFFER_POOL_MAX_SIZE],
    presentation_times: [f64; BUFFER_POOL_MAX_SIZE],
    /// `VBI`: the buffer each reference slot points to
    vbi: [Option<usize>; 8],
    /// `RefFrameType`
    ref_frame_type: [Option<FrameType>; 8],
}

impl Pool {
    fn new() -> Self {
        Self {
            decoder_ref_count: [0; BUFFER_POOL_MAX_SIZE],
            player_ref_count: [0; BUFFER_POOL_MAX_SIZE],
            presentation_times: [-1.0; BUFFER_POOL_MAX_SIZE],
            vbi: [None; 8],
            ref_frame_type: [None; 8],
        }
    }

    fn free_buffer(&mut self, idx: usize) {
        self.decoder_ref_count[idx] = 0;
        self.player_ref_count[idx] = 0;
        self.presentation_times[idx] = -1.0;
    }

    /// `get_free_buffer`
    fn get_free_buffer(&self) -> Option<usize> {
        (0..BUFFER_POOL_MAX_SIZE)
            .find(|&i| self.decoder_ref_count[i] == 0 && self.player_ref_count[i] == 0)
    }

    /// `update_ref_buffers`, which also records the frame type of every slot it refreshes
    /// (`RefFrameType`, spec 7.20)
    fn update_ref_buffers(&mut self, idx: usize, refresh_frame_flags: u8, frame_type: FrameType) {
        for i in 0..8 {
            if refresh_frame_flags & (1 << i) != 0 {
                if let Some(old) = self.vbi[i] {
                    self.decoder_ref_count[old] -= 1;
                }
                self.vbi[i] = Some(idx);
                self.decoder_ref_count[idx] += 1;
                self.ref_frame_type[i] = Some(frame_type);
            }
        }
    }

    /// `start_decode_at_removal_time`: buffers whose frame has been presented by `removal` and is
    /// no longer referenced go back to the pool.
    fn start_decode_at_removal_time(&mut self, removal: f64) {
        for i in 0..BUFFER_POOL_MAX_SIZE {
            if self.player_ref_count[i] > 0 && self.presentation_times[i] <= removal + EPSILON {
                self.player_ref_count[i] = 0;
                if self.decoder_ref_count[i] == 0 {
                    self.free_buffer(i);
                }
            }
        }
    }

    /// `frames_in_buffer_pool`
    fn frames_in_buffer_pool(&self) -> usize {
        (0..BUFFER_POOL_MAX_SIZE)
            .filter(|&i| self.decoder_ref_count[i] != 0 || self.player_ref_count[i] != 0)
            .count()
    }

    /// `time_next_buffer_is_free` (without its first-DFG case): `time` when a buffer is free, or
    /// else when the first buffer that is only held for presentation is presented. `None` when
    /// every buffer is held as a reference.
    fn time_next_buffer_is_free(&self, time: f64) -> Option<f64> {
        let mut buffer_free_time: Option<f64> = None;
        for k in 0..BUFFER_POOL_MAX_SIZE {
            if self.decoder_ref_count[k] == 0 {
                if self.player_ref_count[k] == 0 {
                    return Some(time);
                }
                if buffer_free_time.is_none_or(|t| self.presentation_times[k] < t) {
                    buffer_free_time = Some(self.presentation_times[k]);
                }
            }
        }
        buffer_free_time
    }
}

/// `decode_process()`: runs the frames in decoding order.
pub(super) fn decode_process(
    frames: &[FrameRecord],
    config: &DecodeConfig,
    removal_source: &Removal,
) -> DecodeOutcome {
    let mut pool = Pool::new();
    let mut outcome = DecodeOutcome {
        removal: Vec::new(),
        violations: Vec::new(),
    };
    let mut time = 0.0f64;
    let mut dfg_num = 0usize;
    let mut shown_num = 0usize;
    let mut initial_presentation_delay: Option<f64> = None;

    for (index, frame) in frames.iter().enumerate() {
        let shown = frame.show_frame || frame.show_existing_frame;
        let this_shown_num = shown_num;
        shown_num += usize::from(shown);
        // `PresentationTime[ ShownFrameNum ]`, known once the initial presentation delay is
        let presentation_at =
            |delay: Option<f64>| Some(delay? + *config.presentation?.get(this_shown_num)?);

        let display_idx;
        if !frame.show_existing_frame {
            let removal = match removal_source {
                Removal::Schedule(removal) => removal[dfg_num],
                Removal::Resource { first_removal } => {
                    if dfg_num == 0 {
                        time = *first_removal;
                    }
                    pool.time_next_buffer_is_free(time)
                        .unwrap_or(time)
                        .max(time)
                }
            };
            if matches!(removal_source, Removal::Resource { .. }) {
                outcome.removal.push(removal);
            }
            dfg_num += 1;
            pool.start_decode_at_removal_time(removal);
            time = removal;
            if frame.show_frame
                && presentation_at(initial_presentation_delay)
                    .is_some_and(|presentation| time > presentation + EPSILON)
            {
                outcome
                    .violations
                    .push((ViolationKind::DecodeBufferAvailableLate, index));
            }
            let cfbi = pool.get_free_buffer();
            if cfbi.is_none() {
                outcome
                    .violations
                    .push((ViolationKind::DecodeFrameBufUnavailable, index));
            }
            time += frame.luma_samples as f64 / config.max_decode_rate;
            if let Some(cfbi) = cfbi {
                pool.update_ref_buffers(cfbi, frame.refresh_frame_flags, frame.frame_type);
                if initial_presentation_delay.is_none()
                    && pool.frames_in_buffer_pool() >= config.initial_display_delay
                {
                    initial_presentation_delay = Some(time);
                }
            }
            display_idx = cfbi;
        } else {
            let slot = usize::from(frame.frame_to_show_map_idx & 7);
            display_idx = pool.vbi[slot];
            match display_idx {
                None => outcome
                    .violations
                    .push((ViolationKind::DecodeExistingFrameBufEmpty, index)),
                Some(idx) => {
                    if pool.ref_frame_type[slot] == Some(FrameType::Key) {
                        pool.update_ref_buffers(idx, 0xFF, FrameType::Key);
                    }
                }
            }
        }

        if shown {
            if let (Some(delay), Some(presentation)) = (
                initial_presentation_delay,
                presentation_at(initial_presentation_delay),
            ) {
                if delay != 0.0 {
                    if time > presentation + EPSILON {
                        outcome
                            .violations
                            .push((ViolationKind::DisplayFrameLate, index));
                    }
                    if let Some(idx) = display_idx {
                        pool.presentation_times[idx] = presentation;
                        pool.player_ref_count[idx] += 1;
                    }
                }
            }
        }
    }
    outcome
}

#[cfg(test)]
#[path = "decode_test.rs"]
mod decode_test;
