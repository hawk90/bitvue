//! Shared per-stream [`StreamAnalyzer`] sessions behind the AV1 frame analysis commands
//! (`get_frame_analysis`, `get_residual_analysis`, `get_deblocking_analysis`,
//! `get_coding_flow_analysis`, `get_codec_extended_info`).
//!
//! The coding units of a frame can only be decoded with the state of the frames before it, so each
//! command used to either decode the frame on its own (wrong for almost every inter frame of a
//! real stream) or would have to decode the whole stream up to it on every call. A session keeps
//! that work: the five commands share one analyzer per open stream, so stepping through frames
//! decodes each frame once.
//!
//! **Identity.** A session is found by a fingerprint of the file's content (length plus a 64-bit
//! hash of every byte), not by the address of the buffer or the path: the commands receive a fresh
//! copy of the bytes on every call, and a reopened file of different content must never be served
//! by an old session. At most [`MAX_SESSIONS`] streams are kept (stream A and B); the least
//! recently used one is dropped.
//!
//! **Thread safety.** Requests run on separate threads. Requests for the same stream serialize on
//! that stream's analyzer; different streams do not contend.

use bitvue_av1_codec::overlay_extraction::{ParsedFrame, StreamAnalyzer};
use std::sync::{Arc, Mutex, OnceLock};

/// Streams kept at once.
const MAX_SESSIONS: usize = 2;

/// Why a frame could not be analyzed.
#[derive(Debug)]
pub enum AnalysisError {
    /// No sequence header in the stream's first packets: nothing can be decoded.
    NoSequenceHeader,
    Other(String),
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisError::NoSequenceHeader => {
                write!(f, "no sequence header found in the first few frames")
            }
            AnalysisError::Other(message) => f.write_str(message),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Fingerprint {
    len: usize,
    hash: u64,
}

/// Length and a hash of every byte, eight at a time.
fn fingerprint(data: &[u8]) -> Fingerprint {
    const K: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let (words, remainder) = data.as_chunks::<8>();
    for chunk in words {
        let word = u64::from_le_bytes(*chunk);
        hash = (hash ^ word).wrapping_mul(K).rotate_left(29);
    }
    for &byte in remainder {
        hash = (hash ^ u64::from(byte)).wrapping_mul(K).rotate_left(29);
    }
    Fingerprint {
        len: data.len(),
        hash,
    }
}

struct Entry {
    key: Fingerprint,
    analyzer: Arc<Mutex<StreamAnalyzer>>,
}

/// Sessions, most recently used last.
#[derive(Default)]
pub struct AnalysisSessions {
    entries: Mutex<Vec<Entry>>,
}

impl AnalysisSessions {
    pub fn new() -> Self {
        Self::default()
    }

    /// The analysis of packet `frame_index` of the IVF file `data`.
    pub fn analyze(
        &self,
        data: &[u8],
        frame_index: usize,
    ) -> Result<Arc<ParsedFrame>, AnalysisError> {
        let key = fingerprint(data);
        let analyzer = self.analyzer_for(key, data)?;
        let mut analyzer = analyzer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        analyzer
            .analyze(frame_index)
            .map_err(|e| AnalysisError::Other(e.to_string()))
    }

    fn analyzer_for(
        &self,
        key: Fingerprint,
        data: &[u8],
    ) -> Result<Arc<Mutex<StreamAnalyzer>>, AnalysisError> {
        if let Some(found) = self.touch(key) {
            return Ok(found);
        }
        // Built outside the lock: parsing the container must not block other streams.
        let analyzer = StreamAnalyzer::new(data).map_err(|e| {
            let message = e.to_string();
            if message.contains("no sequence header") {
                AnalysisError::NoSequenceHeader
            } else {
                AnalysisError::Other(message)
            }
        })?;
        let analyzer = Arc::new(Mutex::new(analyzer));
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        // Another thread may have created it in the meantime: keep that one.
        if let Some(position) = entries.iter().position(|e| e.key == key) {
            let entry = entries.remove(position);
            let found = Arc::clone(&entry.analyzer);
            entries.push(entry);
            return Ok(found);
        }
        entries.push(Entry {
            key,
            analyzer: Arc::clone(&analyzer),
        });
        while entries.len() > MAX_SESSIONS {
            entries.remove(0);
        }
        Ok(analyzer)
    }

    /// The analyzer for `key`, marked most recently used.
    fn touch(&self, key: Fingerprint) -> Option<Arc<Mutex<StreamAnalyzer>>> {
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let position = entries.iter().position(|e| e.key == key)?;
        let entry = entries.remove(position);
        let found = Arc::clone(&entry.analyzer);
        entries.push(entry);
        Some(found)
    }
}

/// The process-wide sessions the commands use.
pub fn sessions() -> &'static AnalysisSessions {
    static SESSIONS: OnceLock<AnalysisSessions> = OnceLock::new();
    SESSIONS.get_or_init(AnalysisSessions::new)
}

/// The analysis of packet `frame_index` of `data`, from the shared sessions.
pub fn analyzed_frame(data: &[u8], frame_index: usize) -> Result<Arc<ParsedFrame>, AnalysisError> {
    sessions().analyze(data, frame_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvue_av1_codec::overlay_extraction::{frame_provenance, Provenance};

    const RAV1E: &[u8] = include_bytes!("../../../test_data/av1_rav1e_testsrc2.ivf");
    const SVT: &[u8] = include_bytes!("../../../test_data/av1_svtav1_testsrc2.ivf");

    #[test]
    fn the_fingerprint_follows_the_content() {
        let copy = RAV1E.to_vec();
        assert_eq!(fingerprint(RAV1E), fingerprint(&copy));
        assert_ne!(fingerprint(RAV1E), fingerprint(SVT));
        let mut changed = RAV1E.to_vec();
        let middle = changed.len() / 2;
        changed[middle] ^= 1;
        assert_ne!(fingerprint(RAV1E), fingerprint(&changed));
        // A change in the tail that is not a multiple of eight bytes.
        let mut tail = RAV1E.to_vec();
        *tail.last_mut().unwrap() ^= 1;
        assert_ne!(fingerprint(RAV1E), fingerprint(&tail));
    }

    #[test]
    fn frames_of_a_stream_are_analyzed_with_the_state_of_the_frames_before_them() {
        let sessions = AnalysisSessions::new();
        // Out of order on purpose: the state must come from decoding forward, not from the order of
        // the requests.
        for index in [7, 2, 11, 0, 5] {
            let frame = sessions.analyze(RAV1E, index).unwrap();
            assert_eq!(
                frame_provenance(&frame),
                Provenance::Verified,
                "packet {index}"
            );
        }
    }

    #[test]
    fn a_reopened_file_of_different_content_gets_its_own_session() {
        let sessions = AnalysisSessions::new();
        let a = sessions.analyze(RAV1E, 3).unwrap();
        let b = sessions.analyze(SVT, 3).unwrap();
        assert_ne!(a.dimensions.sb_size, 0);
        assert_ne!(
            (a.tile_data.len(), a.order_hint),
            (b.tile_data.len(), b.order_hint),
            "two different streams must not share a session"
        );
        // Asking for the first stream again is served by its own (still cached) session.
        let again = sessions.analyze(RAV1E, 3).unwrap();
        assert!(Arc::ptr_eq(&a, &again));
    }

    #[test]
    fn at_most_two_streams_are_kept() {
        let sessions = AnalysisSessions::new();
        let first = sessions.analyze(RAV1E, 1).unwrap();
        sessions.analyze(SVT, 1).unwrap();
        let mut other = RAV1E.to_vec();
        let last = other.len() - 1;
        other[last] ^= 1; // a third stream (a different hash), still valid for the first packets
        sessions.analyze(&other, 1).unwrap();
        // The first stream was the least recently used: it is decoded again, not served.
        let again = sessions.analyze(RAV1E, 1).unwrap();
        assert!(!Arc::ptr_eq(&first, &again));
    }

    #[test]
    fn a_stream_without_a_sequence_header_says_so() {
        let sessions = AnalysisSessions::new();
        let no_header = sessions.analyze(&[0u8; 64], 0);
        assert!(no_header.is_err());
    }
}
