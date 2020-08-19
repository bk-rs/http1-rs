use std::fmt;
use std::io::{self, BufRead};
use std::num;
use std::result;

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
    ReadError(io::Error),
    TooLongChunksOfLength,
    InvalidChunksOfLength(Option<num::ParseIntError>),
    TooLongChunksOfCRLF,
    InvalidCRLF,
}
impl fmt::Display for BodyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl From<BodyParseError> for io::Error {
    fn from(err: BodyParseError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, err.to_string())
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
    ) -> result::Result<BodyParseOutput, BodyParseError>;
}
