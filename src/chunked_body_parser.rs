use std::{
    io::{BufRead, Read},
    str,
};

use crate::body_parser::{BodyParseError, BodyParseOutput, BodyParser};
use crate::{CR, CRLF, LF};

//
//
//
const LENGTH_MAX_LEN: usize = 4; // b"FFFF"
const DATA_DEFAULT_LEN: usize = 512;

//
//
//
#[derive(Default)]
pub struct ChunkedBodyParser {
    //
    state: State,
    length_buf: Vec<u8>,
    length: u16,
    data_buf: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    Idle,
    WaitLengthParse,
    WaitDataParse,
    WaitDataParsing,
    WaitCRLFParse(ActionAfterCRLFParsed),
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ActionAfterCRLFParsed {
    Continue,
    Break,
}

impl ChunkedBodyParser {
    pub fn new() -> Self {
        Self {
            length_buf: Vec::with_capacity(LENGTH_MAX_LEN),
            data_buf: vec![0u8; DATA_DEFAULT_LEN],
            ..Default::default()
        }
    }

    pub fn with_data_buf(data_buf: Vec<u8>) -> Self {
        Self {
            length_buf: Vec::with_capacity(LENGTH_MAX_LEN),
            data_buf,
            ..Default::default()
        }
    }
}

//
//
//
impl BodyParser for ChunkedBodyParser {
    fn parse<R: BufRead>(
        &mut self,
        r: &mut R,
        body_buf: &mut Vec<u8>,
    ) -> Result<BodyParseOutput, BodyParseError> {
        let mut take = r.take(0);
        let mut parsed_num_bytes = 0_usize;

        loop {
            if self.state <= State::WaitLengthParse {
                let end_bytes_len = 2_usize;
                take.set_limit(LENGTH_MAX_LEN as u64 + end_bytes_len as u64);

                self.length_buf.clear();
                let n = take
                    .read_until(LF, &mut self.length_buf)
                    .map_err(BodyParseError::ReadError)?;

                if n < end_bytes_len {
                    return Ok(BodyParseOutput::Partial(parsed_num_bytes));
                }
                if !self.length_buf[..n].ends_with(&[LF]) {
                    if n >= LENGTH_MAX_LEN {
                        return Err(BodyParseError::TooLongChunksOfLength);
                    } else {
                        return Ok(BodyParseOutput::Partial(parsed_num_bytes));
                    }
                }
                if !self.length_buf[..n - 1].ends_with(&[CR]) {
                    return Err(BodyParseError::InvalidCRLF);
                }
                let length_bytes = &self.length_buf[..n - end_bytes_len];
                let length_str = str::from_utf8(length_bytes)
                    .map_err(|_| BodyParseError::InvalidChunksOfLength(None))?;
                let length = u16::from_str_radix(length_str, 16)
                    .map_err(|err| BodyParseError::InvalidChunksOfLength(Some(err)))?;

                self.length = length;
                parsed_num_bytes += n;

                if length == 0 {
                    self.state = State::WaitCRLFParse(ActionAfterCRLFParsed::Break);
                } else {
                    self.state = State::WaitDataParse;
                }
            }

            if self.state <= State::WaitDataParsing {
                take.set_limit(self.length as u64);

                let n = take
                    .read(&mut self.data_buf)
                    .map_err(BodyParseError::ReadError)?;
                body_buf.extend_from_slice(&self.data_buf[..n]);

                self.length -= n as u16;
                parsed_num_bytes += n;

                if self.length == 0 {
                    self.state = State::WaitCRLFParse(ActionAfterCRLFParsed::Continue);
                } else {
                    self.state = State::WaitDataParsing;

                    return Ok(BodyParseOutput::Partial(parsed_num_bytes));
                }
            }

            if let State::WaitCRLFParse(action) = &self.state {
                let end_bytes_len = 2_usize;
                take.set_limit(end_bytes_len as u64);

                self.length_buf.clear();
                let n = take
                    .read_until(LF, &mut self.length_buf)
                    .map_err(BodyParseError::ReadError)?;
                if n < end_bytes_len {
                    return Ok(BodyParseOutput::Partial(parsed_num_bytes));
                }
                if &self.length_buf[..n] != CRLF {
                    return Err(BodyParseError::InvalidCRLF);
                }
                parsed_num_bytes += n;

                match action {
                    ActionAfterCRLFParsed::Continue => {
                        self.state = State::WaitLengthParse;

                        continue;
                    }
                    ActionAfterCRLFParsed::Break => {
                        self.state = State::Idle;

                        break Ok(BodyParseOutput::Completed(parsed_num_bytes));
                    }
                }
            }

            unreachable!()
        }
    }
}
