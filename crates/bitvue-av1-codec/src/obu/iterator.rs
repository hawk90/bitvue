//! Iteration over the OBUs of a byte slice.

use bitvue_engine::Result;

use super::parse::parse_obu;
use super::types::Obu;

/// Result of parsing a single OBU, including position information
#[derive(Debug)]
pub struct ObuWithOffset {
    /// The parsed OBU
    pub obu: Obu,
    /// Byte offset of this OBU in the original data
    pub offset: usize,
    /// Number of bytes consumed by this OBU
    pub consumed: usize,
}

/// Iterator over OBUs in a byte slice
///
/// Provides efficient iteration over OBUs with error handling.
/// Use `collect()` to get all OBUs or `for_each()` for processing.
///
/// # Example
/// ```ignore
/// let iter = ObuIterator::new(data);
/// let obus: Result<Vec<_>> = iter.collect();
/// ```
pub struct ObuIterator<'a> {
    data: &'a [u8],
    offset: usize,
    /// Set once `parse_obu` errors, so a truncated/malformed OBU is reported exactly once
    /// instead of forever -- `offset` never advances on `Err` (there's no well-defined "next"
    /// position once parsing has failed), so without this flag `next()`/`next_obu_with_offset()`
    /// would re-run `parse_obu` at the same offset and get the same `Err` on every call,
    /// infinitely. This is a genuine, previously-undiscovered hang: any malformed or truncated
    /// OBU before end-of-data (e.g. a claimed payload size that overruns the buffer) would spin
    /// every `for obu in ObuIterator::new(...)` caller forever -- CLI commands, sidecar commands,
    /// and the indexer all iterate this way. Only surfaced once a compile error in an unrelated
    /// integration test (that had been silently blocking `cargo test --tests` from ever building)
    /// was fixed, letting `test_multi_tile_extraction` actually run for the first time.
    done: bool,
}

impl<'a> ObuIterator<'a> {
    /// Creates a new OBU iterator for the given data
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            offset: 0,
            done: false,
        }
    }

    /// Returns the current offset in the data
    pub fn current_offset(&self) -> usize {
        self.offset
    }

    /// Parses the next OBU with offset information
    ///
    /// Returns `None` when at end of data (or after a parse failure -- see `ObuIterator::done`'s
    /// doc), `Some(Ok(...))` on success, or `Some(Err(...))` on parse failure.
    pub fn next_obu_with_offset(&mut self) -> Option<Result<ObuWithOffset>> {
        if self.done || self.offset >= self.data.len() {
            return None;
        }

        match parse_obu(self.data, self.offset) {
            Ok((obu, consumed)) => {
                let current_offset = self.offset;
                self.offset += consumed;
                Some(Ok(ObuWithOffset {
                    obu,
                    offset: current_offset,
                    consumed,
                }))
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}

impl<'a> Iterator for ObuIterator<'a> {
    type Item = Result<Obu>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.offset >= self.data.len() {
            return None;
        }

        match parse_obu(self.data, self.offset) {
            Ok((obu, consumed)) => {
                self.offset += consumed;
                Some(Ok(obu))
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}
