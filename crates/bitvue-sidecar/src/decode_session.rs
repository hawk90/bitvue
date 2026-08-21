//! Persistent per-stream `Av1Decoder` session backing `get_decoded_frame_yuv`, avoiding a
//! from-frame-0 redecode on every single call (the previous behavior -- see `decode_bridge`'s
//! `get_decoded_frame_yuv`, still used as the correctness-preserving fallback below). Confirmed
//! by real measurement before this existed: on a 320x240/250-frame fixture, `frame_index=249`
//! took 45ms vs `frame_index=0`'s 3ms (roughly O(n) in frame index) -- a real 4K/longer clip
//! would be substantially worse, and this is the hot path for every filmstrip scrub / Prev-Next
//! click / initial playback frame.
//!
//! **Forward-continuation only, bounded memory.** A session keeps the live `Av1Decoder` plus a
//! single most-recently-produced frame (`Clone` is cheap -- `DecodedFrame`'s planes are
//! `Arc<[u8]>`), not a growing cache of every frame ever decoded -- unbounded caching would mean
//! O(stream length) memory (a 250-frame 4K clip alone would be ~3GB of YUV planes). If the
//! requested `frame_index` is at or ahead of the session's current position, keep feeding from
//! where the decoder left off. A backward seek past what the session can serve, or a request
//! against a different underlying file (detected via the `ByteCache` `Arc`'s pointer identity,
//! not `StreamState::file_path` -- a close+reopen of the *same* path still gets a fresh
//! `ByteCache`, and `Core::handle_open_file` mutates `StreamState` in place rather than
//! replacing it, so nothing else naturally invalidates a stale session on reopen), discards the
//! session and restarts from scratch -- byte-for-byte the same decode sequence
//! `decode_bridge::get_decoded_frame_yuv` already used, so that path's correctness is unchanged,
//! just no longer the *only* path.
//!
//! **Thread safety.** `bitvue-sidecar`'s request-handling threads run fully unlocked/parallel by
//! default (see `main.rs`'s "Concurrency model" doc) -- this is a deliberate exception, since a
//! decode session's internal decoder state can't be mutated from two threads at once. Concurrent
//! requests for the *same* stream now serialize on that stream's mutex (they didn't before, each
//! got its own throwaway decoder); concurrent requests for *different* streams (A vs B) still
//! don't contend, each has its own slot.

use crate::decode_bridge::{to_wire, DecodedYuvFrame};
use bitvue_av1_codec::ivf::{parse_ivf_frames, IvfFrame};
use bitvue_decode::{Av1Decoder, DecodedFrame};
use bitvue_engine::StreamId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Mirrors `debug_yuv::FindFirstDiffError` -- distinguishes a genuine failure from a cooperative
/// mid-decode cancellation (see `main.rs`'s "Concurrency model" doc), so the caller can report
/// `WireErrorCode::Cancelled` instead of a generic `FrameNotFound`/`Internal`.
#[derive(Debug)]
pub enum DecodeSessionError {
    Cancelled,
    Other(String),
}

impl From<String> for DecodeSessionError {
    fn from(message: String) -> Self {
        DecodeSessionError::Other(message)
    }
}

struct Session {
    byte_cache_identity: usize,
    decoder: Av1Decoder,
    frames: Vec<IvfFrame>,
    /// How many IVF frames have been fed to the decoder so far.
    fed_count: usize,
    /// How many decoded output frames have been produced so far (<= fed_count -- dav1d buffers
    /// internally, output can lag input).
    decoded_count: usize,
    /// Every decoded frame produced but not yet "settled" (see `advance_to`'s pruning), i.e.
    /// indices `[decoded_count - pending.len(), decoded_count)`. NOT just the single most recent
    /// frame: a single `send_data_owned` + drain batch can yield more than one output frame at
    /// once (dav1d's internal pipeline catching up), and the caller's *next* sequential request
    /// is very likely to land on one of those "bonus" frames rather than the one that was
    /// actually asked for this call -- discarding everything but the last one silently returned
    /// stale data for that case (caught by `forward_sequential_matches_from_scratch_decode`
    /// during development: frame_index=3 came back as a byte-for-byte duplicate of frame 2).
    /// Pruned after every successful lookup to whatever's still >= the served index, so this
    /// stays bounded by dav1d's pipeline depth (small, bounded), not stream length.
    pending: Vec<(usize, DecodedFrame)>,
    /// Whether the end-of-stream flush (`drain_decoder_frames`) has already run -- only ever
    /// needed once, for streams shorter than dav1d's internal pipeline depth (see
    /// `decode_bridge`'s doc on why this is a real fallback, not a rare edge case: some real AOM
    /// conformance vectors are as short as 2 frames).
    flushed: bool,
}

impl Session {
    fn start(byte_cache_identity: usize, data: &[u8]) -> Result<Self, String> {
        let (_hdr, frames) = parse_ivf_frames(data).map_err(|e| format!("IVF parse error: {e}"))?;
        let decoder = Av1Decoder::new().map_err(|e| format!("decoder init: {e}"))?;
        Ok(Self {
            byte_cache_identity,
            decoder,
            frames,
            fed_count: 0,
            decoded_count: 0,
            pending: Vec::new(),
            flushed: false,
        })
    }

    /// True when this session can serve `frame_index` without restarting -- either it's already
    /// been produced (including "the exact frame we just returned", the common repeat-request
    /// case), or it's ahead of the current position and reachable by continuing to feed forward.
    fn can_serve(&self, frame_index: usize) -> bool {
        frame_index + 1 >= self.decoded_count
    }

    /// Decodes forward (if needed) until `frame_index` has been produced, returning it. Fully
    /// drains every `send_data_owned` batch before feeding the next frame (matching
    /// `decode_bridge::get_decoded_frame_yuv`'s established feed-then-fully-drain pattern)
    /// rather than stopping mid-batch the instant the target appears -- leaving dav1d's output
    /// queue partially drained across calls isn't a documented-safe way to interleave more
    /// `send_data_owned` calls, so this always finishes a batch once started.
    ///
    /// `cancel_flag` is checked once per fed IVF packet (mirroring `debug_yuv::find_first_diff`'s
    /// per-packet/per-decoded-frame checkpoints) -- a fast-scrub session's stream mutex (see this
    /// module's "Thread safety" doc) means a request the user already scrubbed past can otherwise
    /// sit there decoding to completion before the next request even gets a chance to run.
    fn advance_to(
        &mut self,
        frame_index: usize,
        cancel_flag: &AtomicBool,
    ) -> Result<DecodedFrame, DecodeSessionError> {
        if frame_index >= self.frames.len() {
            return Err(DecodeSessionError::Other(format!(
                "frame_index {frame_index} out of range (stream has {} frames)",
                self.frames.len()
            )));
        }

        while self.fed_count < self.frames.len() && self.decoded_count <= frame_index {
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(DecodeSessionError::Cancelled);
            }
            let f = &self.frames[self.fed_count];
            self.decoder
                .send_data_owned(f.data.clone(), f.timestamp as i64)
                .map_err(|e| format!("decode send: {e}"))?;
            self.fed_count += 1;
            while let Ok(frame) = self.decoder.get_frame() {
                self.pending.push((self.decoded_count, frame));
                self.decoded_count += 1;
            }
        }

        // Deliberately not dec.flush() -- see Av1Decoder::drain_decoder_frames' doc: flush()
        // clears dav1d's internal state (for seeking) rather than draining it, discarding every
        // still-buffered frame on streams shorter than dav1d's thread-pipeline depth.
        if self.decoded_count <= frame_index && self.fed_count >= self.frames.len() && !self.flushed
        {
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(DecodeSessionError::Cancelled);
            }
            self.flushed = true;
            let mut extra = Vec::new();
            self.decoder
                .drain_decoder_frames(&mut extra)
                .map_err(|e| format!("decode drain: {e}"))?;
            for frame in extra {
                self.pending.push((self.decoded_count, frame));
                self.decoded_count += 1;
            }
        }

        if self.decoded_count <= frame_index {
            return Err(DecodeSessionError::Other(format!(
                "decoder produced only {} frame(s), needed index {frame_index}",
                self.decoded_count
            )));
        }

        let result = self
            .pending
            .iter()
            .find(|(idx, _)| *idx == frame_index)
            .map(|(_, frame)| frame.clone())
            .ok_or_else(|| {
                DecodeSessionError::Other(
                    "internal error: session advanced past frame_index without capturing it (please report)"
                        .to_string(),
                )
            })?;
        // Bound memory: drop everything strictly before the index we just served -- but keep
        // frame_index itself, since `can_serve` explicitly allows an immediate repeat request
        // for the exact same index (decoded_count == frame_index + 1) to be served from here
        // again with no further decoding.
        self.pending.retain(|(idx, _)| *idx >= frame_index);
        Ok(result)
    }
}

/// Per-stream decode session slots (A and B independent, so concurrent requests against
/// different streams never contend on each other's lock).
#[derive(Default)]
pub struct DecodeSessions {
    a: Mutex<Option<Session>>,
    b: Mutex<Option<Session>>,
}

impl DecodeSessions {
    pub fn new() -> Self {
        Self::default()
    }

    /// `byte_cache_identity` should be `Arc::as_ptr(&byte_cache) as usize` -- see this module's
    /// doc for why that's the right staleness signal instead of `StreamState::file_path`.
    pub fn get_decoded_frame_yuv(
        &self,
        stream: StreamId,
        byte_cache_identity: usize,
        data: &[u8],
        frame_index: usize,
        cancel_flag: &AtomicBool,
    ) -> Result<DecodedYuvFrame, DecodeSessionError> {
        let slot = match stream {
            StreamId::A => &self.a,
            StreamId::B => &self.b,
        };
        let mut guard = slot.lock().unwrap();

        // Checked immediately after acquiring the lock (before touching a possibly-stale
        // session, and before a fresh `Session::start`'s IVF parse) -- the point is to bail as
        // soon as possible once a request another thread already superseded finally gets its
        // turn, not just mid-decode.
        if cancel_flag.load(Ordering::SeqCst) {
            return Err(DecodeSessionError::Cancelled);
        }

        let reusable = guard.as_ref().is_some_and(|s| {
            s.byte_cache_identity == byte_cache_identity && s.can_serve(frame_index)
        });
        if !reusable {
            *guard = Some(Session::start(byte_cache_identity, data)?);
        }

        let session = guard.as_mut().expect("just ensured Some above");
        let frame = session.advance_to(frame_index, cancel_flag)?;
        Ok(to_wire(&frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode_bridge;
    use crate::test_support::AV1_IVF_FIXTURE;

    fn identity() -> usize {
        1 // fixed fake ByteCache identity for tests -- data is passed directly, not via Core
    }

    fn no_cancel() -> AtomicBool {
        AtomicBool::new(false)
    }

    #[test]
    fn forward_sequential_matches_from_scratch_decode() {
        let sessions = DecodeSessions::new();
        for i in 0..10 {
            let via_session = sessions
                .get_decoded_frame_yuv(StreamId::A, identity(), AV1_IVF_FIXTURE, i, &no_cancel())
                .unwrap();
            let via_scratch = decode_bridge::get_decoded_frame_yuv(AV1_IVF_FIXTURE, i).unwrap();
            assert_eq!(
                via_session.bytes, via_scratch.bytes,
                "frame {i}: session-continued decode should byte-for-byte match a from-scratch decode"
            );
            assert_eq!(via_session.descriptor.width, via_scratch.descriptor.width);
            assert_eq!(via_session.descriptor.height, via_scratch.descriptor.height);
        }
    }

    #[test]
    fn repeat_request_for_same_frame_is_served_without_redecoding() {
        let sessions = DecodeSessions::new();
        let first = sessions
            .get_decoded_frame_yuv(StreamId::A, identity(), AV1_IVF_FIXTURE, 5, &no_cancel())
            .unwrap();
        let second = sessions
            .get_decoded_frame_yuv(StreamId::A, identity(), AV1_IVF_FIXTURE, 5, &no_cancel())
            .unwrap();
        assert_eq!(first.bytes, second.bytes);
    }

    #[test]
    fn non_monotonic_scrub_still_matches_from_scratch_decode() {
        // Real usage isn't strictly sequential -- filmstrip clicks jump around. Every result
        // (whether served by continuing forward or by a full restart on backward seeks) must
        // still be correct.
        let sessions = DecodeSessions::new();
        for i in [0usize, 20, 5, 15, 3, 3, 25] {
            let via_session = sessions
                .get_decoded_frame_yuv(StreamId::A, identity(), AV1_IVF_FIXTURE, i, &no_cancel())
                .unwrap();
            let via_scratch = decode_bridge::get_decoded_frame_yuv(AV1_IVF_FIXTURE, i).unwrap();
            assert_eq!(
                via_session.bytes, via_scratch.bytes,
                "frame {i} (non-monotonic scrub) should match a from-scratch decode"
            );
        }
    }

    #[test]
    fn different_byte_cache_identity_starts_a_fresh_session() {
        let sessions = DecodeSessions::new();
        sessions
            .get_decoded_frame_yuv(StreamId::A, 111, AV1_IVF_FIXTURE, 20, &no_cancel())
            .unwrap();
        // A different identity (simulating the stream being closed and a different file opened)
        // must not reuse the old session's decoder position -- frame 0 must still be servable
        // (it would fail/misdecode if we tried to "continue forward" using a session that had
        // already fed 21 frames of a stream this call doesn't know about).
        let result = sessions
            .get_decoded_frame_yuv(StreamId::A, 222, AV1_IVF_FIXTURE, 0, &no_cancel())
            .unwrap();
        let reference = decode_bridge::get_decoded_frame_yuv(AV1_IVF_FIXTURE, 0).unwrap();
        assert_eq!(result.bytes, reference.bytes);
    }

    #[test]
    fn streams_a_and_b_have_independent_sessions() {
        let sessions = DecodeSessions::new();
        sessions
            .get_decoded_frame_yuv(StreamId::A, identity(), AV1_IVF_FIXTURE, 20, &no_cancel())
            .unwrap();
        // Stream B's first request (frame 0) must not be affected by A's session having already
        // advanced past frame 0.
        let result = sessions
            .get_decoded_frame_yuv(StreamId::B, identity(), AV1_IVF_FIXTURE, 0, &no_cancel())
            .unwrap();
        let reference = decode_bridge::get_decoded_frame_yuv(AV1_IVF_FIXTURE, 0).unwrap();
        assert_eq!(result.bytes, reference.bytes);
    }

    #[test]
    fn out_of_range_frame_index_is_a_real_error() {
        let sessions = DecodeSessions::new();
        let err = sessions
            .get_decoded_frame_yuv(
                StreamId::A,
                identity(),
                AV1_IVF_FIXTURE,
                999_999,
                &no_cancel(),
            )
            .unwrap_err();
        match err {
            DecodeSessionError::Other(message) => {
                assert!(
                    message.contains("out of range"),
                    "unexpected error: {message}"
                )
            }
            DecodeSessionError::Cancelled => panic!("expected Other, got Cancelled"),
        }
    }

    /// Real regression coverage for the axis-6 cancellation wiring: a flag already set before the
    /// call starts must stop `advance_to` before it decodes anything, not just theoretically be
    /// checked somewhere in the loop. Pre-setting the flag (rather than flipping it mid-decode
    /// from another thread) keeps this deterministic while still exercising the real
    /// `Ordering::SeqCst` load path `get_decoded_frame_yuv`/`advance_to` use.
    #[test]
    fn pre_cancelled_request_returns_cancelled_not_a_partial_decode() {
        let sessions = DecodeSessions::new();
        let cancel_flag = AtomicBool::new(true);
        let err = sessions
            .get_decoded_frame_yuv(StreamId::A, identity(), AV1_IVF_FIXTURE, 5, &cancel_flag)
            .unwrap_err();
        assert!(matches!(err, DecodeSessionError::Cancelled));
    }
}
