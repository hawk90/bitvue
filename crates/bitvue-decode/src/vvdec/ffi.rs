//! Raw bindings to Fraunhofer vvdec 3.x (`vvdec/vvdec.h`). **Generated, do not edit by hand.**
//!
//! The previous hand-written copy of these declarations did not match the header (wrong struct
//! sizes: `vvdecParams` 32 vs 56 bytes, `vvdecPlane` 24 vs 32, `vvdecFrame` 120 vs 152; wrong error
//! codes), which is undefined behaviour. bindgen reads the real header and emits compile-time
//! layout assertions (`size_of`/`offset_of`), so a drifting header breaks the build instead of
//! corrupting memory.
//!
//! Regenerate (bindgen-cli 0.73.2, vvdec 3.2.0 headers; `FNS` is the `|`-joined list of the
//! functions named below):
//!
//! ```text
//! bindgen vvdec/vvdec.h \
//!   --allowlist-function '^(vvdec_params_default|vvdec_decoder_open|vvdec_decoder_close|\
//! vvdec_decode|vvdec_flush|vvdec_frame_unref|vvdec_accessUnit_alloc|vvdec_accessUnit_free|\
//! vvdec_accessUnit_alloc_payload|vvdec_accessUnit_free_payload|vvdec_get_error_msg|\
//! vvdec_get_version|vvdec_get_last_error|vvdec_get_last_additional_error)$' \
//!   --allowlist-type '^vvdec(Params|Frame|Plane|AccessUnit|PicAttributes|Decoder|ColorFormat|\
//! SliceType|NalType|LogLevel|SIMD_Extension|ErrHandlingFlags|ErrorCodes)$' \
//!   --newtype-enum 'vvdec.*' --no-doc-comments --use-core --no-derive-debug \
//!   -o ffi.rs -- -x c -I <prefix>/include
//! ```
//!
//! C enums are newtypes (`vvdecColorFormat(i32)`), not Rust enums: a value the header does not
//! list (a newer library) is then still a valid Rust value instead of undefined behaviour.

#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    clippy::all,
    unsafe_op_in_unsafe_fn
)]

#[repr(C)]
#[derive(Debug)]
pub struct vvdecDecoder {
    _unused: [u8; 0],
}
impl vvdecErrorCodes {
    pub const VVDEC_OK: vvdecErrorCodes = vvdecErrorCodes(0);
    pub const VVDEC_ERR_UNSPECIFIED: vvdecErrorCodes = vvdecErrorCodes(-1);
    pub const VVDEC_ERR_INITIALIZE: vvdecErrorCodes = vvdecErrorCodes(-2);
    pub const VVDEC_ERR_ALLOCATE: vvdecErrorCodes = vvdecErrorCodes(-3);
    pub const VVDEC_ERR_DEC_INPUT: vvdecErrorCodes = vvdecErrorCodes(-4);
    pub const VVDEC_NOT_ENOUGH_MEM: vvdecErrorCodes = vvdecErrorCodes(-5);
    pub const VVDEC_ERR_PARAMETER: vvdecErrorCodes = vvdecErrorCodes(-7);
    pub const VVDEC_ERR_NOT_SUPPORTED: vvdecErrorCodes = vvdecErrorCodes(-10);
    pub const VVDEC_ERR_RESTART_REQUIRED: vvdecErrorCodes = vvdecErrorCodes(-11);
    pub const VVDEC_ERR_CPU: vvdecErrorCodes = vvdecErrorCodes(-30);
    pub const VVDEC_TRY_AGAIN: vvdecErrorCodes = vvdecErrorCodes(-40);
    pub const VVDEC_EOF: vvdecErrorCodes = vvdecErrorCodes(-50);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecErrorCodes(pub ::core::ffi::c_int);
impl vvdecLogLevel {
    pub const VVDEC_SILENT: vvdecLogLevel = vvdecLogLevel(0);
    pub const VVDEC_ERROR: vvdecLogLevel = vvdecLogLevel(1);
    pub const VVDEC_WARNING: vvdecLogLevel = vvdecLogLevel(2);
    pub const VVDEC_INFO: vvdecLogLevel = vvdecLogLevel(3);
    pub const VVDEC_NOTICE: vvdecLogLevel = vvdecLogLevel(4);
    pub const VVDEC_VERBOSE: vvdecLogLevel = vvdecLogLevel(5);
    pub const VVDEC_DETAILS: vvdecLogLevel = vvdecLogLevel(6);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecLogLevel(pub ::core::ffi::c_uint);
impl vvdecPicHashError {
    pub const VVDEC_DPH_NOT_VERIFIED: vvdecPicHashError = vvdecPicHashError(-1);
    pub const VVDEC_DPH_OK: vvdecPicHashError = vvdecPicHashError(0);
    pub const VVDEC_DPH_MISMATCH: vvdecPicHashError = vvdecPicHashError(1);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecPicHashError(pub ::core::ffi::c_int);
impl vvdecSIMD_Extension {
    pub const VVDEC_SIMD_DEFAULT: vvdecSIMD_Extension = vvdecSIMD_Extension(0);
    pub const VVDEC_SIMD_SCALAR: vvdecSIMD_Extension = vvdecSIMD_Extension(1);
    pub const VVDEC_SIMD_LEGACY: vvdecSIMD_Extension = vvdecSIMD_Extension(2);
    pub const VVDEC_SIMD_NEON: vvdecSIMD_Extension = vvdecSIMD_Extension(3);
    pub const VVDEC_SIMD_NEON_RDM: vvdecSIMD_Extension = vvdecSIMD_Extension(4);
    pub const VVDEC_SIMD_SVE: vvdecSIMD_Extension = vvdecSIMD_Extension(5);
    pub const VVDEC_SIMD_SVE2: vvdecSIMD_Extension = vvdecSIMD_Extension(6);
    pub const VVDEC_SIMD_MAX: vvdecSIMD_Extension = vvdecSIMD_Extension(6);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecSIMD_Extension(pub ::core::ffi::c_uint);
impl vvdecErrHandlingFlags {
    pub const VVDEC_ERR_HANDLING_OFF: vvdecErrHandlingFlags = vvdecErrHandlingFlags(0);
    pub const VVDEC_ERR_HANDLING_TRY_CONTINUE: vvdecErrHandlingFlags = vvdecErrHandlingFlags(1);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecErrHandlingFlags(pub ::core::ffi::c_uint);
impl vvdecColorFormat {
    pub const VVDEC_CF_INVALID: vvdecColorFormat = vvdecColorFormat(-1);
    pub const VVDEC_CF_YUV400_PLANAR: vvdecColorFormat = vvdecColorFormat(0);
    pub const VVDEC_CF_YUV420_PLANAR: vvdecColorFormat = vvdecColorFormat(1);
    pub const VVDEC_CF_YUV422_PLANAR: vvdecColorFormat = vvdecColorFormat(2);
    pub const VVDEC_CF_YUV444_PLANAR: vvdecColorFormat = vvdecColorFormat(3);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecColorFormat(pub ::core::ffi::c_int);
impl vvdecFrameFormat {
    pub const VVDEC_FF_INVALID: vvdecFrameFormat = vvdecFrameFormat(-1);
    pub const VVDEC_FF_PROGRESSIVE: vvdecFrameFormat = vvdecFrameFormat(0);
    pub const VVDEC_FF_TOP_FIELD: vvdecFrameFormat = vvdecFrameFormat(1);
    pub const VVDEC_FF_BOT_FIELD: vvdecFrameFormat = vvdecFrameFormat(2);
    pub const VVDEC_FF_TOP_BOT: vvdecFrameFormat = vvdecFrameFormat(3);
    pub const VVDEC_FF_BOT_TOP: vvdecFrameFormat = vvdecFrameFormat(4);
    pub const VVDEC_FF_TOP_BOT_TOP: vvdecFrameFormat = vvdecFrameFormat(5);
    pub const VVDEC_FF_BOT_TOP_BOT: vvdecFrameFormat = vvdecFrameFormat(6);
    pub const VVDEC_FF_FRAME_DOUB: vvdecFrameFormat = vvdecFrameFormat(7);
    pub const VVDEC_FF_FRAME_TRIP: vvdecFrameFormat = vvdecFrameFormat(8);
    pub const VVDEC_FF_TOP_PW_PREV: vvdecFrameFormat = vvdecFrameFormat(9);
    pub const VVDEC_FF_BOT_PW_PREV: vvdecFrameFormat = vvdecFrameFormat(10);
    pub const VVDEC_FF_TOP_PW_NEXT: vvdecFrameFormat = vvdecFrameFormat(11);
    pub const VVDEC_FF_BOT_PW_NEXT: vvdecFrameFormat = vvdecFrameFormat(12);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecFrameFormat(pub ::core::ffi::c_int);
impl vvdecSliceType {
    pub const VVDEC_SLICETYPE_I: vvdecSliceType = vvdecSliceType(0);
    pub const VVDEC_SLICETYPE_P: vvdecSliceType = vvdecSliceType(1);
    pub const VVDEC_SLICETYPE_B: vvdecSliceType = vvdecSliceType(2);
    pub const VVDEC_SLICETYPE_UNKNOWN: vvdecSliceType = vvdecSliceType(3);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecSliceType(pub ::core::ffi::c_uint);
impl vvdecNalType {
    pub const VVC_NAL_UNIT_CODED_SLICE_TRAIL: vvdecNalType = vvdecNalType(0);
    pub const VVC_NAL_UNIT_CODED_SLICE_STSA: vvdecNalType = vvdecNalType(1);
    pub const VVC_NAL_UNIT_CODED_SLICE_RADL: vvdecNalType = vvdecNalType(2);
    pub const VVC_NAL_UNIT_CODED_SLICE_RASL: vvdecNalType = vvdecNalType(3);
    pub const VVC_NAL_UNIT_RESERVED_VCL_4: vvdecNalType = vvdecNalType(4);
    pub const VVC_NAL_UNIT_RESERVED_VCL_5: vvdecNalType = vvdecNalType(5);
    pub const VVC_NAL_UNIT_RESERVED_VCL_6: vvdecNalType = vvdecNalType(6);
    pub const VVC_NAL_UNIT_CODED_SLICE_IDR_W_RADL: vvdecNalType = vvdecNalType(7);
    pub const VVC_NAL_UNIT_CODED_SLICE_IDR_N_LP: vvdecNalType = vvdecNalType(8);
    pub const VVC_NAL_UNIT_CODED_SLICE_CRA: vvdecNalType = vvdecNalType(9);
    pub const VVC_NAL_UNIT_CODED_SLICE_GDR: vvdecNalType = vvdecNalType(10);
    pub const VVC_NAL_UNIT_RESERVED_IRAP_VCL_11: vvdecNalType = vvdecNalType(11);
    pub const VVC_NAL_UNIT_RESERVED_IRAP_VCL_12: vvdecNalType = vvdecNalType(12);
    pub const VVC_NAL_UNIT_DCI: vvdecNalType = vvdecNalType(13);
    pub const VVC_NAL_UNIT_VPS: vvdecNalType = vvdecNalType(14);
    pub const VVC_NAL_UNIT_SPS: vvdecNalType = vvdecNalType(15);
    pub const VVC_NAL_UNIT_PPS: vvdecNalType = vvdecNalType(16);
    pub const VVC_NAL_UNIT_PREFIX_APS: vvdecNalType = vvdecNalType(17);
    pub const VVC_NAL_UNIT_SUFFIX_APS: vvdecNalType = vvdecNalType(18);
    pub const VVC_NAL_UNIT_PH: vvdecNalType = vvdecNalType(19);
    pub const VVC_NAL_UNIT_ACCESS_UNIT_DELIMITER: vvdecNalType = vvdecNalType(20);
    pub const VVC_NAL_UNIT_EOS: vvdecNalType = vvdecNalType(21);
    pub const VVC_NAL_UNIT_EOB: vvdecNalType = vvdecNalType(22);
    pub const VVC_NAL_UNIT_PREFIX_SEI: vvdecNalType = vvdecNalType(23);
    pub const VVC_NAL_UNIT_SUFFIX_SEI: vvdecNalType = vvdecNalType(24);
    pub const VVC_NAL_UNIT_FD: vvdecNalType = vvdecNalType(25);
    pub const VVC_NAL_UNIT_RESERVED_NVCL_26: vvdecNalType = vvdecNalType(26);
    pub const VVC_NAL_UNIT_RESERVED_NVCL_27: vvdecNalType = vvdecNalType(27);
    pub const VVC_NAL_UNIT_UNSPECIFIED_28: vvdecNalType = vvdecNalType(28);
    pub const VVC_NAL_UNIT_UNSPECIFIED_29: vvdecNalType = vvdecNalType(29);
    pub const VVC_NAL_UNIT_UNSPECIFIED_30: vvdecNalType = vvdecNalType(30);
    pub const VVC_NAL_UNIT_UNSPECIFIED_31: vvdecNalType = vvdecNalType(31);
    pub const VVC_NAL_UNIT_INVALID: vvdecNalType = vvdecNalType(32);
}
#[repr(transparent)]
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct vvdecNalType(pub ::core::ffi::c_uint);
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecAccessUnit {
    pub payload: *mut ::core::ffi::c_uchar,
    pub payloadSize: ::core::ffi::c_int,
    pub payloadUsedSize: ::core::ffi::c_int,
    pub cts: u64,
    pub dts: u64,
    pub ctsValid: bool,
    pub dtsValid: bool,
    pub rap: bool,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecAccessUnit"][::core::mem::size_of::<vvdecAccessUnit>() - 40usize];
    ["Alignment of vvdecAccessUnit"][::core::mem::align_of::<vvdecAccessUnit>() - 8usize];
    ["Offset of field: vvdecAccessUnit::payload"]
        [::core::mem::offset_of!(vvdecAccessUnit, payload) - 0usize];
    ["Offset of field: vvdecAccessUnit::payloadSize"]
        [::core::mem::offset_of!(vvdecAccessUnit, payloadSize) - 8usize];
    ["Offset of field: vvdecAccessUnit::payloadUsedSize"]
        [::core::mem::offset_of!(vvdecAccessUnit, payloadUsedSize) - 12usize];
    ["Offset of field: vvdecAccessUnit::cts"]
        [::core::mem::offset_of!(vvdecAccessUnit, cts) - 16usize];
    ["Offset of field: vvdecAccessUnit::dts"]
        [::core::mem::offset_of!(vvdecAccessUnit, dts) - 24usize];
    ["Offset of field: vvdecAccessUnit::ctsValid"]
        [::core::mem::offset_of!(vvdecAccessUnit, ctsValid) - 32usize];
    ["Offset of field: vvdecAccessUnit::dtsValid"]
        [::core::mem::offset_of!(vvdecAccessUnit, dtsValid) - 33usize];
    ["Offset of field: vvdecAccessUnit::rap"]
        [::core::mem::offset_of!(vvdecAccessUnit, rap) - 34usize];
};
unsafe extern "C" {
    pub fn vvdec_accessUnit_alloc() -> *mut vvdecAccessUnit;
}
unsafe extern "C" {
    pub fn vvdec_accessUnit_free(accessUnit: *mut vvdecAccessUnit);
}
unsafe extern "C" {
    pub fn vvdec_accessUnit_alloc_payload(
        accessUnit: *mut vvdecAccessUnit,
        payload_size: ::core::ffi::c_int,
    );
}
unsafe extern "C" {
    pub fn vvdec_accessUnit_free_payload(accessUnit: *mut vvdecAccessUnit);
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecVui {
    pub aspectRatioInfoPresentFlag: bool,
    pub aspectRatioConstantFlag: bool,
    pub nonPackedFlag: bool,
    pub nonProjectedFlag: bool,
    pub aspectRatioIdc: ::core::ffi::c_int,
    pub sarWidth: ::core::ffi::c_int,
    pub sarHeight: ::core::ffi::c_int,
    pub colourDescriptionPresentFlag: bool,
    pub colourPrimaries: ::core::ffi::c_int,
    pub transferCharacteristics: ::core::ffi::c_int,
    pub matrixCoefficients: ::core::ffi::c_int,
    pub progressiveSourceFlag: bool,
    pub interlacedSourceFlag: bool,
    pub chromaLocInfoPresentFlag: bool,
    pub chromaSampleLocTypeTopField: ::core::ffi::c_int,
    pub chromaSampleLocTypeBottomField: ::core::ffi::c_int,
    pub chromaSampleLocType: ::core::ffi::c_int,
    pub overscanInfoPresentFlag: bool,
    pub overscanAppropriateFlag: bool,
    pub videoSignalTypePresentFlag: bool,
    pub videoFullRangeFlag: bool,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecVui"][::core::mem::size_of::<vvdecVui>() - 52usize];
    ["Alignment of vvdecVui"][::core::mem::align_of::<vvdecVui>() - 4usize];
    ["Offset of field: vvdecVui::aspectRatioInfoPresentFlag"]
        [::core::mem::offset_of!(vvdecVui, aspectRatioInfoPresentFlag) - 0usize];
    ["Offset of field: vvdecVui::aspectRatioConstantFlag"]
        [::core::mem::offset_of!(vvdecVui, aspectRatioConstantFlag) - 1usize];
    ["Offset of field: vvdecVui::nonPackedFlag"]
        [::core::mem::offset_of!(vvdecVui, nonPackedFlag) - 2usize];
    ["Offset of field: vvdecVui::nonProjectedFlag"]
        [::core::mem::offset_of!(vvdecVui, nonProjectedFlag) - 3usize];
    ["Offset of field: vvdecVui::aspectRatioIdc"]
        [::core::mem::offset_of!(vvdecVui, aspectRatioIdc) - 4usize];
    ["Offset of field: vvdecVui::sarWidth"][::core::mem::offset_of!(vvdecVui, sarWidth) - 8usize];
    ["Offset of field: vvdecVui::sarHeight"]
        [::core::mem::offset_of!(vvdecVui, sarHeight) - 12usize];
    ["Offset of field: vvdecVui::colourDescriptionPresentFlag"]
        [::core::mem::offset_of!(vvdecVui, colourDescriptionPresentFlag) - 16usize];
    ["Offset of field: vvdecVui::colourPrimaries"]
        [::core::mem::offset_of!(vvdecVui, colourPrimaries) - 20usize];
    ["Offset of field: vvdecVui::transferCharacteristics"]
        [::core::mem::offset_of!(vvdecVui, transferCharacteristics) - 24usize];
    ["Offset of field: vvdecVui::matrixCoefficients"]
        [::core::mem::offset_of!(vvdecVui, matrixCoefficients) - 28usize];
    ["Offset of field: vvdecVui::progressiveSourceFlag"]
        [::core::mem::offset_of!(vvdecVui, progressiveSourceFlag) - 32usize];
    ["Offset of field: vvdecVui::interlacedSourceFlag"]
        [::core::mem::offset_of!(vvdecVui, interlacedSourceFlag) - 33usize];
    ["Offset of field: vvdecVui::chromaLocInfoPresentFlag"]
        [::core::mem::offset_of!(vvdecVui, chromaLocInfoPresentFlag) - 34usize];
    ["Offset of field: vvdecVui::chromaSampleLocTypeTopField"]
        [::core::mem::offset_of!(vvdecVui, chromaSampleLocTypeTopField) - 36usize];
    ["Offset of field: vvdecVui::chromaSampleLocTypeBottomField"]
        [::core::mem::offset_of!(vvdecVui, chromaSampleLocTypeBottomField) - 40usize];
    ["Offset of field: vvdecVui::chromaSampleLocType"]
        [::core::mem::offset_of!(vvdecVui, chromaSampleLocType) - 44usize];
    ["Offset of field: vvdecVui::overscanInfoPresentFlag"]
        [::core::mem::offset_of!(vvdecVui, overscanInfoPresentFlag) - 48usize];
    ["Offset of field: vvdecVui::overscanAppropriateFlag"]
        [::core::mem::offset_of!(vvdecVui, overscanAppropriateFlag) - 49usize];
    ["Offset of field: vvdecVui::videoSignalTypePresentFlag"]
        [::core::mem::offset_of!(vvdecVui, videoSignalTypePresentFlag) - 50usize];
    ["Offset of field: vvdecVui::videoFullRangeFlag"]
        [::core::mem::offset_of!(vvdecVui, videoFullRangeFlag) - 51usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecHrd {
    pub numUnitsInTick: u32,
    pub timeScale: u32,
    pub generalNalHrdParamsPresentFlag: bool,
    pub generalVclHrdParamsPresentFlag: bool,
    pub generalSamePicTimingInAllOlsFlag: bool,
    pub tickDivisor: u32,
    pub generalDecodingUnitHrdParamsPresentFlag: bool,
    pub bitRateScale: u32,
    pub cpbSizeScale: u32,
    pub cpbSizeDuScale: u32,
    pub hrdCpbCnt: u32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecHrd"][::core::mem::size_of::<vvdecHrd>() - 36usize];
    ["Alignment of vvdecHrd"][::core::mem::align_of::<vvdecHrd>() - 4usize];
    ["Offset of field: vvdecHrd::numUnitsInTick"]
        [::core::mem::offset_of!(vvdecHrd, numUnitsInTick) - 0usize];
    ["Offset of field: vvdecHrd::timeScale"][::core::mem::offset_of!(vvdecHrd, timeScale) - 4usize];
    ["Offset of field: vvdecHrd::generalNalHrdParamsPresentFlag"]
        [::core::mem::offset_of!(vvdecHrd, generalNalHrdParamsPresentFlag) - 8usize];
    ["Offset of field: vvdecHrd::generalVclHrdParamsPresentFlag"]
        [::core::mem::offset_of!(vvdecHrd, generalVclHrdParamsPresentFlag) - 9usize];
    ["Offset of field: vvdecHrd::generalSamePicTimingInAllOlsFlag"]
        [::core::mem::offset_of!(vvdecHrd, generalSamePicTimingInAllOlsFlag) - 10usize];
    ["Offset of field: vvdecHrd::tickDivisor"]
        [::core::mem::offset_of!(vvdecHrd, tickDivisor) - 12usize];
    ["Offset of field: vvdecHrd::generalDecodingUnitHrdParamsPresentFlag"]
        [::core::mem::offset_of!(vvdecHrd, generalDecodingUnitHrdParamsPresentFlag) - 16usize];
    ["Offset of field: vvdecHrd::bitRateScale"]
        [::core::mem::offset_of!(vvdecHrd, bitRateScale) - 20usize];
    ["Offset of field: vvdecHrd::cpbSizeScale"]
        [::core::mem::offset_of!(vvdecHrd, cpbSizeScale) - 24usize];
    ["Offset of field: vvdecHrd::cpbSizeDuScale"]
        [::core::mem::offset_of!(vvdecHrd, cpbSizeDuScale) - 28usize];
    ["Offset of field: vvdecHrd::hrdCpbCnt"]
        [::core::mem::offset_of!(vvdecHrd, hrdCpbCnt) - 32usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecOlsHrd {
    pub fixedPicRateGeneralFlag: bool,
    pub fixedPicRateWithinCvsFlag: bool,
    pub elementDurationInTc: u32,
    pub lowDelayHrdFlag: bool,
    pub bitRateValueMinus1: [[u32; 2usize]; 32usize],
    pub cpbSizeValueMinus1: [[u32; 2usize]; 32usize],
    pub ducpbSizeValueMinus1: [[u32; 2usize]; 32usize],
    pub duBitRateValueMinus1: [[u32; 2usize]; 32usize],
    pub cbrFlag: [[bool; 2usize]; 32usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecOlsHrd"][::core::mem::size_of::<vvdecOlsHrd>() - 1100usize];
    ["Alignment of vvdecOlsHrd"][::core::mem::align_of::<vvdecOlsHrd>() - 4usize];
    ["Offset of field: vvdecOlsHrd::fixedPicRateGeneralFlag"]
        [::core::mem::offset_of!(vvdecOlsHrd, fixedPicRateGeneralFlag) - 0usize];
    ["Offset of field: vvdecOlsHrd::fixedPicRateWithinCvsFlag"]
        [::core::mem::offset_of!(vvdecOlsHrd, fixedPicRateWithinCvsFlag) - 1usize];
    ["Offset of field: vvdecOlsHrd::elementDurationInTc"]
        [::core::mem::offset_of!(vvdecOlsHrd, elementDurationInTc) - 4usize];
    ["Offset of field: vvdecOlsHrd::lowDelayHrdFlag"]
        [::core::mem::offset_of!(vvdecOlsHrd, lowDelayHrdFlag) - 8usize];
    ["Offset of field: vvdecOlsHrd::bitRateValueMinus1"]
        [::core::mem::offset_of!(vvdecOlsHrd, bitRateValueMinus1) - 12usize];
    ["Offset of field: vvdecOlsHrd::cpbSizeValueMinus1"]
        [::core::mem::offset_of!(vvdecOlsHrd, cpbSizeValueMinus1) - 268usize];
    ["Offset of field: vvdecOlsHrd::ducpbSizeValueMinus1"]
        [::core::mem::offset_of!(vvdecOlsHrd, ducpbSizeValueMinus1) - 524usize];
    ["Offset of field: vvdecOlsHrd::duBitRateValueMinus1"]
        [::core::mem::offset_of!(vvdecOlsHrd, duBitRateValueMinus1) - 780usize];
    ["Offset of field: vvdecOlsHrd::cbrFlag"]
        [::core::mem::offset_of!(vvdecOlsHrd, cbrFlag) - 1036usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecSeqInfo {
    pub maxWidth: u32,
    pub maxHeight: u32,
    pub reservedPtr_1: *mut ::core::ffi::c_void,
    pub reservedPtr_2: *mut ::core::ffi::c_void,
    pub maxLatencyIncreasePlus1: u32,
    pub maxNumReorderPics: u8,
    pub reserved_1: [i8; 3usize],
    pub reserved_2: i64,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecSeqInfo"][::core::mem::size_of::<vvdecSeqInfo>() - 40usize];
    ["Alignment of vvdecSeqInfo"][::core::mem::align_of::<vvdecSeqInfo>() - 8usize];
    ["Offset of field: vvdecSeqInfo::maxWidth"]
        [::core::mem::offset_of!(vvdecSeqInfo, maxWidth) - 0usize];
    ["Offset of field: vvdecSeqInfo::maxHeight"]
        [::core::mem::offset_of!(vvdecSeqInfo, maxHeight) - 4usize];
    ["Offset of field: vvdecSeqInfo::reservedPtr_1"]
        [::core::mem::offset_of!(vvdecSeqInfo, reservedPtr_1) - 8usize];
    ["Offset of field: vvdecSeqInfo::reservedPtr_2"]
        [::core::mem::offset_of!(vvdecSeqInfo, reservedPtr_2) - 16usize];
    ["Offset of field: vvdecSeqInfo::maxLatencyIncreasePlus1"]
        [::core::mem::offset_of!(vvdecSeqInfo, maxLatencyIncreasePlus1) - 24usize];
    ["Offset of field: vvdecSeqInfo::maxNumReorderPics"]
        [::core::mem::offset_of!(vvdecSeqInfo, maxNumReorderPics) - 28usize];
    ["Offset of field: vvdecSeqInfo::reserved_1"]
        [::core::mem::offset_of!(vvdecSeqInfo, reserved_1) - 29usize];
    ["Offset of field: vvdecSeqInfo::reserved_2"]
        [::core::mem::offset_of!(vvdecSeqInfo, reserved_2) - 32usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecPicAttributes {
    pub nalType: vvdecNalType,
    pub sliceType: vvdecSliceType,
    pub isRefPic: bool,
    pub temporalLayer: u32,
    pub poc: i64,
    pub bits: u32,
    pub vui: *mut vvdecVui,
    pub hrd: *mut vvdecHrd,
    pub olsHrd: *mut vvdecOlsHrd,
    pub seqInfo: *mut vvdecSeqInfo,
    pub picHashError: vvdecPicHashError,
    pub userData: *mut ::core::ffi::c_void,
    pub reservedPtr_2: *mut ::core::ffi::c_void,
    pub reserved_1: i64,
    pub reserved_2: i64,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecPicAttributes"][::core::mem::size_of::<vvdecPicAttributes>() - 104usize];
    ["Alignment of vvdecPicAttributes"][::core::mem::align_of::<vvdecPicAttributes>() - 8usize];
    ["Offset of field: vvdecPicAttributes::nalType"]
        [::core::mem::offset_of!(vvdecPicAttributes, nalType) - 0usize];
    ["Offset of field: vvdecPicAttributes::sliceType"]
        [::core::mem::offset_of!(vvdecPicAttributes, sliceType) - 4usize];
    ["Offset of field: vvdecPicAttributes::isRefPic"]
        [::core::mem::offset_of!(vvdecPicAttributes, isRefPic) - 8usize];
    ["Offset of field: vvdecPicAttributes::temporalLayer"]
        [::core::mem::offset_of!(vvdecPicAttributes, temporalLayer) - 12usize];
    ["Offset of field: vvdecPicAttributes::poc"]
        [::core::mem::offset_of!(vvdecPicAttributes, poc) - 16usize];
    ["Offset of field: vvdecPicAttributes::bits"]
        [::core::mem::offset_of!(vvdecPicAttributes, bits) - 24usize];
    ["Offset of field: vvdecPicAttributes::vui"]
        [::core::mem::offset_of!(vvdecPicAttributes, vui) - 32usize];
    ["Offset of field: vvdecPicAttributes::hrd"]
        [::core::mem::offset_of!(vvdecPicAttributes, hrd) - 40usize];
    ["Offset of field: vvdecPicAttributes::olsHrd"]
        [::core::mem::offset_of!(vvdecPicAttributes, olsHrd) - 48usize];
    ["Offset of field: vvdecPicAttributes::seqInfo"]
        [::core::mem::offset_of!(vvdecPicAttributes, seqInfo) - 56usize];
    ["Offset of field: vvdecPicAttributes::picHashError"]
        [::core::mem::offset_of!(vvdecPicAttributes, picHashError) - 64usize];
    ["Offset of field: vvdecPicAttributes::userData"]
        [::core::mem::offset_of!(vvdecPicAttributes, userData) - 72usize];
    ["Offset of field: vvdecPicAttributes::reservedPtr_2"]
        [::core::mem::offset_of!(vvdecPicAttributes, reservedPtr_2) - 80usize];
    ["Offset of field: vvdecPicAttributes::reserved_1"]
        [::core::mem::offset_of!(vvdecPicAttributes, reserved_1) - 88usize];
    ["Offset of field: vvdecPicAttributes::reserved_2"]
        [::core::mem::offset_of!(vvdecPicAttributes, reserved_2) - 96usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecPlane {
    pub ptr: *mut ::core::ffi::c_uchar,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub bytesPerSample: u32,
    pub allocator: *mut ::core::ffi::c_void,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecPlane"][::core::mem::size_of::<vvdecPlane>() - 32usize];
    ["Alignment of vvdecPlane"][::core::mem::align_of::<vvdecPlane>() - 8usize];
    ["Offset of field: vvdecPlane::ptr"][::core::mem::offset_of!(vvdecPlane, ptr) - 0usize];
    ["Offset of field: vvdecPlane::width"][::core::mem::offset_of!(vvdecPlane, width) - 8usize];
    ["Offset of field: vvdecPlane::height"][::core::mem::offset_of!(vvdecPlane, height) - 12usize];
    ["Offset of field: vvdecPlane::stride"][::core::mem::offset_of!(vvdecPlane, stride) - 16usize];
    ["Offset of field: vvdecPlane::bytesPerSample"]
        [::core::mem::offset_of!(vvdecPlane, bytesPerSample) - 20usize];
    ["Offset of field: vvdecPlane::allocator"]
        [::core::mem::offset_of!(vvdecPlane, allocator) - 24usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecFrame {
    pub planes: [vvdecPlane; 3usize],
    pub numPlanes: u32,
    pub width: u32,
    pub height: u32,
    pub bitDepth: u32,
    pub frameFormat: vvdecFrameFormat,
    pub colorFormat: vvdecColorFormat,
    pub sequenceNumber: u64,
    pub cts: u64,
    pub ctsValid: bool,
    pub picAttributes: *mut vvdecPicAttributes,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecFrame"][::core::mem::size_of::<vvdecFrame>() - 152usize];
    ["Alignment of vvdecFrame"][::core::mem::align_of::<vvdecFrame>() - 8usize];
    ["Offset of field: vvdecFrame::planes"][::core::mem::offset_of!(vvdecFrame, planes) - 0usize];
    ["Offset of field: vvdecFrame::numPlanes"]
        [::core::mem::offset_of!(vvdecFrame, numPlanes) - 96usize];
    ["Offset of field: vvdecFrame::width"][::core::mem::offset_of!(vvdecFrame, width) - 100usize];
    ["Offset of field: vvdecFrame::height"][::core::mem::offset_of!(vvdecFrame, height) - 104usize];
    ["Offset of field: vvdecFrame::bitDepth"]
        [::core::mem::offset_of!(vvdecFrame, bitDepth) - 108usize];
    ["Offset of field: vvdecFrame::frameFormat"]
        [::core::mem::offset_of!(vvdecFrame, frameFormat) - 112usize];
    ["Offset of field: vvdecFrame::colorFormat"]
        [::core::mem::offset_of!(vvdecFrame, colorFormat) - 116usize];
    ["Offset of field: vvdecFrame::sequenceNumber"]
        [::core::mem::offset_of!(vvdecFrame, sequenceNumber) - 120usize];
    ["Offset of field: vvdecFrame::cts"][::core::mem::offset_of!(vvdecFrame, cts) - 128usize];
    ["Offset of field: vvdecFrame::ctsValid"]
        [::core::mem::offset_of!(vvdecFrame, ctsValid) - 136usize];
    ["Offset of field: vvdecFrame::picAttributes"]
        [::core::mem::offset_of!(vvdecFrame, picAttributes) - 144usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vvdecParams {
    pub threads: ::core::ffi::c_int,
    pub parseDelay: ::core::ffi::c_int,
    pub logLevel: vvdecLogLevel,
    pub verifyPictureHash: bool,
    pub filmGrainSynthesis: bool,
    pub simd: vvdecSIMD_Extension,
    pub opaque: *mut ::core::ffi::c_void,
    pub errHandlingFlags: vvdecErrHandlingFlags,
    pub reserved_1: i32,
    pub reserved_2: i32,
    pub reserved_3: i32,
    pub reserved_4: i32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of vvdecParams"][::core::mem::size_of::<vvdecParams>() - 56usize];
    ["Alignment of vvdecParams"][::core::mem::align_of::<vvdecParams>() - 8usize];
    ["Offset of field: vvdecParams::threads"]
        [::core::mem::offset_of!(vvdecParams, threads) - 0usize];
    ["Offset of field: vvdecParams::parseDelay"]
        [::core::mem::offset_of!(vvdecParams, parseDelay) - 4usize];
    ["Offset of field: vvdecParams::logLevel"]
        [::core::mem::offset_of!(vvdecParams, logLevel) - 8usize];
    ["Offset of field: vvdecParams::verifyPictureHash"]
        [::core::mem::offset_of!(vvdecParams, verifyPictureHash) - 12usize];
    ["Offset of field: vvdecParams::filmGrainSynthesis"]
        [::core::mem::offset_of!(vvdecParams, filmGrainSynthesis) - 13usize];
    ["Offset of field: vvdecParams::simd"][::core::mem::offset_of!(vvdecParams, simd) - 16usize];
    ["Offset of field: vvdecParams::opaque"]
        [::core::mem::offset_of!(vvdecParams, opaque) - 24usize];
    ["Offset of field: vvdecParams::errHandlingFlags"]
        [::core::mem::offset_of!(vvdecParams, errHandlingFlags) - 32usize];
    ["Offset of field: vvdecParams::reserved_1"]
        [::core::mem::offset_of!(vvdecParams, reserved_1) - 36usize];
    ["Offset of field: vvdecParams::reserved_2"]
        [::core::mem::offset_of!(vvdecParams, reserved_2) - 40usize];
    ["Offset of field: vvdecParams::reserved_3"]
        [::core::mem::offset_of!(vvdecParams, reserved_3) - 44usize];
    ["Offset of field: vvdecParams::reserved_4"]
        [::core::mem::offset_of!(vvdecParams, reserved_4) - 48usize];
};
unsafe extern "C" {
    pub fn vvdec_params_default(param: *mut vvdecParams);
}
unsafe extern "C" {
    pub fn vvdec_get_version() -> *const ::core::ffi::c_char;
}
unsafe extern "C" {
    pub fn vvdec_decoder_open(arg1: *mut vvdecParams) -> *mut vvdecDecoder;
}
unsafe extern "C" {
    pub fn vvdec_decoder_close(arg1: *mut vvdecDecoder) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    pub fn vvdec_decode(
        arg1: *mut vvdecDecoder,
        accessUnit: *mut vvdecAccessUnit,
        frame: *mut *mut vvdecFrame,
    ) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    pub fn vvdec_flush(arg1: *mut vvdecDecoder, frame: *mut *mut vvdecFrame) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    pub fn vvdec_frame_unref(arg1: *mut vvdecDecoder, frame: *mut vvdecFrame)
        -> ::core::ffi::c_int;
}
unsafe extern "C" {
    pub fn vvdec_get_last_error(arg1: *mut vvdecDecoder) -> *const ::core::ffi::c_char;
}
unsafe extern "C" {
    pub fn vvdec_get_last_additional_error(arg1: *mut vvdecDecoder) -> *const ::core::ffi::c_char;
}
unsafe extern "C" {
    pub fn vvdec_get_error_msg(nRet: ::core::ffi::c_int) -> *const ::core::ffi::c_char;
}
