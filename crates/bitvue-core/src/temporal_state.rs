//! Temporal State Evidence Layer - Extended Evidence Chain Layer 5
//!
//! Tracks DPB (Decoded Picture Buffer) state and reference frame evolution
//! over time for animation and insight generation.
//!
//! Per PERFECT_VISUALIZATION_SPEC: Layer 5 enables DPB animation and
//! reference tracking visualization.

use serde::{Deserialize, Serialize};

use crate::evidence::EvidenceId;
use crate::semantic_evidence::Codec;

// ═══════════════════════════════════════════════════════════════════════════
// Reference Frame Types
// ═══════════════════════════════════════════════════════════════════════════

/// Temporal reference frame type (codec-specific, distinct from reference_graph::ReferenceType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemporalRefType {
    // AV1 reference types
    Last,
    Last2,
    Last3,
    Golden,
    Bwdref,
    Altref2,
    Altref,
    Intrabc,

    // H.264/HEVC reference types
    ShortTerm,
    LongTerm,

    // VP9 reference types
    Vp9Last,
    Vp9Golden,
    Vp9Altref,

    // Generic
    L0, // Forward reference
    L1, // Backward reference
}

impl TemporalRefType {
    pub fn is_forward(&self) -> bool {
        matches!(
            self,
            Self::Last
                | Self::Last2
                | Self::Last3
                | Self::Golden
                | Self::Vp9Last
                | Self::Vp9Golden
                | Self::ShortTerm
                | Self::L0
        )
    }

    pub fn is_backward(&self) -> bool {
        matches!(
            self,
            Self::Bwdref | Self::Altref | Self::Altref2 | Self::Vp9Altref | Self::L1
        )
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Last => "LAST",
            Self::Last2 => "LAST2",
            Self::Last3 => "LAST3",
            Self::Golden => "GOLDEN",
            Self::Bwdref => "BWD",
            Self::Altref2 => "ALTREF2",
            Self::Altref => "ALTREF",
            Self::Intrabc => "INTRABC",
            Self::ShortTerm => "ST",
            Self::LongTerm => "LT",
            Self::Vp9Last => "LAST",
            Self::Vp9Golden => "GOLDEN",
            Self::Vp9Altref => "ALTREF",
            Self::L0 => "L0",
            Self::L1 => "L1",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Reference Slot State
// ═══════════════════════════════════════════════════════════════════════════

/// A single reference frame slot in the DPB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceSlot {
    /// Slot index (0-7 for AV1, 0-15 for H.264, etc.)
    pub slot_idx: u8,

    /// Frame display index stored in this slot
    pub frame_display_idx: u64,

    /// Frame decode index (internal)
    pub frame_decode_idx: u64,

    /// Reference type (how this slot is used)
    pub ref_type: TemporalRefType,

    /// How many times this reference has been used
    pub usage_count: u32,

    /// Age in frames since insertion
    pub age: u32,

    /// Picture order count (codec-specific)
    pub poc: Option<i32>,

    /// Is this a long-term reference?
    pub is_long_term: bool,

    /// Timestamp when inserted
    pub inserted_at_display_idx: u64,
}

impl ReferenceSlot {
    pub fn new(
        slot_idx: u8,
        frame_display_idx: u64,
        frame_decode_idx: u64,
        ref_type: TemporalRefType,
    ) -> Self {
        Self {
            slot_idx,
            frame_display_idx,
            frame_decode_idx,
            ref_type,
            usage_count: 0,
            age: 0,
            poc: None,
            is_long_term: false,
            inserted_at_display_idx: frame_display_idx,
        }
    }

    pub fn increment_usage(&mut self) {
        self.usage_count += 1;
    }

    pub fn increment_age(&mut self) {
        self.age += 1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DPB State Events
// ═══════════════════════════════════════════════════════════════════════════

/// Event type for DPB state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DpbEvent {
    /// Frame inserted into DPB
    Inserted {
        slot_idx: u8,
        frame_display_idx: u64,
        ref_type: TemporalRefType,
        reason: String,
    },

    /// Frame evicted from DPB
    Evicted {
        slot_idx: u8,
        frame_display_idx: u64,
        reason: EvictionReason,
    },

    /// Reference type changed (e.g., short-term to long-term)
    TypeChanged {
        slot_idx: u8,
        old_type: TemporalRefType,
        new_type: TemporalRefType,
        reason: String,
    },

    /// Frame marked as used for reference
    Referenced {
        slot_idx: u8,
        by_frame_display_idx: u64,
        prediction_direction: PredictionDirection,
    },

    /// DPB flushed (e.g., at IDR)
    Flushed {
        reason: String,
        frames_evicted: Vec<u64>,
    },
}

/// Reason for evicting a frame from DPB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionReason {
    SlidingWindow,
    MmcoMarkUnused,
    BufferFull,
    IdrFlush,
    KeyFrameFlush,
    Replaced { by_frame: u64 },
    ManualEviction,
}

impl EvictionReason {
    pub fn description(&self) -> String {
        match self {
            Self::SlidingWindow => "Sliding window eviction (oldest frame)".into(),
            Self::MmcoMarkUnused => "MMCO: marked as unused".into(),
            Self::BufferFull => "DPB full, evicted to make room".into(),
            Self::IdrFlush => "IDR frame: DPB flushed".into(),
            Self::KeyFrameFlush => "Key frame: DPB flushed".into(),
            Self::Replaced { by_frame } => format!("Replaced by frame {}", by_frame),
            Self::ManualEviction => "Manual eviction".into(),
        }
    }
}

/// Prediction direction for reference usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionDirection {
    Forward,  // L0
    Backward, // L1
    Bidirectional,
}

// ═══════════════════════════════════════════════════════════════════════════
// DPB State Snapshot
// ═══════════════════════════════════════════════════════════════════════════

/// Complete DPB state at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpbStateSnapshot {
    /// Frame display index when this snapshot was taken
    pub at_display_idx: u64,

    /// Frame decode index (internal)
    pub at_decode_idx: u64,

    /// Codec for this DPB
    pub codec: Codec,

    /// Maximum DPB capacity
    pub max_capacity: u8,

    /// Current occupancy
    pub current_occupancy: u8,

    /// All reference slots
    pub slots: Vec<ReferenceSlot>,

    /// Events that occurred at this frame
    pub events: Vec<DpbEvent>,

    /// Active references used by current frame
    pub active_refs: Vec<ActiveReference>,
}

/// Reference actively used by current frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveReference {
    pub slot_idx: u8,
    pub ref_type: TemporalRefType,
    pub frame_display_idx: u64,
    pub weight: Option<f32>, // For weighted prediction
}

impl DpbStateSnapshot {
    pub fn new(at_display_idx: u64, at_decode_idx: u64, codec: Codec, max_capacity: u8) -> Self {
        Self {
            at_display_idx,
            at_decode_idx,
            codec,
            max_capacity,
            current_occupancy: 0,
            slots: Vec::new(),
            events: Vec::new(),
            active_refs: Vec::new(),
        }
    }

    /// Get buffer fullness as percentage
    pub fn fullness_percent(&self) -> f32 {
        if self.max_capacity == 0 {
            0.0
        } else {
            (self.current_occupancy as f32 / self.max_capacity as f32) * 100.0
        }
    }

    /// Find slot by frame display index
    pub fn find_slot_by_frame(&self, frame_display_idx: u64) -> Option<&ReferenceSlot> {
        self.slots
            .iter()
            .find(|s| s.frame_display_idx == frame_display_idx)
    }

    /// Find slot by reference type
    pub fn find_slot_by_type(&self, ref_type: TemporalRefType) -> Option<&ReferenceSlot> {
        self.slots.iter().find(|s| s.ref_type == ref_type)
    }

    /// Get empty slot indices
    pub fn empty_slots(&self) -> Vec<u8> {
        let occupied: Vec<u8> = self.slots.iter().map(|s| s.slot_idx).collect();
        (0..self.max_capacity)
            .filter(|i| !occupied.contains(i))
            .collect()
    }

    /// Add an event
    pub fn add_event(&mut self, event: DpbEvent) {
        self.events.push(event);
    }

    /// Add a slot
    pub fn add_slot(&mut self, slot: ReferenceSlot) {
        self.slots.push(slot);
        self.current_occupancy = self.slots.len() as u8;
    }

    /// Remove a slot by index
    pub fn remove_slot(&mut self, slot_idx: u8) -> Option<ReferenceSlot> {
        if let Some(pos) = self.slots.iter().position(|s| s.slot_idx == slot_idx) {
            let slot = self.slots.remove(pos);
            self.current_occupancy = self.slots.len() as u8;
            Some(slot)
        } else {
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Temporal State Timeline
// ═══════════════════════════════════════════════════════════════════════════

/// Complete temporal state history for animation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemporalStateTimeline {
    /// DPB snapshots indexed by display_idx
    snapshots: Vec<DpbStateSnapshot>,

    /// Reference usage graph: (from_frame, to_frame, ref_type)
    reference_edges: Vec<TemporalReferenceEdge>,
}

/// Edge in the temporal reference graph (distinct from reference_graph::ReferenceEdge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalReferenceEdge {
    pub from_display_idx: u64,
    pub to_display_idx: u64,
    pub ref_type: TemporalRefType,
    pub weight: Option<f32>,
}

impl TemporalStateTimeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a DPB snapshot
    pub fn add_snapshot(&mut self, snapshot: DpbStateSnapshot) {
        self.snapshots.push(snapshot);
        self.snapshots.sort_by_key(|s| s.at_display_idx);
    }

    /// Add a reference edge
    pub fn add_reference_edge(&mut self, edge: TemporalReferenceEdge) {
        self.reference_edges.push(edge);
    }

    /// Get snapshot at specific display index
    pub fn get_snapshot(&self, display_idx: u64) -> Option<&DpbStateSnapshot> {
        self.snapshots
            .iter()
            .find(|s| s.at_display_idx == display_idx)
    }

    /// Get snapshot at or before display index
    pub fn get_snapshot_at_or_before(&self, display_idx: u64) -> Option<&DpbStateSnapshot> {
        self.snapshots
            .iter()
            .rev()
            .find(|s| s.at_display_idx <= display_idx)
    }

    /// Get all snapshots in range
    pub fn get_snapshots_in_range(&self, start: u64, end: u64) -> Vec<&DpbStateSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.at_display_idx >= start && s.at_display_idx <= end)
            .collect()
    }

    /// Get outgoing references from a frame
    pub fn get_outgoing_refs(&self, from_display_idx: u64) -> Vec<&TemporalReferenceEdge> {
        self.reference_edges
            .iter()
            .filter(|e| e.from_display_idx == from_display_idx)
            .collect()
    }

    /// Get incoming references to a frame (who uses this frame as reference)
    pub fn get_incoming_refs(&self, to_display_idx: u64) -> Vec<&TemporalReferenceEdge> {
        self.reference_edges
            .iter()
            .filter(|e| e.to_display_idx == to_display_idx)
            .collect()
    }

    /// Get reference chain (all frames this frame depends on, recursively)
    pub fn get_reference_chain(&self, display_idx: u64, max_depth: usize) -> Vec<u64> {
        let mut chain = Vec::new();
        let mut to_visit = vec![(display_idx, 0)];
        let mut visited = std::collections::HashSet::new();

        while let Some((idx, depth)) = to_visit.pop() {
            if depth > max_depth || visited.contains(&idx) {
                continue;
            }
            visited.insert(idx);

            for edge in self.get_outgoing_refs(idx) {
                if !chain.contains(&edge.to_display_idx) {
                    chain.push(edge.to_display_idx);
                    to_visit.push((edge.to_display_idx, depth + 1));
                }
            }
        }

        chain
    }

    /// Get frames that depend on this frame (recursively)
    pub fn get_dependent_chain(&self, display_idx: u64, max_depth: usize) -> Vec<u64> {
        let mut chain = Vec::new();
        let mut to_visit = vec![(display_idx, 0)];
        let mut visited = std::collections::HashSet::new();

        while let Some((idx, depth)) = to_visit.pop() {
            if depth > max_depth || visited.contains(&idx) {
                continue;
            }
            visited.insert(idx);

            for edge in self.get_incoming_refs(idx) {
                if !chain.contains(&edge.from_display_idx) {
                    chain.push(edge.from_display_idx);
                    to_visit.push((edge.from_display_idx, depth + 1));
                }
            }
        }

        chain
    }

    /// Get total number of snapshots
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Get all snapshots
    pub fn all_snapshots(&self) -> &[DpbStateSnapshot] {
        &self.snapshots
    }

    /// Get all reference edges
    pub fn all_edges(&self) -> &[TemporalReferenceEdge] {
        &self.reference_edges
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Temporal State Evidence Record
// ═══════════════════════════════════════════════════════════════════════════

/// Evidence record linking temporal state to evidence chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalStateEvidence {
    /// Unique evidence ID
    pub id: EvidenceId,

    /// Display index this evidence applies to
    pub display_idx: u64,

    /// Link to semantic evidence (if applicable)
    pub semantic_link: Option<EvidenceId>,

    /// Link to decode evidence
    pub decode_link: EvidenceId,

    /// Snapshot of DPB state
    pub dpb_state: DpbStateSnapshot,
}

impl TemporalStateEvidence {
    pub fn new(
        id: EvidenceId,
        display_idx: u64,
        decode_link: EvidenceId,
        dpb_state: DpbStateSnapshot,
    ) -> Self {
        Self {
            id,
            display_idx,
            semantic_link: None,
            decode_link,
            dpb_state,
        }
    }
}

/// Index for temporal state evidence
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemporalStateIndex {
    evidence: Vec<TemporalStateEvidence>,
}

impl TemporalStateIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, evidence: TemporalStateEvidence) {
        self.evidence.push(evidence);
        self.evidence.sort_by_key(|e| e.display_idx);
    }

    pub fn find_by_id(&self, id: &str) -> Option<&TemporalStateEvidence> {
        self.evidence.iter().find(|e| e.id == id)
    }

    pub fn find_by_display_idx(&self, display_idx: u64) -> Option<&TemporalStateEvidence> {
        self.evidence.iter().find(|e| e.display_idx == display_idx)
    }

    pub fn find_by_decode_link(&self, decode_id: &str) -> Vec<&TemporalStateEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.decode_link == decode_id)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.evidence.len()
    }

    pub fn is_empty(&self) -> bool {
        self.evidence.is_empty()
    }

    pub fn all(&self) -> &[TemporalStateEvidence] {
        &self.evidence
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
include!("temporal_state_test.rs");
