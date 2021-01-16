use std::{
    error, fmt,
    io::{self, BufRead},
    num,
};

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
impl error::Error for BodyParseError {}
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
    ) -> Result<BodyParseOutput, BodyParseError>;
}
