use core::num::ParseIntError;
use std::io::{BufRead, Error as IoError, ErrorKind as IoErrorKind};

//
//
//
#[derive(Debug, PartialEq, Eq)]
pub enum BodyParseOutput {
    Completed(usize),
    Partial(usize),
}

#[derive(Debug)]
pub enum BodyParseError {
    ReadError(IoError),
    TooLongChunksOfLength,
    InvalidChunksOfLength(Option<ParseIntError>),
    TooLongChunksOfCRLF,
    InvalidCRLF,
}
impl core::fmt::Display for BodyParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BodyParseError {}
impl From<BodyParseError> for IoError {
    fn from(err: BodyParseError) -> IoError {
        IoError::new(IoErrorKind::InvalidInput, err.to_string())
    }
}

//
//
//
pub trait BodyParser {
    fn parse<R: BufRead>(
        &mut self,
        r: &mut R,
        body_buf: &mut Vec<u8>,
    ) -> Result<BodyParseOutput, BodyParseError>;
}
