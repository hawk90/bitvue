//! VVC/H.266 decoding through Fraunhofer's vvdec (system library, 3.x).
//!
//! The pieces, each with one job:
//!
//! * `ffi`     bindings generated from the C header (declarations only)
//! * `handle`  safe RAII wrappers over those bindings; all `unsafe` lives here
//! * `convert` decoded picture to [`DecodedFrame`]; pure Rust, no FFI
//! * `worker`  runs an engine on its own thread and bounds how long we wait for it
//! * this file: [`VvcDecoder`], the [`Decoder`] implementation, wiring the above together
//!
//! # Requirements
//!
//! libvvdec 3.x, found through pkg-config at build time (`build.rs`):
//! macOS `brew install vvdec`; Debian/Ubuntu have no package, so build
//! <https://github.com/fraunhoferhhi/vvdec> (cmake) and point `PKG_CONFIG_PATH` at it.
//!
//! # How decoding works
//!
//! `send_data` takes Annex B data, one access unit or any number of them, and hands it to vvdec,
//! which parses the NAL units itself. vvdec holds pictures back (a short stream outputs nothing
//! until the flush), so [`Decoder::flush`] is what drains the tail; [`Decoder::decode_all`] does
//! `send_data` + `flush` for you. `get_frame` only takes a finished frame off a queue and never
//! calls into the library.

mod convert;
mod ffi;
mod handle;
mod worker;

use std::collections::VecDeque;
use std::time::Duration;

use crate::decoder::{DecodeError, DecodedFrame, Result};
use crate::traits::{CodecType, Decoder, DecoderCapabilities};

use convert::MAX_FRAME_DIMENSION;
use handle::ErrorKind;
use worker::{Engine, EngineError, Worker, WorkerError};

/// How long one decoder call may take before the worker is abandoned and replaced.
///
/// Prevents a hostile stream from hanging the caller forever; any legitimate call is far shorter.
const DECODE_TIMEOUT: Duration = Duration::from_secs(10);

/// Consecutive timeouts after which the decoder gives up for good.
const MAX_CONSECUTIVE_TIMEOUTS: usize = 3;

/// `"3.2.0"`-style version of the vvdec library in use.
pub fn vvdec_version() -> String {
    handle::version()
}

/// The real engine: a vvdec decoder plus the conversion of its pictures.
struct VvdecEngine {
    decoder: handle::Decoder,
}

impl VvdecEngine {
    fn open() -> std::result::Result<Self, EngineError> {
        handle::Decoder::open(None)
            .map(|decoder| Self { decoder })
            .map_err(engine_error)
    }
}

fn engine_error(e: handle::VvdecError) -> EngineError {
    EngineError {
        kind: e.kind,
        message: e.to_string(),
    }
}

fn convert_frame(frame: handle::Frame<'_>) -> std::result::Result<DecodedFrame, EngineError> {
    let picture = frame.picture().map_err(engine_error)?;
    convert::to_decoded_frame(&picture).map_err(|e| EngineError {
        kind: ErrorKind::Fatal,
        message: e.to_string(),
    })
}

impl Engine for VvdecEngine {
    fn decode(
        &mut self,
        data: &[u8],
        cts: Option<i64>,
    ) -> std::result::Result<Option<DecodedFrame>, EngineError> {
        match self.decoder.decode(data, cts).map_err(engine_error)? {
            Some(frame) => convert_frame(frame).map(Some),
            None => Ok(None),
        }
    }

    fn flush(&mut self) -> std::result::Result<Option<DecodedFrame>, EngineError> {
        match self.decoder.flush().map_err(engine_error)? {
            Some(frame) => convert_frame(frame).map(Some),
            None => Ok(None),
        }
    }
}

/// VVC/H.266 decoder using the vvdec library.
///
/// `Send` automatically: it holds a channel to the worker thread and plain data, never a raw
/// pointer (the C handles live and die on the worker thread, see `worker.rs`).
pub struct VvcDecoder {
    worker: Worker,
    /// Frames the library has produced that `get_frame` has not handed out yet.
    queue: VecDeque<DecodedFrame>,
    consecutive_timeouts: usize,
}

impl VvcDecoder {
    /// Creates a decoder; fails if libvvdec cannot be initialised.
    pub fn new() -> Result<Self> {
        Ok(Self {
            worker: spawn_worker()?,
            queue: VecDeque::new(),
            consecutive_timeouts: 0,
        })
    }

    /// Replaces the worker (and so the C decoder) with a fresh one.
    fn respawn(&mut self) -> Result<()> {
        self.worker = spawn_worker()?;
        Ok(())
    }

    /// Maps a worker failure to the decoder's error type and applies the recovery policy:
    /// bad input keeps the decoder; a restart-required state or a lost worker replaces it.
    fn handle_failure(&mut self, error: WorkerError) -> DecodeError {
        match error {
            WorkerError::Engine(e) if e.kind == ErrorKind::Input => {
                self.consecutive_timeouts = 0;
                DecodeError::Decode(e.message)
            }
            WorkerError::Engine(e) => {
                self.consecutive_timeouts = 0;
                if e.kind == ErrorKind::RestartRequired {
                    // The stream state is unrecoverable; drop what is queued from it and restart.
                    self.queue.clear();
                    if let Err(restart) = self.respawn() {
                        return restart;
                    }
                }
                DecodeError::Decode(e.message)
            }
            WorkerError::TimedOut => {
                self.consecutive_timeouts += 1;
                tracing::error!(
                    "vvdec call exceeded {DECODE_TIMEOUT:?} ({}/{MAX_CONSECUTIVE_TIMEOUTS})",
                    self.consecutive_timeouts
                );
                self.queue.clear();
                if self.consecutive_timeouts < MAX_CONSECUTIVE_TIMEOUTS {
                    if let Err(restart) = self.respawn() {
                        return restart;
                    }
                }
                DecodeError::Decode(format!(
                    "vvdec call exceeded {} seconds",
                    DECODE_TIMEOUT.as_secs()
                ))
            }
            WorkerError::Dead(message) => {
                self.queue.clear();
                if let Err(restart) = self.respawn() {
                    return restart;
                }
                DecodeError::Decode(message)
            }
        }
    }

    fn ensure_usable(&self) -> Result<()> {
        if self.consecutive_timeouts >= MAX_CONSECUTIVE_TIMEOUTS && self.worker.is_dead() {
            return Err(DecodeError::Decode(format!(
                "vvdec timed out {MAX_CONSECUTIVE_TIMEOUTS} times in a row; decoder disabled \
                 (call reset() to try again)"
            )));
        }
        Ok(())
    }
}

fn spawn_worker() -> Result<Worker> {
    Worker::spawn(VvdecEngine::open, DECODE_TIMEOUT)
        .map_err(|e| DecodeError::Init(format!("vvdec: {}", e.message)))
}

impl Decoder for VvcDecoder {
    fn codec_type(&self) -> CodecType {
        CodecType::H266
    }

    fn capabilities(&self) -> DecoderCapabilities {
        DecoderCapabilities {
            codec: CodecType::H266,
            max_width: MAX_FRAME_DIMENSION,
            max_height: MAX_FRAME_DIMENSION,
            supported_bit_depths: vec![8, 10, 12],
            hw_accel: false, // vvdec is software-only
        }
    }

    fn send_data(&mut self, data: &[u8], timestamp: Option<i64>) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        self.ensure_usable()?;
        match self.worker.decode(data.to_vec(), timestamp) {
            Ok(frame) => {
                self.consecutive_timeouts = 0;
                self.queue.extend(frame);
                Ok(())
            }
            Err(e) => Err(self.handle_failure(e)),
        }
    }

    fn get_frame(&mut self) -> Result<DecodedFrame> {
        self.queue.pop_front().ok_or(DecodeError::NoFrame)
    }

    /// `send_data` plus `flush`: vvdec holds pictures back until the stream ends, so without the
    /// flush the default implementation would return nothing for short inputs.
    fn decode_all(&mut self, data: &[u8]) -> Result<Vec<DecodedFrame>> {
        self.send_data(data, None)?;
        self.flush();
        self.collect_frames()
    }

    fn flush(&mut self) {
        loop {
            match self.worker.flush() {
                Ok(Some(frame)) => self.queue.push_back(frame),
                Ok(None) => break,
                Err(e) => {
                    // `flush` cannot return an error; log it and stop draining.
                    let error = self.handle_failure(e);
                    tracing::warn!("vvdec flush stopped: {error}");
                    break;
                }
            }
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.queue.clear();
        self.consecutive_timeouts = 0;
        self.respawn()
    }
}
