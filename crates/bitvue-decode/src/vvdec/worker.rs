//! Runs a decoder on its own thread and bounds how long the caller waits for it.
//!
//! Why a thread at all: a decoder call can in principle hang on hostile input, and Rust cannot
//! interrupt a call into C. The old implementation spawned a thread per call that borrowed the
//! decoder's raw pointers, and on timeout returned while that thread was still using them; the
//! next `reset()` then freed them underneath it (use-after-free).
//!
//! Here the engine (whatever owns the C handles) is **created on the worker thread and never
//! leaves it**. Only plain values cross the thread boundary: the input bytes going in and the
//! decoded frame (or an error) coming out. So there is no `unsafe impl Send`, and a timeout
//! cannot free anything: the caller just stops waiting and drops its channel ends. The worker
//! finishes the call it is in, sees the closed channel, drops the engine itself and exits. If
//! the call never returns the thread leaks, but nothing is ever freed while in use.
//!
//! The module knows nothing about vvdec (`Engine` is a trait), so its timeout and panic handling
//! is tested with fake engines, without libvvdec.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::decoder::DecodedFrame;

use super::handle::ErrorKind;

/// A failure reported by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EngineError {
    pub kind: ErrorKind,
    pub message: String,
}

/// What the worker thread owns and drives. Not required to be `Send`: it is built on the worker.
pub(super) trait Engine {
    /// Feed data; `Ok(None)` = consumed, no picture ready yet.
    fn decode(
        &mut self,
        data: &[u8],
        cts: Option<i64>,
    ) -> Result<Option<DecodedFrame>, EngineError>;
    /// Drain one held-back picture; `Ok(None)` = nothing left.
    fn flush(&mut self) -> Result<Option<DecodedFrame>, EngineError>;
}

/// Why a worker call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WorkerError {
    /// The engine reported an error; the worker is still alive.
    Engine(EngineError),
    /// The call took longer than the timeout. The worker is abandoned (see module docs).
    TimedOut,
    /// The worker thread is gone (it panicked, or an earlier call timed out).
    Dead(String),
}

enum Command {
    Decode(Vec<u8>, Option<i64>),
    Flush,
}

type Reply = Result<Option<DecodedFrame>, EngineError>;

/// Handle to a worker thread running one [`Engine`].
pub(super) struct Worker {
    commands: Sender<(Command, Sender<Reply>)>,
    /// `None` once the worker was abandoned: we only ever *join* a thread that has finished.
    thread: Option<JoinHandle<()>>,
    timeout: Duration,
    dead: bool,
}

impl Worker {
    /// Starts a worker. `make_engine` runs on the worker thread; if it fails, so does `spawn`.
    pub(super) fn spawn<E, F>(make_engine: F, timeout: Duration) -> Result<Self, EngineError>
    where
        E: Engine,
        F: FnOnce() -> Result<E, EngineError> + Send + 'static,
    {
        let (commands, inbox) = mpsc::channel::<(Command, Sender<Reply>)>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), EngineError>>();

        let thread = thread::Builder::new()
            .name("vvdec-worker".into())
            .spawn(move || {
                let mut engine = match make_engine() {
                    Ok(engine) => {
                        let _ = ready_tx.send(Ok(()));
                        engine
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                // Ends when every `Sender` is dropped (the owner went away or gave up waiting).
                while let Ok((command, reply)) = inbox.recv() {
                    let result = match command {
                        Command::Decode(data, cts) => engine.decode(&data, cts),
                        Command::Flush => engine.flush(),
                    };
                    // The caller may have timed out and left: a failed send is expected then.
                    let _ = reply.send(result);
                }
                // `engine` drops here, on the thread that used it.
            })
            .map_err(|e| EngineError {
                kind: ErrorKind::Fatal,
                message: format!("could not start the decoder thread: {e}"),
            })?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                commands,
                thread: Some(thread),
                timeout,
                dead: false,
            }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                let _ = thread.join();
                Err(EngineError {
                    kind: ErrorKind::Fatal,
                    message: "the decoder thread died while starting".into(),
                })
            }
        }
    }

    pub(super) fn decode(
        &mut self,
        data: Vec<u8>,
        cts: Option<i64>,
    ) -> Result<Option<DecodedFrame>, WorkerError> {
        self.call(Command::Decode(data, cts))
    }

    pub(super) fn flush(&mut self) -> Result<Option<DecodedFrame>, WorkerError> {
        self.call(Command::Flush)
    }

    /// True once the worker timed out or died; a new one must be spawned.
    pub(super) fn is_dead(&self) -> bool {
        self.dead
    }

    fn call(&mut self, command: Command) -> Result<Option<DecodedFrame>, WorkerError> {
        if self.dead {
            return Err(WorkerError::Dead(
                "the decoder thread is no longer running".into(),
            ));
        }
        let (reply_tx, reply_rx) = mpsc::channel();
        if self.commands.send((command, reply_tx)).is_err() {
            self.dead = true;
            return Err(WorkerError::Dead("the decoder thread exited".into()));
        }
        match reply_rx.recv_timeout(self.timeout) {
            Ok(result) => result.map_err(WorkerError::Engine),
            Err(RecvTimeoutError::Timeout) => {
                // Abandon: do not join, do not free. See the module docs.
                self.dead = true;
                self.thread = None;
                Err(WorkerError::TimedOut)
            }
            Err(RecvTimeoutError::Disconnected) => {
                // The reply sender was dropped without a reply: the engine panicked.
                self.dead = true;
                Err(WorkerError::Dead("the decoder thread panicked".into()))
            }
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // Closing the command channel ends the worker loop. Join only if it was not abandoned
        // (an abandoned worker may still be inside a call that never returns).
        let (dead_tx, _) = mpsc::channel();
        drop(std::mem::replace(&mut self.commands, dead_tx));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::{ChromaFormat, FrameType};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    fn frame(timestamp: i64) -> DecodedFrame {
        DecodedFrame {
            width: 2,
            height: 2,
            bit_depth: 8,
            y_plane: vec![0u8; 4].into(),
            y_stride: 2,
            u_plane: None,
            u_stride: 0,
            v_plane: None,
            v_stride: 0,
            timestamp,
            frame_type: FrameType::Inter,
            qp_avg: None,
            chroma_format: ChromaFormat::Monochrome,
        }
    }

    const SHORT: Duration = Duration::from_millis(150);
    const LONG: Duration = Duration::from_secs(10);

    /// Records when it is dropped, and which thread dropped it.
    struct Probe {
        dropped: Arc<AtomicBool>,
        dropped_on: Arc<std::sync::Mutex<Option<thread::ThreadId>>>,
        sleep_on_decode: Duration,
        panic_on_decode: bool,
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            *self.dropped_on.lock().unwrap() = Some(thread::current().id());
            self.dropped.store(true, Ordering::SeqCst);
        }
    }
    impl Engine for Probe {
        fn decode(
            &mut self,
            data: &[u8],
            cts: Option<i64>,
        ) -> Result<Option<DecodedFrame>, EngineError> {
            if self.panic_on_decode {
                panic!("engine panic (expected in this test)");
            }
            thread::sleep(self.sleep_on_decode);
            if data.is_empty() {
                return Ok(None);
            }
            Ok(Some(frame(cts.unwrap_or(-1))))
        }
        fn flush(&mut self) -> Result<Option<DecodedFrame>, EngineError> {
            Ok(None)
        }
    }

    fn probe(
        sleep: Duration,
        panics: bool,
    ) -> (
        Probe,
        Arc<AtomicBool>,
        Arc<std::sync::Mutex<Option<thread::ThreadId>>>,
    ) {
        let dropped = Arc::new(AtomicBool::new(false));
        let on = Arc::new(std::sync::Mutex::new(None));
        (
            Probe {
                dropped: dropped.clone(),
                dropped_on: on.clone(),
                sleep_on_decode: sleep,
                panic_on_decode: panics,
            },
            dropped,
            on,
        )
    }

    #[test]
    fn frames_and_empty_results_pass_through() {
        let (p, ..) = probe(Duration::ZERO, false);
        let mut w = Worker::spawn(move || Ok(p), LONG).unwrap();
        assert_eq!(w.decode(vec![1], Some(42)).unwrap().unwrap().timestamp, 42);
        assert!(w.decode(vec![], None).unwrap().is_none());
        assert!(w.flush().unwrap().is_none());
        assert!(!w.is_dead());
    }

    #[test]
    fn engine_is_built_and_dropped_on_the_worker_thread() {
        let (p, dropped, dropped_on) = probe(Duration::ZERO, false);
        let main_thread = thread::current().id();
        let w = Worker::spawn(move || Ok(p), LONG).unwrap();
        drop(w);
        assert!(dropped.load(Ordering::SeqCst), "engine was never dropped");
        let on = dropped_on.lock().unwrap().unwrap();
        assert_ne!(
            on, main_thread,
            "the engine must be dropped by the worker, not the caller"
        );
    }

    /// The regression the whole design exists for: after a timeout the engine must NOT be freed
    /// while its call is still running, and must be freed afterwards by the worker itself.
    #[test]
    fn timeout_does_not_free_the_engine_while_it_is_in_use() {
        let (p, dropped, dropped_on) = probe(Duration::from_millis(600), false);
        let mut w = Worker::spawn(move || Ok(p), SHORT).unwrap();
        assert!(matches!(
            w.decode(vec![1], None),
            Err(WorkerError::TimedOut)
        ));
        assert!(w.is_dead());
        // The decode is still running (600 ms > 150 ms timeout). Even after dropping the handle
        // the engine must still be alive.
        drop(w);
        assert!(
            !dropped.load(Ordering::SeqCst),
            "engine freed while its call was still running"
        );
        // Once the call returns, the worker sees the closed channel and drops the engine itself.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !dropped.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(
            dropped.load(Ordering::SeqCst),
            "abandoned worker never released the engine"
        );
        assert_ne!(dropped_on.lock().unwrap().unwrap(), thread::current().id());
    }

    #[test]
    fn a_dead_worker_refuses_further_calls() {
        let (p, ..) = probe(Duration::from_millis(400), false);
        let mut w = Worker::spawn(move || Ok(p), SHORT).unwrap();
        assert!(matches!(
            w.decode(vec![1], None),
            Err(WorkerError::TimedOut)
        ));
        assert!(matches!(w.decode(vec![1], None), Err(WorkerError::Dead(_))));
        assert!(matches!(w.flush(), Err(WorkerError::Dead(_))));
    }

    #[test]
    fn an_engine_panic_is_reported_not_propagated() {
        let (p, ..) = probe(Duration::ZERO, true);
        let mut w = Worker::spawn(move || Ok(p), LONG).unwrap();
        assert!(matches!(w.decode(vec![1], None), Err(WorkerError::Dead(_))));
        assert!(w.is_dead());
    }

    #[test]
    fn engine_errors_keep_the_worker_alive() {
        struct Failing(AtomicUsize);
        impl Engine for Failing {
            fn decode(
                &mut self,
                _: &[u8],
                _: Option<i64>,
            ) -> Result<Option<DecodedFrame>, EngineError> {
                let n = self.0.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Err(EngineError {
                        kind: ErrorKind::Input,
                        message: "bad input".into(),
                    })
                } else {
                    Ok(Some(frame(n as i64)))
                }
            }
            fn flush(&mut self) -> Result<Option<DecodedFrame>, EngineError> {
                Ok(None)
            }
        }
        let mut w = Worker::spawn(|| Ok(Failing(AtomicUsize::new(0))), LONG).unwrap();
        match w.decode(vec![1], None) {
            Err(WorkerError::Engine(e)) => assert_eq!(e.kind, ErrorKind::Input),
            other => panic!("expected an engine error, got {other:?}"),
        }
        assert!(!w.is_dead());
        assert_eq!(w.decode(vec![1], None).unwrap().unwrap().timestamp, 1);
    }

    #[test]
    fn spawn_reports_an_engine_that_fails_to_start() {
        let err = Worker::spawn::<Probe, _>(
            || {
                Err(EngineError {
                    kind: ErrorKind::Fatal,
                    message: "no library".into(),
                })
            },
            LONG,
        )
        .err()
        .expect("spawn must fail");
        assert_eq!(err.message, "no library");
    }
}
