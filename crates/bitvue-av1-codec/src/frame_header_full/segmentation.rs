//! `segmentation_params()` (spec 5.9.14).

use super::*;

pub(super) const MAX_SEGMENTS: usize = 8;
pub(super) const SEG_LVL_MAX: usize = 8;
const SEGMENTATION_FEATURE_BITS: [u8; SEG_LVL_MAX] = [8, 6, 6, 6, 6, 3, 0, 0];
const SEGMENTATION_FEATURE_SIGNED: [bool; SEG_LVL_MAX] =
    [true, true, true, true, true, false, false, false];
/// `SEG_LVL_REF_FRAME` (spec's `Segmentation_Feature_Bits` index 5) -- features at or above this
/// index gate `SegIdPreSkip` (`SegmentationInfo::seg_id_pre_skip`'s doc).
pub(crate) const SEG_LVL_REF_FRAME: usize = 5;
/// `SEG_LVL_SKIP` (spec index 6) -- forces `skip = 1`/`RefFrame[0] = LAST_FRAME` with no bits
/// read when active for a CU's segment (`SymbolDecoder::read_is_inter`'s doc,
/// `crate::tile::coding_unit::parse_coding_unit`'s `skip`/`ref_frame` call sites).
pub(crate) const SEG_LVL_SKIP: usize = 6;
/// `SEG_LVL_GLOBALMV` (spec index 7) -- forces `is_inter = 1`/`RefFrame[0] = LAST_FRAME` with no
/// bits read when active (same call sites as `SEG_LVL_SKIP`).
pub(crate) const SEG_LVL_GLOBALMV: usize = 7;

/// Real segmentation state exposed for `segment_id()` (spec 5.11.9/5.11.10) callers -- previously
/// this crate read (for bitstream sync) then discarded every segmentation bit
/// (`skip_segmentation_params`'s original name/doc). `enabled`/`update_map`/`temporal_update` are
/// direct bitstream reads. `seg_id_pre_skip`/`last_active_seg_id` are spec 5.9.14's derived
/// values (`SegIdPreSkip`/`LastActiveSegId`): computed from `FeatureEnabled[seg][feature]` across
/// all `MAX_SEGMENTS`x`SEG_LVL_MAX` cells -- `seg_id_pre_skip` true if any segment has a feature
/// at or above `SEG_LVL_REF_FRAME` enabled (real spec gate for *where* `segment_id()` gets called
/// relative to `skip` -- see `parse_coding_unit`'s call sites), `last_active_seg_id` the highest
/// segment index with any feature enabled (only affects `neg_deinterleave`'s numeric decode, not
/// bitstream position -- verified against dav1d's `read_segment_id`, the symbol read itself is
/// always a fixed 8-way alphabet regardless of this value).
///
/// **Known gap, matching `RefFrameState`'s same class of limitation**: when
/// `segmentation_update_data` is `false` (only possible when `primary_ref_frame !=
/// PRIMARY_REF_NONE` and the encoder explicitly doesn't resend feature data that frame), the real
/// `FeatureEnabled` state carries over from a previous frame -- this crate's production call
/// sites parse each frame independently (`ParsedFrame::parse`, see its `reference_select` field's
/// doc for the same architectural limitation), so there's no real state to carry over. Falls back
/// to `seg_id_pre_skip = false` (matches the common case: QP-only segmentation, e.g. `SEG_LVL_ALT_
/// Q`-based cyclic refresh, never sets a `SEG_LVL_REF_FRAME`+ feature) and `last_active_seg_id =
/// MAX_SEGMENTS - 1` (the safe/permissive bound, doesn't affect bitstream position either way).
/// `seg_id_pre_skip` genuinely gates real bitstream position, so a wrong fallback here is a real
/// (if narrow and documented) desync risk -- not verified against a real `update_data == false`
/// stream, since generating one needs a specific encoder cooperation this session didn't
/// reach. `feature_enabled`/`feature_data` inherit the exact same `update_data == false` gap
/// (all-disabled/all-zero fallback in that case, same as `seg_id_pre_skip`'s).
#[derive(Debug, Clone, Copy)]
pub struct SegmentationInfo {
    pub enabled: bool,
    pub update_map: bool,
    pub temporal_update: bool,
    pub seg_id_pre_skip: bool,
    pub last_active_seg_id: u8,
    /// `FeatureEnabled[segment][feature]` (spec 5.9.14) -- previously read for bit-position sync
    /// only, then discarded (this struct's original gap, closed alongside `feature_data`). Real
    /// spec's `seg_feature_active(feature)` for a given `segment_id` is
    /// `enabled && feature_enabled[segment_id][feature]` -- see `seg_feature_active`.
    pub feature_enabled: [[bool; SEG_LVL_MAX]; MAX_SEGMENTS],
    /// `FeatureData[segment][feature]` (spec 5.9.14) -- only meaningful where the matching
    /// `feature_enabled` cell is `true` (real spec's own convention; an inactive feature's data is
    /// never read at all, so this stays `0` there, not a real "zero override"). See
    /// `seg_feature_data`.
    pub feature_data: [[i16; SEG_LVL_MAX]; MAX_SEGMENTS],
}

impl Default for SegmentationInfo {
    fn default() -> Self {
        Self {
            enabled: false,
            update_map: false,
            temporal_update: false,
            seg_id_pre_skip: false,
            last_active_seg_id: 0,
            feature_enabled: [[false; SEG_LVL_MAX]; MAX_SEGMENTS],
            feature_data: [[0; SEG_LVL_MAX]; MAX_SEGMENTS],
        }
    }
}

impl SegmentationInfo {
    /// `seg_feature_active_idx(segment_id, feature)` (spec 5.9.14 / used throughout 5.11.5-25) --
    /// `false` whenever segmentation itself is off, matching real spec's `enabled` gate being
    /// implicit in every `seg_feature_active` call site (this crate makes it explicit here so
    /// callers don't need to separately check `self.enabled`).
    pub fn seg_feature_active(&self, segment_id: u8, feature: usize) -> bool {
        self.enabled
            && self
                .feature_enabled
                .get(segment_id as usize)
                .is_some_and(|f| f[feature])
    }

    /// `FeatureData[segment_id][feature]` -- `0` if segmentation is off or the feature isn't
    /// active for this segment (real spec never reads a value in that case either).
    pub fn seg_feature_data(&self, segment_id: u8, feature: usize) -> i16 {
        if !self.seg_feature_active(segment_id, feature) {
            return 0;
        }
        self.feature_data
            .get(segment_id as usize)
            .map(|f| f[feature])
            .unwrap_or(0)
    }
}

/// `FeatureEnabled`/`FeatureData` as a frame leaves them (spec `save_segmentation_params`), which
/// a later frame with this frame as its `primary_ref_frame` starts from (`load_previous`) when it
/// does not send new feature data (`segmentation_update_data = 0`).
#[derive(Debug, Clone, Copy, Default)]
pub struct SegmentationFeatures {
    pub feature_enabled: [[bool; SEG_LVL_MAX]; MAX_SEGMENTS],
    pub feature_data: [[i16; SEG_LVL_MAX]; MAX_SEGMENTS],
}

impl From<&SegmentationInfo> for SegmentationFeatures {
    fn from(info: &SegmentationInfo) -> Self {
        Self {
            feature_enabled: info.feature_enabled,
            feature_data: info.feature_data,
        }
    }
}

/// `previous`: the features saved by the `primary_ref_frame` (all clear for
/// `PRIMARY_REF_NONE`), used when this frame enables segmentation without sending new data.
pub(super) fn parse_segmentation_params(
    reader: &mut BitReader,
    primary_ref_frame: u32,
    previous: &SegmentationFeatures,
) -> Result<SegmentationInfo> {
    let enabled = reader.read_bit()?;
    if !enabled {
        return Ok(SegmentationInfo::default());
    }
    let (update_map, temporal_update, update_data) = if primary_ref_frame == PRIMARY_REF_NONE {
        (true, false, true)
    } else {
        let update_map = reader.read_bit()?;
        let temporal_update = if update_map {
            reader.read_bit()?
        } else {
            false
        };
        let update_data = reader.read_bit()?;
        (update_map, temporal_update, update_data)
    };
    let mut seg_id_pre_skip = false;
    let mut last_active_seg_id = 0;
    // Without new data the features are the ones loaded from the primary reference frame.
    let mut feature_enabled = previous.feature_enabled;
    let mut feature_data = previous.feature_data;
    if update_data {
        feature_enabled = [[false; SEG_LVL_MAX]; MAX_SEGMENTS];
        feature_data = [[0i16; SEG_LVL_MAX]; MAX_SEGMENTS];
        for seg in 0..MAX_SEGMENTS {
            for feature in 0..SEG_LVL_MAX {
                let this_feature_enabled = reader.read_bit()?;
                feature_enabled[seg][feature] = this_feature_enabled;
                if this_feature_enabled {
                    let bits = SEGMENTATION_FEATURE_BITS[feature];
                    if bits > 0 {
                        let value = if SEGMENTATION_FEATURE_SIGNED[feature] {
                            reader.read_su(bits + 1)?
                        } else {
                            reader.read_bits(bits)? as i32
                        };
                        feature_data[seg][feature] = value as i16;
                    }
                }
            }
        }
    }
    for (seg, features) in feature_enabled.iter().enumerate() {
        for (feature, &on) in features.iter().enumerate() {
            if on {
                last_active_seg_id = seg as u8;
                if feature >= SEG_LVL_REF_FRAME {
                    seg_id_pre_skip = true;
                }
            }
        }
    }
    Ok(SegmentationInfo {
        enabled,
        update_map,
        temporal_update,
        seg_id_pre_skip,
        last_active_seg_id,
        feature_enabled,
        feature_data,
    })
}
