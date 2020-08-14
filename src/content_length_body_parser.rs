use std::io::{BufRead, Read};
use std::result;

use crate::body_parser::{BodyParseError, BodyParseOutput, BodyParser};

//
//
//
#[derive(Default)]
pub struct ContentLengthBodyParser {
    length: usize,
}
impl ContentLengthBodyParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_length(&mut self, length: usize) {
        self.length = length
    }
    pub fn get_length(&self) -> usize {
        self.length
    }
}

//
//
//
impl BodyParser for ContentLengthBodyParser {
    fn parse<R: BufRead>(
        &mut self,
        r: &mut R,
        body_buf: &mut Vec<u8>,
    ) -> result::Result<BodyParseOutput, BodyParseError> {
        let mut take = r.take(self.length as u64);

        let n = take
            .read(body_buf)
            .map_err(|err| BodyParseError::ReadError(err))?;
        self.length -= n;

        if self.length == 0 {
            Ok(BodyParseOutput::Completed(n))
        } else {
            Ok(BodyParseOutput::Partial(n))
        }
    }
}
