//! VC-3/DNxHD parser error type.

#[derive(Debug, thiserror::Error)]
pub enum Vc3Error {
    #[error("unexpected end of data at offset {0}")]
    UnexpectedEof(usize),

    #[error("invalid DNxHD header magic at offset {0:#x}")]
    InvalidMagic(usize),

    #[error("unsupported compression ID {0}")]
    UnsupportedCompId(u32),

    #[error("data error: {0}")]
    Data(&'static str),
}

pub type Result<T> = std::result::Result<T, Vc3Error>;
