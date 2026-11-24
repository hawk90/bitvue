//! Level-derived buffer limits (AV1 spec Annex A.3): the decoder model's `BitRate` and
//! `BufferSize` are not signaled in the stream, they follow from the level, tier and profile of the
//! operating point (spec Annex E.2).

use super::Av1Profile;

/// The decoder model's smoothing-buffer limits for an operating point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferLimits {
    /// `BitRate` = `MaxBitrate` * `BitrateProfileFactor`, bits per second
    pub bit_rate_bps: u64,
    /// `BufferSize` = `MaxBufferSize` * `BitrateProfileFactor` with `MaxBufferSize` = `MaxBitrate`
    /// * 1 second, in bits
    pub buffer_size_bits: u64,
}

/// `MainMbps` and `HighMbps` (megabits per second, x10 to stay integral) of the defined levels,
/// indexed by `seq_level_idx` (spec A.3). `None` marks a level the spec does not define, and a
/// tier a level has no limit for (`HighMbps` exists from level 4.0).
fn mbps_x10(seq_level_idx: u8, high_tier: bool) -> Option<u64> {
    let (main, high): (u64, Option<u64>) = match seq_level_idx {
        0 => (15, None),               // 2.0
        1 => (30, None),               // 2.1
        4 => (60, None),               // 3.0
        5 => (100, None),              // 3.1
        8 => (120, Some(300)),         // 4.0
        9 => (200, Some(500)),         // 4.1
        12 => (300, Some(1000)),       // 5.0
        13 => (400, Some(1600)),       // 5.1
        14 | 15 => (600, Some(2400)),  // 5.2, 5.3
        16 => (600, Some(2400)),       // 6.0
        17 => (1000, Some(4800)),      // 6.1
        18 | 19 => (1600, Some(8000)), // 6.2, 6.3
        _ => return None,
    };
    if high_tier {
        high
    } else {
        Some(main)
    }
}

/// `X.Y` for a `seq_level_idx` (spec A.3: X = 2 + (idx >> 2), Y = idx & 3); `None` for the reserved
/// values 24-30 and for 31, the "maximum parameters" level that has no level-based limits.
pub fn level_name(seq_level_idx: u8) -> Option<String> {
    (seq_level_idx <= 23).then(|| format!("{}.{}", 2 + (seq_level_idx >> 2), seq_level_idx & 3))
}

/// The buffer limits of a level and tier for a profile; `None` when the spec defines none (an
/// undefined level such as 2.2, the maximum-parameters level 31, a high tier below level 4.0, or a
/// reserved profile).
pub fn buffer_limits(
    profile: Av1Profile,
    seq_level_idx: u8,
    high_tier: bool,
) -> Option<BufferLimits> {
    let factor = match profile {
        Av1Profile::Main => 1,
        Av1Profile::High => 2,
        Av1Profile::Professional => 3,
        Av1Profile::Reserved(_) => return None,
    };
    let max_bitrate = mbps_x10(seq_level_idx, high_tier)? * 100_000;
    Some(BufferLimits {
        bit_rate_bps: max_bitrate * factor,
        buffer_size_bits: max_bitrate * factor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_names_follow_the_spec_formula() {
        for (idx, name) in [
            (0, "2.0"),
            (1, "2.1"),
            (4, "3.0"),
            (8, "4.0"),
            (9, "4.1"),
            (19, "6.3"),
        ] {
            assert_eq!(level_name(idx).as_deref(), Some(name), "idx {idx}");
        }
        // 20-23 are levels 7.x (named, but with no limits in the tables)
        assert_eq!(level_name(20).as_deref(), Some("7.0"));
        assert_eq!(level_name(23).as_deref(), Some("7.3"));
        assert_eq!(level_name(24), None);
        assert_eq!(level_name(30), None);
        assert_eq!(level_name(31), None, "maximum parameters level");
    }

    /// Spec A.3's MainMbps / HighMbps column, level by level.
    #[test]
    fn limits_match_the_spec_table() {
        let main = |idx| buffer_limits(Av1Profile::Main, idx, false).map(|l| l.bit_rate_bps);
        let high = |idx| buffer_limits(Av1Profile::Main, idx, true).map(|l| l.bit_rate_bps);
        let table: [(u8, u64, Option<u64>); 14] = [
            (0, 1_500_000, None),
            (1, 3_000_000, None),
            (4, 6_000_000, None),
            (5, 10_000_000, None),
            (8, 12_000_000, Some(30_000_000)),
            (9, 20_000_000, Some(50_000_000)),
            (12, 30_000_000, Some(100_000_000)),
            (13, 40_000_000, Some(160_000_000)),
            (14, 60_000_000, Some(240_000_000)),
            (15, 60_000_000, Some(240_000_000)),
            (16, 60_000_000, Some(240_000_000)),
            (17, 100_000_000, Some(480_000_000)),
            (18, 160_000_000, Some(800_000_000)),
            (19, 160_000_000, Some(800_000_000)),
        ];
        for (idx, main_bps, high_bps) in table {
            assert_eq!(main(idx), Some(main_bps), "main tier, level idx {idx}");
            assert_eq!(high(idx), high_bps, "high tier, level idx {idx}");
        }
    }

    #[test]
    fn buffer_size_is_one_second_of_bit_rate_and_profile_scales_both() {
        let l = buffer_limits(Av1Profile::Main, 9, false).unwrap();
        assert_eq!(l.buffer_size_bits, l.bit_rate_bps);
        let high = buffer_limits(Av1Profile::High, 9, false).unwrap();
        let pro = buffer_limits(Av1Profile::Professional, 9, false).unwrap();
        assert_eq!(high.bit_rate_bps, 2 * l.bit_rate_bps);
        assert_eq!(pro.bit_rate_bps, 3 * l.bit_rate_bps);
        assert_eq!(pro.buffer_size_bits, 3 * l.buffer_size_bits);
    }

    #[test]
    fn undefined_levels_tiers_and_profiles_have_no_limits() {
        assert_eq!(
            buffer_limits(Av1Profile::Main, 2, false),
            None,
            "level 2.2 is undefined"
        );
        assert_eq!(
            buffer_limits(Av1Profile::Main, 20, false),
            None,
            "level 7.0 has no row"
        );
        assert_eq!(buffer_limits(Av1Profile::Main, 31, false), None);
        assert_eq!(
            buffer_limits(Av1Profile::Main, 4, true),
            None,
            "no high tier below 4.0"
        );
        assert_eq!(buffer_limits(Av1Profile::Reserved(3), 8, false), None);
    }
}
