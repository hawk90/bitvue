//! JPEG XS parser error type.

#[derive(Debug, thiserror::Error)]
pub enum JpegXsError {
    #[error("unexpected end of data at offset {0}")]
    UnexpectedEof(usize),

    #[error("invalid marker 0x{0:04X} at offset {1}")]
    InvalidMarker(u16, usize),

    #[error("missing required marker: {0}")]
    MissingMarker(&'static str),

    #[error("unsupported profile/level: {0}")]
    UnsupportedProfile(u8),

    #[error("data error: {0}")]
    Data(&'static str),
}

pub type Result<T> = std::result::Result<T, JpegXsError>;
