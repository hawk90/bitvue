//! Annex B byte streams (H.264 / H.265 / H.266 spec Annex B): finding the NAL units.
//!
//! ```text
//! byte stream  = { [leading_zero_8bits] [zero_byte] 00 00 01  nal_unit  { trailing_zero_8bits } }
//! ```
//!
//! A NAL unit never contains `00 00 00`, `00 00 01` or `00 00 02` (emulation prevention) and its
//! last byte is never `00`, so the bytes of a NAL unit are exactly those between its start code
//! `00 00 01` and the next one, **minus the zero bytes in front of that next start code** (the
//! optional `zero_byte` and any `trailing_zero_8bits`). Everything here follows that one rule.

/// One NAL unit of an Annex B byte stream, as byte offsets into the data it was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NalRange {
    /// First byte of the start code: the zero byte right in front of `00 00 01` if there is one
    /// (the `zero_byte` of a 4-byte start code; with more zeros than that in front, only the last
    /// one counts, the others are `trailing_zero_8bits` of the unit before), else the first `00`
    /// of `00 00 01`.
    pub start_code: usize,
    /// First byte of the NAL unit (just past `00 00 01`).
    pub start: usize,
    /// One past the last byte of the NAL unit. Always greater than `start`.
    pub end: usize,
}

/// Offsets of every `00 00 01` in `data`, in order and not overlapping.
///
/// One pass: when the third byte of a window is above 1 no start code can begin in the next two
/// positions, so the scan moves three bytes at a time through payload data.
pub fn start_code_prefixes(data: &[u8]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut i = 0;
    while i + 2 < data.len() {
        match data[i + 2] {
            0 => i += 1,
            1 if data[i] == 0 && data[i + 1] == 0 => {
                found.push(i);
                i += 3;
            }
            _ => i += 3,
        }
    }
    found
}

/// The NAL units of `data`, in order. Bytes before the first start code are ignored; a start code
/// with nothing behind it (back to back start codes, or the end of the data) holds no NAL unit and
/// is left out.
pub fn find_nal_ranges(data: &[u8]) -> Vec<NalRange> {
    let prefixes = start_code_prefixes(data);
    let mut units = Vec::with_capacity(prefixes.len());
    for (n, &prefix) in prefixes.iter().enumerate() {
        let start = prefix + 3;
        let limit = prefixes.get(n + 1).copied().unwrap_or(data.len());
        let mut end = limit;
        while end > start && data[end - 1] == 0 {
            end -= 1;
        }
        // (the byte in front of a start code is never the `01` ending the one before it)
        let start_code = if prefix > 0 && data[prefix - 1] == 0 {
            prefix - 1
        } else {
            prefix
        };
        if end > start {
            units.push(NalRange {
                start_code,
                start,
                end,
            });
        }
    }
    units
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triples(data: &[u8]) -> Vec<(usize, usize, usize)> {
        find_nal_ranges(data)
            .iter()
            .map(|r| (r.start_code, r.start, r.end))
            .collect()
    }

    #[test]
    fn three_and_four_byte_start_codes() {
        let data = [0, 0, 0, 1, 0x67, 0x42, 0, 0, 1, 0x68, 0xCE, 0x3C];
        assert_eq!(triples(&data), vec![(0, 4, 6), (6, 9, 12)]);
    }

    #[test]
    fn zero_bytes_before_a_start_code_belong_to_no_unit() {
        // 7 zeros in front of the second start code: trailing_zero_8bits and a zero_byte
        let data = [0, 0, 0, 1, 0xAA, 0xBB, 0, 0, 0, 0, 0, 0, 0, 1, 0xCC];
        assert_eq!(triples(&data), vec![(0, 4, 6), (10, 14, 15)]);
        // zeros at the very end of the data are trailing_zero_8bits too
        assert_eq!(triples(&[0, 0, 1, 0xAA, 0, 0, 0]), vec![(0, 3, 4)]);
    }

    #[test]
    fn a_unit_shorter_than_a_start_code_is_found_and_so_is_the_one_after_it() {
        // 2-byte end-of-sequence NAL, then a 4-byte start code (#421)
        let data = [0, 0, 0, 1, 0x48, 0x01, 0, 0, 0, 1, 0x26, 0x01, 0xAF];
        assert_eq!(triples(&data), vec![(0, 4, 6), (6, 10, 13)]);
        // 1-byte units back to back
        assert_eq!(
            triples(&[0, 0, 1, 7, 0, 0, 1, 8, 0, 0, 1, 9]),
            vec![(0, 3, 4), (4, 7, 8), (8, 11, 12)]
        );
    }

    #[test]
    fn start_codes_with_nothing_behind_them_hold_no_unit() {
        assert!(triples(&[]).is_empty());
        assert!(triples(&[1, 2, 3]).is_empty());
        assert!(triples(&[0, 0, 1]).is_empty());
        assert!(triples(&[0, 0, 1, 0, 0, 1]).is_empty());
        assert!(triples(&[0, 0, 0, 1, 0, 0, 0, 1]).is_empty());
    }

    #[test]
    fn bytes_before_the_first_start_code_are_ignored() {
        assert_eq!(triples(&[0xFF, 0, 0, 1, 0x40, 0x01]), vec![(1, 4, 6)]);
        // leading_zero_8bits: the zero in front of the code is its zero_byte
        assert_eq!(triples(&[0, 0, 0, 0, 1, 0x40]), vec![(1, 5, 6)]);
    }

    #[test]
    fn start_code_prefixes_do_not_overlap() {
        assert_eq!(start_code_prefixes(&[0, 0, 1, 0, 0, 1]), vec![0, 3]);
        assert_eq!(start_code_prefixes(&[0, 0, 0, 0, 1]), vec![2]);
        assert_eq!(start_code_prefixes(&[0, 0, 5, 0, 0, 1]), vec![3]);
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    /// The same definition written the slow, obvious way: look at every offset, no skipping.
    fn reference(data: &[u8]) -> (Vec<usize>, Vec<(usize, usize, usize)>) {
        let mut prefixes = Vec::new();
        let mut i = 0;
        while i + 3 <= data.len() {
            if data[i..i + 3] == [0, 0, 1] {
                prefixes.push(i);
                i += 3;
            } else {
                i += 1;
            }
        }
        let mut units = Vec::new();
        for (n, &p) in prefixes.iter().enumerate() {
            let start = p + 3;
            let limit = *prefixes.get(n + 1).unwrap_or(&data.len());
            let end = (start..limit)
                .rev()
                .find(|&k| data[k] != 0)
                .map_or(start, |k| k + 1);
            let start_code = if p > 0 && data[p - 1] == 0 { p - 1 } else { p };
            if end > start {
                units.push((start_code, start, end));
            }
        }
        (prefixes, units)
    }

    #[test]
    fn agrees_with_the_slow_definition_on_zero_heavy_random_data() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for round in 0..200_000 {
            let len = (rng.next() % 48) as usize;
            let data: Vec<u8> = (0..len)
                .map(|_| match rng.next() % 5 {
                    0..=2 => 0,
                    3 => 1,
                    _ => (rng.next() % 256) as u8,
                })
                .collect();
            let (prefixes, units) = reference(&data);
            assert_eq!(
                start_code_prefixes(&data),
                prefixes,
                "round {round}: {data:?}"
            );
            assert_eq!(triples(&data), units, "round {round}: {data:?}");
        }
    }

    #[test]
    fn built_streams_give_back_every_unit_exactly() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for round in 0..20_000 {
            let (mut data, mut want) = (Vec::new(), Vec::new());
            // leading_zero_8bits before the first start code
            data.extend(std::iter::repeat_n(0, (rng.next() % 3) as usize));
            for _ in 0..1 + rng.next() % 6 {
                if rng.next().is_multiple_of(2) {
                    data.push(0); // zero_byte
                }
                // the last zero in front of the code (zero_byte or a trailing zero), if any
                let start_code = if data.last() == Some(&0) {
                    data.len() - 1
                } else {
                    data.len()
                };
                data.extend_from_slice(&[0, 0, 1]);
                let start = data.len();
                // payload: no zero byte at the end, single zeros inside are fine
                let n = 1 + rng.next() % 9;
                for k in 0..n {
                    // never two zeros in a row: `00 00 01` cannot occur inside a unit
                    let single_zero = k > 0
                        && k < n - 1
                        && data.last() != Some(&0)
                        && rng.next().is_multiple_of(4);
                    data.push(if single_zero {
                        0
                    } else {
                        1 + (rng.next() % 255) as u8
                    });
                }
                let end = data.len();
                // trailing_zero_8bits
                data.extend(std::iter::repeat_n(0, (rng.next() % 3) as usize));
                want.push((start_code, start, end));
            }
            assert_eq!(triples(&data), want, "round {round}: {data:?}");
        }
    }
}
