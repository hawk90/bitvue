//! Provenance - Data lineage tracking for evidence chain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provenance tracks the origin and lineage of data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Provenance {
    /// Source of the data (e.g., "decoder", "parser", "cache")
    pub source: String,
    /// Version of the source that produced this data
    pub source_version: String,
    /// Timestamp when the data was generated
    pub timestamp: String,
    /// Parent provenance IDs (for data derived from multiple sources)
    pub parent_ids: Vec<String>,
    /// Additional context
    pub context: HashMap<String, String>,
}
