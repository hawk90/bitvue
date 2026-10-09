//! The CDFs a frame leaves behind for later frames (spec `save_cdfs` after the symbol decoder's
//! exit process, `context_update_tile_id`'s tile), as dav1d's `dav1d_cdf_thread_update` does.
//!
//! Starting from the CDFs the frame itself started with (`self`):
//! - every CDF of the "always" group takes the tile's final adapted value, with its adaptation
//!   counter reset to 0;
//! - the inter-only group (inter modes, references, compound tools, motion vectors) does so only
//!   for inter frames; an intra frame leaves them as they were at its start;
//! - `use_intrabc` and the key-frame luma mode CDFs (`kfym`) are never taken over.
//!
//! Every field of [`CdfContext`] is listed explicitly below, so adding a CDF without deciding
//! which group it belongs to does not compile.

use super::{CdfContext, MvComponentCdfs};

/// Sets the adaptation counter (the last element) of every CDF in a nested structure to zero.
trait ResetCounters {
    fn reset_counters(&mut self);
}

impl ResetCounters for Vec<u16> {
    fn reset_counters(&mut self) {
        if let Some(count) = self.last_mut() {
            *count = 0;
        }
    }
}

impl<T: ResetCounters, const N: usize> ResetCounters for [T; N] {
    fn reset_counters(&mut self) {
        for item in self {
            item.reset_counters();
        }
    }
}

impl ResetCounters for MvComponentCdfs {
    fn reset_counters(&mut self) {
        self.sign.reset_counters();
        self.classes.reset_counters();
        self.class0.reset_counters();
        self.class0_fp.reset_counters();
        self.class_n.reset_counters();
        self.class_n_fp.reset_counters();
        self.class0_hp.reset_counters();
        self.class_n_hp.reset_counters();
    }
}

fn saved<T: ResetCounters + Clone>(tile_final: &T) -> T {
    let mut copy = tile_final.clone();
    copy.reset_counters();
    copy
}

impl CdfContext {
    /// The CDFs to store for a frame that started from `self` and ended its (single) tile with
    /// `tile_final`, when the frame updates its CDFs at the end (`disable_frame_end_update_cdf =
    /// 0`). A frame that does not keeps `self` as is.
    pub fn saved_at_frame_end(&self, tile_final: &CdfContext, frame_is_intra: bool) -> CdfContext {
        CdfContext {
            partition_cdfs: saved(&tile_final.partition_cdfs),
            skip_cdf: saved(&tile_final.skip_cdf),
            skip_mode_cdf: if frame_is_intra {
                self.skip_mode_cdf.clone()
            } else {
                saved(&tile_final.skip_mode_cdf)
            },
            intra_cdf: if frame_is_intra {
                self.intra_cdf.clone()
            } else {
                saved(&tile_final.intra_cdf)
            },
            y_mode_cdf: if frame_is_intra {
                self.y_mode_cdf.clone()
            } else {
                saved(&tile_final.y_mode_cdf)
            },
            motion_mode_cdf: if frame_is_intra {
                self.motion_mode_cdf.clone()
            } else {
                saved(&tile_final.motion_mode_cdf)
            },
            obmc_cdf: if frame_is_intra {
                self.obmc_cdf.clone()
            } else {
                saved(&tile_final.obmc_cdf)
            },
            interintra_cdf: if frame_is_intra {
                self.interintra_cdf.clone()
            } else {
                saved(&tile_final.interintra_cdf)
            },
            interintra_mode_cdf: if frame_is_intra {
                self.interintra_mode_cdf.clone()
            } else {
                saved(&tile_final.interintra_mode_cdf)
            },
            interintra_wedge_cdf: if frame_is_intra {
                self.interintra_wedge_cdf.clone()
            } else {
                saved(&tile_final.interintra_wedge_cdf)
            },
            wedge_comp_cdf: if frame_is_intra {
                self.wedge_comp_cdf.clone()
            } else {
                saved(&tile_final.wedge_comp_cdf)
            },
            wedge_idx_cdf: if frame_is_intra {
                self.wedge_idx_cdf.clone()
            } else {
                saved(&tile_final.wedge_idx_cdf)
            },
            mask_comp_cdf: if frame_is_intra {
                self.mask_comp_cdf.clone()
            } else {
                saved(&tile_final.mask_comp_cdf)
            },
            jnt_comp_cdf: if frame_is_intra {
                self.jnt_comp_cdf.clone()
            } else {
                saved(&tile_final.jnt_comp_cdf)
            },
            filter_cdf: if frame_is_intra {
                self.filter_cdf.clone()
            } else {
                saved(&tile_final.filter_cdf)
            },
            drl_bit_cdf: if frame_is_intra {
                self.drl_bit_cdf.clone()
            } else {
                saved(&tile_final.drl_bit_cdf)
            },
            seg_pred_cdf: if frame_is_intra {
                self.seg_pred_cdf.clone()
            } else {
                saved(&tile_final.seg_pred_cdf)
            },
            seg_id_cdf: saved(&tile_final.seg_id_cdf),
            pal_y_cdf: saved(&tile_final.pal_y_cdf),
            pal_uv_cdf: saved(&tile_final.pal_uv_cdf),
            pal_sz_cdf: saved(&tile_final.pal_sz_cdf),
            color_map_cdf: saved(&tile_final.color_map_cdf),
            angle_delta_cdf: saved(&tile_final.angle_delta_cdf),
            uv_mode_cdf: saved(&tile_final.uv_mode_cdf),
            cfl_sign_cdf: saved(&tile_final.cfl_sign_cdf),
            cfl_alpha_cdf: saved(&tile_final.cfl_alpha_cdf),
            use_filter_intra_cdf: saved(&tile_final.use_filter_intra_cdf),
            filter_intra_mode_cdf: saved(&tile_final.filter_intra_mode_cdf),
            txpart_cdf: saved(&tile_final.txpart_cdf),
            kfym: self.kfym.clone(),
            newmv_mode_cdf: if frame_is_intra {
                self.newmv_mode_cdf.clone()
            } else {
                saved(&tile_final.newmv_mode_cdf)
            },
            globalmv_mode_cdf: if frame_is_intra {
                self.globalmv_mode_cdf.clone()
            } else {
                saved(&tile_final.globalmv_mode_cdf)
            },
            refmv_mode_cdf: if frame_is_intra {
                self.refmv_mode_cdf.clone()
            } else {
                saved(&tile_final.refmv_mode_cdf)
            },
            compound_mode_cdf: if frame_is_intra {
                self.compound_mode_cdf.clone()
            } else {
                saved(&tile_final.compound_mode_cdf)
            },
            mv_joint_cdf: if frame_is_intra {
                self.mv_joint_cdf.clone()
            } else {
                saved(&tile_final.mv_joint_cdf)
            },
            mv_comp: if frame_is_intra {
                self.mv_comp.clone()
            } else {
                saved(&tile_final.mv_comp)
            },
            delta_q_cdf: saved(&tile_final.delta_q_cdf),
            delta_lf_cdf: saved(&tile_final.delta_lf_cdf),
            txb_skip_cdf: saved(&tile_final.txb_skip_cdf),
            coeff_base_cdf: saved(&tile_final.coeff_base_cdf),
            coeff_br_cdf: saved(&tile_final.coeff_br_cdf),
            dc_sign_cdf: saved(&tile_final.dc_sign_cdf),
            eob_bin_16_cdf: saved(&tile_final.eob_bin_16_cdf),
            eob_bin_32_cdf: saved(&tile_final.eob_bin_32_cdf),
            eob_bin_64_cdf: saved(&tile_final.eob_bin_64_cdf),
            eob_bin_128_cdf: saved(&tile_final.eob_bin_128_cdf),
            eob_bin_256_cdf: saved(&tile_final.eob_bin_256_cdf),
            eob_bin_512_cdf: saved(&tile_final.eob_bin_512_cdf),
            eob_bin_1024_cdf: saved(&tile_final.eob_bin_1024_cdf),
            eob_hi_bit_cdf: saved(&tile_final.eob_hi_bit_cdf),
            coeff_base_eob_cdf: saved(&tile_final.coeff_base_eob_cdf),
            txb_skip_cdf_chroma: saved(&tile_final.txb_skip_cdf_chroma),
            dc_sign_cdf_chroma: saved(&tile_final.dc_sign_cdf_chroma),
            eob_bin_16_cdf_chroma: saved(&tile_final.eob_bin_16_cdf_chroma),
            eob_bin_32_cdf_chroma: saved(&tile_final.eob_bin_32_cdf_chroma),
            eob_bin_64_cdf_chroma: saved(&tile_final.eob_bin_64_cdf_chroma),
            eob_bin_128_cdf_chroma: saved(&tile_final.eob_bin_128_cdf_chroma),
            eob_bin_256_cdf_chroma: saved(&tile_final.eob_bin_256_cdf_chroma),
            eob_bin_512_cdf_chroma: saved(&tile_final.eob_bin_512_cdf_chroma),
            eob_bin_1024_cdf_chroma: saved(&tile_final.eob_bin_1024_cdf_chroma),
            eob_hi_bit_cdf_chroma: saved(&tile_final.eob_hi_bit_cdf_chroma),
            coeff_base_eob_cdf_chroma: saved(&tile_final.coeff_base_eob_cdf_chroma),
            coeff_base_cdf_chroma: saved(&tile_final.coeff_base_cdf_chroma),
            coeff_br_cdf_chroma: saved(&tile_final.coeff_br_cdf_chroma),
            txtp_intra1_cdf: saved(&tile_final.txtp_intra1_cdf),
            txtp_intra2_cdf: saved(&tile_final.txtp_intra2_cdf),
            txtp_inter1_cdf: saved(&tile_final.txtp_inter1_cdf),
            txtp_inter2_cdf: saved(&tile_final.txtp_inter2_cdf),
            txtp_inter3_cdf: saved(&tile_final.txtp_inter3_cdf),
            txsz_cdf: saved(&tile_final.txsz_cdf),
            comp_mode_cdf: if frame_is_intra {
                self.comp_mode_cdf.clone()
            } else {
                saved(&tile_final.comp_mode_cdf)
            },
            single_ref_p1_cdf: if frame_is_intra {
                self.single_ref_p1_cdf.clone()
            } else {
                saved(&tile_final.single_ref_p1_cdf)
            },
            single_ref_p2_cdf: if frame_is_intra {
                self.single_ref_p2_cdf.clone()
            } else {
                saved(&tile_final.single_ref_p2_cdf)
            },
            single_ref_p3_cdf: if frame_is_intra {
                self.single_ref_p3_cdf.clone()
            } else {
                saved(&tile_final.single_ref_p3_cdf)
            },
            single_ref_p4_cdf: if frame_is_intra {
                self.single_ref_p4_cdf.clone()
            } else {
                saved(&tile_final.single_ref_p4_cdf)
            },
            single_ref_p5_cdf: if frame_is_intra {
                self.single_ref_p5_cdf.clone()
            } else {
                saved(&tile_final.single_ref_p5_cdf)
            },
            single_ref_p6_cdf: if frame_is_intra {
                self.single_ref_p6_cdf.clone()
            } else {
                saved(&tile_final.single_ref_p6_cdf)
            },
            comp_ref_type_cdf: if frame_is_intra {
                self.comp_ref_type_cdf.clone()
            } else {
                saved(&tile_final.comp_ref_type_cdf)
            },
            uni_comp_ref_cdf: if frame_is_intra {
                self.uni_comp_ref_cdf.clone()
            } else {
                saved(&tile_final.uni_comp_ref_cdf)
            },
            uni_comp_ref_p1_cdf: if frame_is_intra {
                self.uni_comp_ref_p1_cdf.clone()
            } else {
                saved(&tile_final.uni_comp_ref_p1_cdf)
            },
            uni_comp_ref_p2_cdf: if frame_is_intra {
                self.uni_comp_ref_p2_cdf.clone()
            } else {
                saved(&tile_final.uni_comp_ref_p2_cdf)
            },
            comp_ref_cdf: if frame_is_intra {
                self.comp_ref_cdf.clone()
            } else {
                saved(&tile_final.comp_ref_cdf)
            },
            comp_ref_p1_cdf: if frame_is_intra {
                self.comp_ref_p1_cdf.clone()
            } else {
                saved(&tile_final.comp_ref_p1_cdf)
            },
            comp_ref_p2_cdf: if frame_is_intra {
                self.comp_ref_p2_cdf.clone()
            } else {
                saved(&tile_final.comp_ref_p2_cdf)
            },
            comp_bwdref_cdf: if frame_is_intra {
                self.comp_bwdref_cdf.clone()
            } else {
                saved(&tile_final.comp_bwdref_cdf)
            },
            comp_bwdref_p1_cdf: if frame_is_intra {
                self.comp_bwdref_p1_cdf.clone()
            } else {
                saved(&tile_final.comp_bwdref_p1_cdf)
            },
            use_intrabc_cdf: self.use_intrabc_cdf.clone(),
            restore_switchable_cdf: saved(&tile_final.restore_switchable_cdf),
            restore_wiener_cdf: saved(&tile_final.restore_wiener_cdf),
            restore_sgrproj_cdf: saved(&tile_final.restore_sgrproj_cdf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context whose CDFs all differ from the defaults in a recognisable way: first value 1234
    /// and counter 7 in one CDF of each group.
    fn adapted() -> CdfContext {
        let mut c = CdfContext::new();
        c.skip_cdf[0][0] = 1234; // always group
        c.skip_cdf[0][2] = 7;
        c.y_mode_cdf[0][0] = 4321; // inter-only group
        *c.y_mode_cdf[0].last_mut().unwrap() = 7;
        c.mv_comp[1].sign[0] = 2222; // inter-only, nested struct
        c.mv_comp[1].sign[2] = 7;
        c.use_intrabc_cdf[0] = 555; // never taken over
        c.kfym[0][0][0] = 777; // never taken over
        c
    }

    #[test]
    fn an_inter_frame_takes_the_adapted_values_and_resets_every_counter() {
        let start = CdfContext::new();
        let saved = start.saved_at_frame_end(&adapted(), false);

        assert_eq!(saved.skip_cdf[0][0], 1234);
        assert_eq!(saved.skip_cdf[0][2], 0, "counter reset");
        assert_eq!(saved.y_mode_cdf[0][0], 4321);
        assert_eq!(*saved.y_mode_cdf[0].last().unwrap(), 0);
        assert_eq!(saved.mv_comp[1].sign[0], 2222);
        assert_eq!(saved.mv_comp[1].sign[2], 0);
    }

    #[test]
    fn an_intra_frame_leaves_the_inter_only_cdfs_as_they_were_at_its_start() {
        let start = CdfContext::new();
        let saved = start.saved_at_frame_end(&adapted(), true);

        assert_eq!(saved.skip_cdf[0][0], 1234, "shared CDFs are taken over");
        assert_eq!(saved.y_mode_cdf, start.y_mode_cdf);
        assert_eq!(saved.mv_comp[1].sign, start.mv_comp[1].sign);
    }

    #[test]
    fn use_intrabc_and_the_key_frame_mode_cdfs_are_never_taken_over() {
        let start = CdfContext::new();
        for intra in [false, true] {
            let saved = start.saved_at_frame_end(&adapted(), intra);
            assert_eq!(saved.use_intrabc_cdf, start.use_intrabc_cdf);
            assert_eq!(saved.kfym, start.kfym);
        }
    }
}
