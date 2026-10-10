//! Unit tests for the CDF tables and accessors.

use super::defaults::to_descending;
use super::*;

#[test]
fn test_partition_cdf_uniform() {
    let cdf = PartitionCdf::uniform(4);
    assert_eq!(cdf.num_symbols, 4);
    assert_eq!(cdf.cdf.len(), 5);
    assert_eq!(cdf.cdf[0], 0);
    assert_eq!(cdf.cdf[4], CDF_SCALE);

    // Check uniform distribution
    assert_eq!(cdf.cdf[1], 8192); // 1/4
    assert_eq!(cdf.cdf[2], 16384); // 2/4
    assert_eq!(cdf.cdf[3], 24576); // 3/4
}

#[test]
fn test_partition_cdf_biased() {
    let cdf = PartitionCdf::biased_none(4);
    assert_eq!(cdf.num_symbols, 4);
    assert_eq!(cdf.cdf[0], 0);
    assert_eq!(cdf.cdf[4], CDF_SCALE);

    // NONE should have highest probability
    assert!(cdf.cdf[1] > cdf.cdf[2] - cdf.cdf[1]);
}

#[test]
fn test_cdf_context_creation() {
    let context = CdfContext::new();

    // Check we have CDFs for all block sizes
    assert_eq!(context.partition_cdfs.len(), 6); // log2(4) to log2(128)
}

#[test]
fn test_cdf_context_get_partition() {
    let mut context = CdfContext::new();

    // 4x4 block (log2 = 2): trivial 1-symbol placeholder, never actually read
    let cdf_4x4 = context.get_partition_cdf_mut(2, 0);
    assert_eq!(cdf_4x4.len(), 2); // 1 symbol + count slot

    // 8x8 block (log2 = 3): 4 real symbols (NONE/HORZ/VERT/SPLIT only)
    let cdf_8x8 = context.get_partition_cdf_mut(3, 0);
    assert_eq!(cdf_8x8.len(), 5); // 4 symbols + count slot

    // 16x16 block (log2 = 4): 10 real symbols
    let cdf_16x16 = context.get_partition_cdf_mut(4, 0);
    assert_eq!(cdf_16x16.len(), 11); // 10 symbols + count slot

    // 128x128 block (log2 = 7): only 8 real symbols (no HORZ_4/VERT_4)
    let cdf_128x128 = context.get_partition_cdf_mut(7, 0);
    assert_eq!(cdf_128x128.len(), 9); // 8 symbols + count slot

    // Every context (0..=3) is independently addressable.
    for ctx in 0..4 {
        let cdf = context.get_partition_cdf_mut(4, ctx);
        assert_eq!(
            cdf.len(),
            11,
            "context {ctx} should have the same alphabet size"
        );
    }
}

#[test]
fn test_cdf_scale() {
    assert_eq!(CDF_SCALE, 32768);
    assert_eq!(CDF_SCALE, 1 << 15);
}

#[test]
fn test_split_or_horz_vert_prob_8x8_no_tail_term() {
    // 8x8/ctx0 real literal: [13636, 7258, 2376, 0, 0] (indices 4..8 don't exist for this
    // 4-symbol alphabet -- HorzA/HorzB/Horz4/VertA/VertB all treated as 0).
    let mut context = CdfContext::new();
    let cdf = context.get_partition_cdf_mut(3, 0);
    assert_eq!(split_or_horz_prob(cdf), 7258); // cdf[1] - 0 + 0
    assert_eq!(split_or_vert_prob(cdf), 8754); // cdf[0] - cdf[1] + cdf[2] - 0
}

#[test]
fn test_split_or_horz_vert_prob_16x16_includes_tail_term() {
    // 16x16/ctx0 real literal (10-symbol alphabet, has Horz4/Vert4):
    // [17171, 11839, 8197, 6062, 5104, 3947, 3167, 2197, 866, 0, 0]
    let mut context = CdfContext::new();
    let cdf = context.get_partition_cdf_mut(4, 0);
    assert_eq!(split_or_horz_prob(cdf), 9351); // 11839 - 5104 + 3947 + (866 - 2197)
    assert_eq!(split_or_vert_prob(cdf), 11693); // 17171-11839+8197-3167 + (2197-866)
}

#[test]
fn test_split_or_horz_vert_prob_128x128_omits_tail_term() {
    // 128x128/ctx0 real literal (8-symbol alphabet, no Horz4/Vert4 at all -- unlike 8x8, the
    // real VertB/HorzB entries here are nonzero, so an unguarded read would wrongly include
    // them in the tail term; the explicit `cdf.len() >= 11` check must skip it instead of
    // relying on padding-as-zero (see `split_or_horz_prob`'s doc).
    let mut context = CdfContext::new();
    let cdf = context.get_partition_cdf_mut(7, 0);
    assert_eq!(cdf, &[4869, 4549, 4239, 284, 229, 149, 129, 0, 0]);
    assert_eq!(split_or_horz_prob(cdf), 4549 - 229 + 149); // no tail term
    assert_eq!(split_or_vert_prob(cdf), 4869 - 4549 + 4239 - 129); // no tail term
}

#[test]
fn test_mv_joint_cdf_spec_compliant() {
    let mut context = CdfContext::new();
    let cdf = context.get_mv_joint_cdf_mut();

    // Verify length: 4 symbols + adaptation count = 5 values
    assert_eq!(cdf.len(), 5);

    assert_eq!(cdf[3], 0, "last real entry should be 0");
    assert_eq!(cdf[4], 0, "adaptation count should start at 0");

    for i in 1..cdf.len() - 1 {
        assert!(
            cdf[i] <= cdf[i - 1],
            "CDF should be monotonically non-increasing at index {}",
            i
        );
    }

    // Verify spec-compliant values from rav1d (same conversion as mv_class above).
    assert_eq!(cdf[0], 28672, "MV_JOINT_ZERO descending threshold");
    assert_eq!(cdf[1], 21504, "MV_JOINT_HNZVZ descending threshold");
    assert_eq!(cdf[2], 13440, "MV_JOINT_HZVNZ descending threshold");
}

#[test]
fn test_to_descending_round_trips_ascending_shape() {
    // A well-formed ascending CDF (cdf[0]=0..cdf[n]=CDF_SCALE) should convert into a
    // well-formed descending one (monotonically non-increasing, last real entry 0, trailing
    // count-slot 0) with the same length.
    let ascending = vec![0u16, 8192, 16384, 24576, CDF_SCALE];
    let descending = to_descending(&ascending);

    assert_eq!(descending.len(), ascending.len());
    assert_eq!(descending, vec![24576, 16384, 8192, 0, 0]);
    for i in 1..descending.len() - 1 {
        assert!(descending[i] <= descending[i - 1]);
    }
}

/// The real dav1d/spec qindex-bucket formula (mirrored at both real production call sites --
/// `overlay_extraction::cu_parser::parse_coding_units_with_outcome` and
/// `overlay_extraction::partition::parse_partition_trees_from_tile_data`) is
/// `qcat = (base_q_idx>20) + (base_q_idx>60) + (base_q_idx>120)`. Confirms the boundary
/// values land on the correct side (dav1d's `>`, not `>=`, so `base_q_idx == 20/60/120`
/// itself stays in the *lower* bucket) -- an off-by-one here would silently apply the wrong
/// bucket to every frame whose QP lands exactly on a boundary.
#[test]
fn qcat_formula_boundaries_match_real_dav1d_thresholds() {
    fn qcat_of(base_q_idx: i16) -> u8 {
        (base_q_idx > 20) as u8 + (base_q_idx > 60) as u8 + (base_q_idx > 120) as u8
    }

    assert_eq!(qcat_of(0), 0);
    assert_eq!(
        qcat_of(20),
        0,
        "== 20 must stay in bucket 0 (dav1d uses > not >=)"
    );
    assert_eq!(qcat_of(21), 1, "21 is the first value in bucket 1");
    assert_eq!(qcat_of(60), 1, "== 60 must stay in bucket 1");
    assert_eq!(qcat_of(61), 2, "61 is the first value in bucket 2");
    assert_eq!(qcat_of(120), 2, "== 120 must stay in bucket 2");
    assert_eq!(qcat_of(121), 3, "121 is the first value in bucket 3");
    assert_eq!(qcat_of(255), 3, "max real base_q_idx stays in bucket 3");
}

/// Non-degenerate check that `new_with_qcat`'s 4 match arms actually hold 4 *different* sets
/// of literals per field -- guards against the failure mode of this port that would be
/// hardest to catch any other way: accidentally copy-pasting bucket 0's numbers into the
/// 1/2/3 arms (still compiles, still produces valid CDFs, still passes every shape/self-check
/// gate the generator script ran -- only a direct value comparison across buckets catches
/// it). Samples one field per distinct shape family (`txb_skip`/`dc_sign`/`eob_bin_16`
/// 2D-plane/`coeff_base_eob`/`coeff_base`/`coeff_br`, plus their `_chroma` siblings) rather
/// than all 26, since a copy-paste bug would essentially never affect only a handful of the
/// 26 fields while sparing the rest (they were all generated by the same script, from the
/// same per-field extraction rule).
#[test]
fn new_with_qcat_buckets_hold_real_distinct_values() {
    let c0 = CdfContext::new_with_qcat(0);
    let c1 = CdfContext::new_with_qcat(1);
    let c2 = CdfContext::new_with_qcat(2);
    let c3 = CdfContext::new_with_qcat(3);

    // Every pairwise combination must differ for each sampled field -- not just "0 != 3"
    // (which alone wouldn't catch e.g. 1 and 2 being accidental duplicates of each other).
    macro_rules! assert_all_buckets_differ {
        ($field:ident) => {
            assert_ne!(
                c0.$field,
                c1.$field,
                "{}: bucket 0 and 1 are identical -- looks like a copy-paste",
                stringify!($field)
            );
            assert_ne!(
                c0.$field,
                c2.$field,
                "{}: bucket 0 and 2 are identical -- looks like a copy-paste",
                stringify!($field)
            );
            assert_ne!(
                c0.$field,
                c3.$field,
                "{}: bucket 0 and 3 are identical -- looks like a copy-paste",
                stringify!($field)
            );
            assert_ne!(
                c1.$field,
                c2.$field,
                "{}: bucket 1 and 2 are identical -- looks like a copy-paste",
                stringify!($field)
            );
            assert_ne!(
                c1.$field,
                c3.$field,
                "{}: bucket 1 and 3 are identical -- looks like a copy-paste",
                stringify!($field)
            );
            assert_ne!(
                c2.$field,
                c3.$field,
                "{}: bucket 2 and 3 are identical -- looks like a copy-paste",
                stringify!($field)
            );
        };
    }

    assert_all_buckets_differ!(txb_skip_cdf);
    assert_all_buckets_differ!(txb_skip_cdf_chroma);
    // `dc_sign` is the one real exception: dav1d's own `default_coef_cdf[0..=3].dc_sign` is
    // *genuinely* byte-for-byte identical across all 4 qindex buckets in the reference source
    // (confirmed directly against `cdf.c` -- every other field's bucket varies, only this
    // one's sign-bit default doesn't) -- so the non-degenerate check here is the opposite of
    // every other field: bucket 0 and 3 must match, proving the port didn't accidentally
    // invent variation dav1d's source doesn't have.
    assert_eq!(
        c0.dc_sign_cdf, c3.dc_sign_cdf,
        "dc_sign_cdf: dav1d's real default is identical across all 4 qindex buckets -- \
         buckets should match exactly, not diverge"
    );
    assert_eq!(
        c0.dc_sign_cdf_chroma, c3.dc_sign_cdf_chroma,
        "dc_sign_cdf_chroma: dav1d's real default is identical across all 4 qindex buckets -- \
         buckets should match exactly, not diverge"
    );
    assert_all_buckets_differ!(eob_bin_16_cdf);
    assert_all_buckets_differ!(eob_bin_16_cdf_chroma);
    assert_all_buckets_differ!(eob_bin_512_cdf);
    assert_all_buckets_differ!(eob_bin_512_cdf_chroma);
    assert_all_buckets_differ!(eob_hi_bit_cdf);
    assert_all_buckets_differ!(eob_hi_bit_cdf_chroma);
    assert_all_buckets_differ!(coeff_base_eob_cdf);
    assert_all_buckets_differ!(coeff_base_eob_cdf_chroma);
    assert_all_buckets_differ!(coeff_base_cdf);
    assert_all_buckets_differ!(coeff_base_cdf_chroma);
    assert_all_buckets_differ!(coeff_br_cdf);
    assert_all_buckets_differ!(coeff_br_cdf_chroma);

    // Every other (qcat-independent) field must stay identical across buckets -- only the
    // residual-coefficient family is qindex-bucketed in real AV1.
    assert_eq!(c0.partition_cdfs, c3.partition_cdfs);
    assert_eq!(c0.skip_cdf, c3.skip_cdf);
    assert_eq!(c0.txsz_cdf, c3.txsz_cdf);
    assert_eq!(c0.txtp_intra1_cdf, c3.txtp_intra1_cdf);
}

/// `qcat.min(3)` should clamp rather than panic for any caller that passes an un-derived
/// value >3 (dav1d's real bucket count) -- matches `new_with_qcat`'s doc.
#[test]
fn new_with_qcat_clamps_values_above_three() {
    let c3 = CdfContext::new_with_qcat(3);
    let c_over = CdfContext::new_with_qcat(200);
    assert_eq!(c3.coeff_base_cdf, c_over.coeff_base_cdf);
    assert_eq!(c3.txb_skip_cdf, c_over.txb_skip_cdf);
}

/// `new()` must stay byte-for-byte equivalent to `new_with_qcat(0)` -- every pre-existing
/// caller (tests, `tile/partition.rs`'s no-real-caller helper) relies on this being a true
/// zero-behavior-change wrapper.
#[test]
fn new_is_equivalent_to_new_with_qcat_zero() {
    let via_new = CdfContext::new();
    let via_explicit = CdfContext::new_with_qcat(0);
    assert_eq!(via_new.txb_skip_cdf, via_explicit.txb_skip_cdf);
    assert_eq!(via_new.coeff_base_cdf, via_explicit.coeff_base_cdf);
    assert_eq!(via_new.coeff_br_cdf, via_explicit.coeff_br_cdf);
    assert_eq!(via_new.dc_sign_cdf, via_explicit.dc_sign_cdf);
    assert_eq!(via_new.partition_cdfs, via_explicit.partition_cdfs);
}

#[test]
fn mv_component_cdfs_match_dav1d_defaults() {
    let mut context = CdfContext::new();
    let comp = context.get_mv_component_cdfs_mut(0);
    assert_eq!(comp.sign[0], 16384);
    assert_eq!(comp.classes.len(), 12);
    assert_eq!(comp.classes[0], 32768 - 28672);
    assert_eq!(comp.class0[0], 32768 - 27648);
    assert_eq!(comp.class0_fp[1][1], 32768 - 21248);
    assert_eq!(comp.class_n[9][0], 32768 - 30720);
    assert_eq!(comp.class_n_fp[2], 32768 - 21248);
    assert_eq!(comp.class0_hp[0], 32768 - 20480);
    assert_eq!(comp.class_n_hp[0], 32768 - 16384);
    // Both components start from identical defaults.
    let other = context.get_mv_component_cdfs_mut(1).classes.clone();
    assert_eq!(context.get_mv_component_cdfs_mut(0).classes, other);
}
