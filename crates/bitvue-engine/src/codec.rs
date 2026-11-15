//! The one codec identifier shared by every crate.
//!
//! Several crates used to define their own enum for "which codec is this" (`semantic_evidence`,
//! `player::extractor`, `bitvue-decode`, `bitvue-codecs-parser`, the CLI's `ForceCodec`), each
//! with a different set of codecs and different spellings (`H264`/`AVC`, `H265`/`HEVC`). This is
//! the union of them; they are being moved onto it one crate at a time (#437).

use serde::{Deserialize, Serialize};

/// Video codec identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Codec {
    /// H.264/AVC
    Avc,
    /// H.265/HEVC
    Hevc,
    /// VP9
    Vp9,
    /// H.266/VVC
    Vvc,
    /// AV1
    Av1,
    /// MPEG-2
    Mpeg2,
    /// AVS3
    Avs3,
    /// JPEG XS
    JpegXs,
    /// VC-3 (DNxHD/DNxHR)
    Vc3,
    /// Unknown codec
    Unknown,
}

impl Codec {
    /// Every codec but [`Codec::Unknown`].
    pub const KNOWN: [Codec; 9] = [
        Codec::Avc,
        Codec::Hevc,
        Codec::Vp9,
        Codec::Vvc,
        Codec::Av1,
        Codec::Mpeg2,
        Codec::Avs3,
        Codec::JpegXs,
        Codec::Vc3,
    ];
}

impl Codec {
    /// The codec an IVF header's FourCC names, `None` for a FourCC this project does not decode.
    pub fn from_ivf_fourcc(fourcc: &[u8; 4]) -> Option<Codec> {
        match fourcc {
            b"AV01" => Some(Codec::Av1),
            b"VP90" | b"VP9 " => Some(Codec::Vp9),
            _ => None,
        }
    }

    /// The codec an MP4 sample entry (`avc1`, `hev1`, ...) names, `None` for an unknown entry.
    pub fn from_mp4_sample_entry(entry: &str) -> Option<Codec> {
        match entry {
            "av01" => Some(Codec::Av1),
            "avc1" | "avc3" => Some(Codec::Avc),
            "hev1" | "hvc1" => Some(Codec::Hevc),
            _ => None,
        }
    }
}

impl std::fmt::Display for Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Codec::Avc => write!(f, "H.264/AVC"),
            Codec::Hevc => write!(f, "H.265/HEVC"),
            Codec::Vp9 => write!(f, "VP9"),
            Codec::Vvc => write!(f, "H.266/VVC"),
            Codec::Av1 => write!(f, "AV1"),
            Codec::Mpeg2 => write!(f, "MPEG-2"),
            Codec::Avs3 => write!(f, "AVS3"),
            Codec::JpegXs => write!(f, "JPEG XS"),
            Codec::Vc3 => write!(f, "VC-3"),
            Codec::Unknown => write!(f, "Unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_codec_has_a_distinct_name() {
        let mut names: Vec<_> = Codec::KNOWN.iter().map(|c| c.to_string()).collect();
        names.push(Codec::Unknown.to_string());
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    #[test]
    fn ivf_fourccs_name_their_codec() {
        assert_eq!(Codec::from_ivf_fourcc(b"AV01"), Some(Codec::Av1));
        assert_eq!(Codec::from_ivf_fourcc(b"VP90"), Some(Codec::Vp9));
        assert_eq!(Codec::from_ivf_fourcc(b"VP9 "), Some(Codec::Vp9));
        // Lower case is not a FourCC IVF writers produce, and VP8 has no decoder here.
        assert_eq!(Codec::from_ivf_fourcc(b"av01"), None);
        assert_eq!(Codec::from_ivf_fourcc(b"VP80"), None);
        assert_eq!(Codec::from_ivf_fourcc(b"\0\0\0\0"), None);
    }

    #[test]
    fn mp4_sample_entries_name_their_codec() {
        for (entry, codec) in [
            ("av01", Codec::Av1),
            ("avc1", Codec::Avc),
            ("avc3", Codec::Avc),
            ("hev1", Codec::Hevc),
            ("hvc1", Codec::Hevc),
        ] {
            assert_eq!(Codec::from_mp4_sample_entry(entry), Some(codec), "{entry}");
        }
        for unknown in ["AV01", "mp4a", "", "avc"] {
            assert_eq!(Codec::from_mp4_sample_entry(unknown), None, "{unknown}");
        }
    }

    #[test]
    fn the_new_codecs_display() {
        assert_eq!(Codec::Avs3.to_string(), "AVS3");
        assert_eq!(Codec::JpegXs.to_string(), "JPEG XS");
        assert_eq!(Codec::Vc3.to_string(), "VC-3");
    }
}
