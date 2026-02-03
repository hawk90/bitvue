//! CRC algorithm variants and parameters.
//!
//! This module defines the various CRC algorithms, their parameters,
//! and predefined configurations.

/// CRC algorithm identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrcAlgorithm {
    /// CRC-32 (ISO 3309, used in ZIP, PNG, etc.)
    Crc32,
    /// CRC-32C (Castagnoli, used in iSCSI, SCTP, etc.)
    Crc32C,
    /// CRC-64-ECMA (used in ECMA-182)
    Crc64Ecma,
    /// CRC-64-ISO (used in ISO 3309)
    Crc64Iso,
    /// CRC-16 (used in USB, Modbus, etc.)
    Crc16,
    /// CRC-16-CCITT (used in X.25, Bluetooth, etc.)
    Crc16Ccitt,
    /// CRC-8 (used in 1-Wire, SMBus, etc.)
    Crc8,
}

impl CrcAlgorithm {
    /// Returns the polynomial for this CRC algorithm.
    pub const fn polynomial(&self) -> u64 {
        match self {
            CrcAlgorithm::Crc32 => 0x04c11db7,
            CrcAlgorithm::Crc32C => 0x1edc6f41,
            CrcAlgorithm::Crc64Ecma => 0x42f0e1eba9ea3693,
            CrcAlgorithm::Crc64Iso => 0x000000000000001b,
            CrcAlgorithm::Crc16 => 0x8005,
            CrcAlgorithm::Crc16Ccitt => 0x1021,
            CrcAlgorithm::Crc8 => 0x07,
        }
    }

    /// Returns the initial value for this CRC algorithm.
    pub const fn initial(&self) -> u64 {
        match self {
            CrcAlgorithm::Crc32 | CrcAlgorithm::Crc32C => 0xffffffff,
            CrcAlgorithm::Crc64Ecma | CrcAlgorithm::Crc64Iso => 0xffffffffffffffff,
            CrcAlgorithm::Crc16 => 0x0000,
            CrcAlgorithm::Crc16Ccitt => 0xffff,
            CrcAlgorithm::Crc8 => 0x00,
        }
    }

    /// Returns the final XOR value for this CRC algorithm.
    pub const fn final_xor(&self) -> u64 {
        match self {
            CrcAlgorithm::Crc32 | CrcAlgorithm::Crc32C => 0xffffffff,
            CrcAlgorithm::Crc64Ecma | CrcAlgorithm::Crc64Iso => 0xffffffffffffffff,
            CrcAlgorithm::Crc16 => 0x0000,
            CrcAlgorithm::Crc16Ccitt => 0x0000,
            CrcAlgorithm::Crc8 => 0x00,
        }
    }

    /// Returns the width in bits for this CRC algorithm.
    pub const fn width(&self) -> usize {
        match self {
            CrcAlgorithm::Crc32 | CrcAlgorithm::Crc32C => 32,
            CrcAlgorithm::Crc64Ecma | CrcAlgorithm::Crc64Iso => 64,
            CrcAlgorithm::Crc16 | CrcAlgorithm::Crc16Ccitt => 16,
            CrcAlgorithm::Crc8 => 8,
        }
    }

    /// Returns true if this algorithm reflects input bytes.
    pub const fn reflect_input(&self) -> bool {
        match self {
            CrcAlgorithm::Crc32 | CrcAlgorithm::Crc32C => true,
            CrcAlgorithm::Crc64Ecma | CrcAlgorithm::Crc64Iso => true,
            CrcAlgorithm::Crc16 => true,
            CrcAlgorithm::Crc16Ccitt => false,
            CrcAlgorithm::Crc8 => false,
        }
    }

    /// Returns true if this algorithm reflects the output.
    pub const fn reflect_output(&self) -> bool {
        match self {
            CrcAlgorithm::Crc32 | CrcAlgorithm::Crc32C => true,
            CrcAlgorithm::Crc64Ecma | CrcAlgorithm::Crc64Iso => true,
            CrcAlgorithm::Crc16 => true,
            CrcAlgorithm::Crc16Ccitt => false,
            CrcAlgorithm::Crc8 => false,
        }
    }
}

/// CRC parameters for custom CRC implementations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrcParams {
    /// Polynomial (without the x^width term)
    pub polynomial: u64,
    /// Initial value
    pub initial: u64,
    /// Final XOR value
    pub final_xor: u64,
    /// Width in bits
    pub width: u8,
    /// Whether to reflect input bytes
    pub reflect_input: bool,
    /// Whether to reflect the output
    pub reflect_output: bool,
}

impl CrcParams {
    /// Creates new CRC parameters.
    pub const fn new(
        polynomial: u64,
        initial: u64,
        final_xor: u64,
        width: u8,
        reflect_input: bool,
        reflect_output: bool,
    ) -> Self {
        Self {
            polynomial,
            initial,
            final_xor,
            width,
            reflect_input,
            reflect_output,
        }
    }

    /// Returns parameters for CRC-32.
    pub const fn crc32() -> Self {
        Self {
            polynomial: 0x04c11db7,
            initial: 0xffffffff,
            final_xor: 0xffffffff,
            width: 32,
            reflect_input: true,
            reflect_output: true,
        }
    }

    /// Returns parameters for CRC-32C.
    pub const fn crc32c() -> Self {
        Self {
            polynomial: 0x1edc6f41,
            initial: 0xffffffff,
            final_xor: 0xffffffff,
            width: 32,
            reflect_input: true,
            reflect_output: true,
        }
    }

    /// Returns parameters for CRC-16.
    pub const fn crc16() -> Self {
        Self {
            polynomial: 0x8005,
            initial: 0x0000,
            final_xor: 0x0000,
            width: 16,
            reflect_input: true,
            reflect_output: true,
        }
    }

    /// Returns parameters for CRC-16-CCITT.
    pub const fn crc16_ccitt() -> Self {
        Self {
            polynomial: 0x1021,
            initial: 0xffff,
            final_xor: 0x0000,
            width: 16,
            reflect_input: false,
            reflect_output: false,
        }
    }

    /// Returns parameters for CRC-8.
    pub const fn crc8() -> Self {
        Self {
            polynomial: 0x07,
            initial: 0x00,
            final_xor: 0x00,
            width: 8,
            reflect_input: false,
            reflect_output: false,
        }
    }
}

/// CRC-32/MPEG-2 parameters (used in MPEG-2 video).
pub const CRC32_MPEG_2: CrcParams = CrcParams {
    polynomial: 0x04c11db7,
    initial: 0xffffffff,
    final_xor: 0x00000000,
    width: 32,
    reflect_input: false,
    reflect_output: false,
};

/// CRC-32/BZIP2 parameters (used in bzip2).
pub const CRC32_BZIP2: CrcParams = CrcParams {
    polynomial: 0x04c11db7,
    initial: 0xffffffff,
    final_xor: 0xffffffff,
    width: 32,
    reflect_input: false,
    reflect_output: true,
};

/// CRC-32/POSIX parameters (used in cksum).
pub const CRC32_POSIX: CrcParams = CrcParams {
    polynomial: 0x04c11db7,
    initial: 0x00000000,
    final_xor: 0xffffffff,
    width: 32,
    reflect_input: false,
    reflect_output: false,
};

/// CRC-32/JAMCRC parameters.
pub const CRC32_JAMCRC: CrcParams = CrcParams {
    polynomial: 0x04c11db7,
    initial: 0xffffffff,
    final_xor: 0x00000000,
    width: 32,
    reflect_input: true,
    reflect_output: true,
};

/// CRC-32/XFER parameters.
pub const CRC32_XFER: CrcParams = CrcParams {
    polynomial: 0x000000af,
    initial: 0x00000000,
    final_xor: 0x00000000,
    width: 32,
    reflect_input: false,
    reflect_output: false,
};

/// Additional CRC-32 variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crc32Variant {
    /// Standard CRC-32 (ISO 3309, PKZIP, PNG)
    Standard,
    /// CRC-32C (Castagnoli, iSCSI, SCTP)
    Castagnoli,
    /// CRC-32/MPEG-2
    Mpeg2,
    /// CRC-32/BZIP2
    Bzip2,
    /// CRC-32/POSIX (cksum)
    Posix,
    /// CRC-32/JAMCRC
    Jamcrc,
    /// CRC-32/XFER
    Xfer,
}

impl Crc32Variant {
    /// Returns the parameters for this variant.
    pub const fn params(&self) -> CrcParams {
        match self {
            Crc32Variant::Standard => CrcParams::crc32(),
            Crc32Variant::Castagnoli => CrcParams::crc32c(),
            Crc32Variant::Mpeg2 => CRC32_MPEG_2,
            Crc32Variant::Bzip2 => CRC32_BZIP2,
            Crc32Variant::Posix => CRC32_POSIX,
            Crc32Variant::Jamcrc => CRC32_JAMCRC,
            Crc32Variant::Xfer => CRC32_XFER,
        }
    }

    /// Returns the algorithm identifier.
    pub const fn algorithm(&self) -> CrcAlgorithm {
        match self {
            Crc32Variant::Standard => CrcAlgorithm::Crc32,
            Crc32Variant::Castagnoli => CrcAlgorithm::Crc32C,
            _ => CrcAlgorithm::Crc32,
        }
    }
}

/// Additional CRC-64 variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crc64Variant {
    /// CRC-64-ECMA (ECMA-182, used in HDLC, XMODEM)
    Ecma,
    /// CRC-64-ISO (ISO 3309)
    Iso,
    /// CRC-64-Jones (used in SHR, highway sensors)
    Jones,
}

impl Crc64Variant {
    /// Returns the parameters for this variant.
    pub const fn params(&self) -> CrcParams {
        match self {
            Crc64Variant::Ecma => CrcParams {
                polynomial: 0x42f0e1eba9ea3693,
                initial: 0xffffffffffffffff,
                final_xor: 0xffffffffffffffff,
                width: 64,
                reflect_input: true,
                reflect_output: true,
            },
            Crc64Variant::Iso => CrcParams {
                polynomial: 0x000000000000001b,
                initial: 0xffffffffffffffff,
                final_xor: 0xffffffffffffffff,
                width: 64,
                reflect_input: true,
                reflect_output: true,
            },
            Crc64Variant::Jones => CrcParams {
                polynomial: 0xad93d23594c935a9,
                initial: 0xffffffffffffffff,
                final_xor: 0xffffffffffffffff,
                width: 64,
                reflect_input: true,
                reflect_output: true,
            },
        }
    }
}

/// Predefined CRC-32 configurations.
pub struct Crc32Presets;

impl Crc32Presets {
    pub const STANDARD: CrcParams = CrcParams::crc32();
    pub const CASTAGNOLI: CrcParams = CrcParams::crc32c();
    pub const MPEG2: CrcParams = CRC32_MPEG_2;
    pub const BZIP2: CrcParams = CRC32_BZIP2;
    pub const POSIX: CrcParams = CRC32_POSIX;
    pub const JAMCRC: CrcParams = CRC32_JAMCRC;
    pub const XFER: CrcParams = CRC32_XFER;
}

/// Predefined CRC-64 configurations.
pub struct Crc64Presets;

impl Crc64Presets {
    pub const ECMA: CrcParams = CrcParams {
        polynomial: 0x42f0e1eba9ea3693,
        initial: 0xffffffffffffffff,
        final_xor: 0xffffffffffffffff,
        width: 64,
        reflect_input: true,
        reflect_output: true,
    };

    pub const ISO: CrcParams = CrcParams {
        polynomial: 0x000000000000001b,
        initial: 0xffffffffffffffff,
        final_xor: 0xffffffffffffffff,
        width: 64,
        reflect_input: true,
        reflect_output: true,
    };

    pub const JONES: CrcParams = Crc64Variant::Jones.params();
}

/// Predefined CRC-16 configurations.
pub struct Crc16Presets;

impl Crc16Presets {
    pub const STANDARD: CrcParams = CrcParams::crc16();
    pub const CCITT: CrcParams = CrcParams::crc16_ccitt();

    pub const IBM: CrcParams = CrcParams {
        polynomial: 0x8005,
        initial: 0x0000,
        final_xor: 0x0000,
        width: 16,
        reflect_input: true,
        reflect_output: true,
    };

    pub const DECT_R: CrcParams = CrcParams {
        polynomial: 0x0589,
        initial: 0x0000,
        final_xor: 0x0000,
        width: 16,
        reflect_input: false,
        reflect_output: false,
    };

    pub const DECT_X: CrcParams = CrcParams {
        polynomial: 0x1021,
        initial: 0x0000,
        final_xor: 0x0000,
        width: 16,
        reflect_input: true,
        reflect_output: true,
    };
}

/// Predefined CRC-8 configurations.
pub struct Crc8Presets;

impl Crc8Presets {
    pub const STANDARD: CrcParams = CrcParams::crc8();

    pub const CDMA2000: CrcParams = CrcParams {
        polynomial: 0x9b,
        initial: 0xff,
        final_xor: 0x00,
        width: 8,
        reflect_input: false,
        reflect_output: false,
    };

    pub const DVBS2: CrcParams = CrcParams {
        polynomial: 0xd5,
        initial: 0x00,
        final_xor: 0x00,
        width: 8,
        reflect_input: false,
        reflect_output: false,
    };

    pub const WCDMA: CrcParams = CrcParams {
        polynomial: 0x1d,
        initial: 0xff,
        final_xor: 0x00,
        width: 8,
        reflect_input: true,
        reflect_output: true,
    };

    pub const ROHC: CrcParams = CrcParams {
        polynomial: 0x07,
        initial: 0xff,
        final_xor: 0x00,
        width: 8,
        reflect_input: true,
        reflect_output: true,
    };

    pub const MAXIM: CrcParams = CrcParams {
        polynomial: 0x31,
        initial: 0x00,
        final_xor: 0x00,
        width: 8,
        reflect_input: false,
        reflect_output: false,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc_algorithm_properties() {
        let algo = CrcAlgorithm::Crc32;
        assert_eq!(algo.polynomial(), 0x04c11db7);
        assert_eq!(algo.initial(), 0xffffffff);
        assert_eq!(algo.final_xor(), 0xffffffff);
        assert_eq!(algo.width(), 32);
        assert!(algo.reflect_input());
        assert!(algo.reflect_output());
    }

    #[test]
    fn test_crc32c_properties() {
        let algo = CrcAlgorithm::Crc32C;
        assert_eq!(algo.polynomial(), 0x1edc6f41);
        assert_eq!(algo.initial(), 0xffffffff);
        assert_eq!(algo.final_xor(), 0xffffffff);
        assert_eq!(algo.width(), 32);
    }

    #[test]
    fn test_crc64_properties() {
        let algo = CrcAlgorithm::Crc64Ecma;
        assert_eq!(algo.polynomial(), 0x42f0e1eba9ea3693);
        assert_eq!(algo.initial(), 0xffffffffffffffff);
        assert_eq!(algo.final_xor(), 0xffffffffffffffff);
        assert_eq!(algo.width(), 64);
    }

    #[test]
    fn test_crc_params_crc32() {
        let params = CrcParams::crc32();
        assert_eq!(params.polynomial, 0x04c11db7);
        assert_eq!(params.initial, 0xffffffff);
        assert_eq!(params.final_xor, 0xffffffff);
        assert_eq!(params.width, 32);
    }

    #[test]
    fn test_crc_params_crc32c() {
        let params = CrcParams::crc32c();
        assert_eq!(params.polynomial, 0x1edc6f41);
        assert_eq!(params.initial, 0xffffffff);
        assert_eq!(params.final_xor(), 0xffffffff);
        assert_eq!(params.width, 32);
    }

    #[test]
    fn test_crc32_variant_params() {
        assert_eq!(Crc32Variant::Standard.params().polynomial, 0x04c11db7);
        assert_eq!(Crc32Variant::Castagnoli.params().polynomial, 0x1edc6f41);
    }

    #[test]
    fn test_crc64_variant_params() {
        let ecma_params = Crc64Variant::Ecma.params();
        assert_eq!(ecma_params.polynomial, 0x42f0e1eba9ea3693);
        assert_eq!(ecma_params.width, 64);
    }
}
