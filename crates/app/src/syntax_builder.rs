//! Syntax Builder - Converts parsed AV1 structures to SyntaxModel
//!
//! Phase 0: Uses new syntax parser with bit-level tracking

use bitvue_core::{Result, SyntaxModel};

/// Build a syntax model from raw OBU data
///
/// This function uses the new syntax parser (bitvue_av1::parse_obu_syntax)
/// which provides bit-level tracking for Tri-sync functionality.
///
/// # Arguments
///
/// * `obu_data` - Raw OBU bytes (including header, size, and payload)
/// * `obu_index` - Index of this OBU in the stream
/// * `global_offset` - Absolute bit offset from file start
///
/// # Returns
///
/// A complete SyntaxModel with all parsed fields and bit ranges
pub fn build_syntax_from_obu_data(
    obu_data: &[u8],
    obu_index: usize,
    global_offset: u64,
) -> Result<SyntaxModel> {
    bitvue_av1::parse_obu_syntax(obu_data, obu_index, global_offset)
}
