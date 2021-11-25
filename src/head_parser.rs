use std::{
    cmp,
    convert::TryInto,
    error, fmt,
    io::{self, BufRead, Take},
};

use http::{
    header::{HeaderName, InvalidHeaderName, InvalidHeaderValue},
    method::InvalidMethod,
    status::InvalidStatusCode,
    uri::InvalidUri,
    HeaderMap, HeaderValue, Method, StatusCode, Uri, Version,
};

use crate::{ReasonPhrase, COLON, CR, HTTP_VERSION_10, HTTP_VERSION_11, LF, SP};

//
//
//
const HTTP_VERSION_LEN: usize = 8;
const STATUS_CODE_LEN: usize = 3;

const HEADERS_MAX_LEN: usize = 8192;
const URI_MAX_LEN: usize = 2048;

pub type IsAllCompleted = bool;

//
//
//
#[derive(Debug, Clone)]
pub struct HeadParseConfig {
    header_max_len: usize,
    headers_max_len: usize,
    // res
    reason_phrase_max_len: usize,
    // req
    method_max_len: usize,
    uri_max_len: usize,
}
impl Default for HeadParseConfig {
    fn default() -> Self {
        HeadParseConfig {
            header_max_len: 32 + 448,
            headers_max_len: 4096,
            // res
            reason_phrase_max_len: 40,
            // req
            method_max_len: 8,
            uri_max_len: 512,
        }
    }
}
impl HeadParseConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn buf_capacity(&self) -> usize {
        self.get_header_max_len()
    }
    pub fn header_map_capacity(&self) -> usize {
        cmp::min(self.get_header_max_len() * 6, self.get_headers_max_len())
    }

    pub fn set_header_max_len(&mut self, value: u16) -> &mut Self {
        self.header_max_len = value as usize;
        self
    }
    pub fn get_header_max_len(&self) -> usize {
        self.header_max_len
    }
    pub fn set_headers_max_len(&mut self, value: u16) -> &mut Self {
        self.headers_max_len = cmp::min(value, HEADERS_MAX_LEN as u16) as usize;
        self
    }
    pub fn get_headers_max_len(&self) -> usize {
        self.headers_max_len
    }
    // res
    pub fn set_reason_phrase_max_len(&mut self, value: u8) -> &mut Self {
        self.reason_phrase_max_len = value as usize;
        self
    }
    pub fn get_reason_phrase_max_len(&self) -> usize {
        self.reason_phrase_max_len
    }
    // req
    pub fn set_method_max_len(&mut self, value: u8) -> &mut Self {
        self.method_max_len = value as usize;
        self
    }
    pub fn get_method_max_len(&self) -> usize {
        self.method_max_len
    }
    pub fn set_uri_max_len(&mut self, value: u16) -> &mut Self {
        self.uri_max_len = cmp::min(value, URI_MAX_LEN as u16) as usize;
        self
    }
    pub fn get_uri_max_len(&self) -> usize {
        self.uri_max_len
    }
}

//
//
//
#[derive(Debug, PartialEq, Eq)]
pub enum HeadParseOutput {
    Completed(usize),
    Partial(usize),
}

#[derive(Debug)]
pub enum HeadParseError {
    ReadError(io::Error),
    TooLongHttpVersion,
    InvalidHttpVersion,
    TooLongHeader,
    InvalidHeader,
    InvalidHeaderName(InvalidHeaderName),
    InvalidHeaderValue(InvalidHeaderValue),
    TooLongHeaders,
    InvalidCRLF,
    // res
    TooLongStatusCode,
    InvalidStatusCode(InvalidStatusCode),
    TooLongReasonPhrase,
    // req
    TooLongMethod,
    InvalidMethod(InvalidMethod),
    TooLongUri,
    InvalidUri(InvalidUri),
}
impl fmt::Display for HeadParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl error::Error for HeadParseError {}
impl From<HeadParseError> for io::Error {
    fn from(err: HeadParseError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, err.to_string())
    }
}

//
//
//
pub trait HeadParser {
    fn new() -> Self;
    fn with_config(config: HeadParseConfig) -> Self;

    fn get_headers(&self) -> &HeaderMap<HeaderValue>;
    fn get_version(&self) -> &Version;

    fn parse<R: BufRead>(&mut self, r: &mut R) -> Result<HeadParseOutput, HeadParseError>;

    fn parse_header<R: BufRead>(
        take: &mut Take<R>,
        buf: &mut Vec<u8>,
        config: &HeadParseConfig,
        headers: &mut HeaderMap<HeaderValue>,
    ) -> Result<Option<(IsAllCompleted, usize)>, HeadParseError> {
        let end_bytes_len = 2_usize;
        take.set_limit(config.get_header_max_len() as u64 + end_bytes_len as u64);
        let n = take
            .read_until(LF, buf)
            .map_err(HeadParseError::ReadError)?;
        if n < end_bytes_len {
            return Ok(None);
        }
        if !buf[..n].ends_with(&[LF]) {
            if n >= config.get_header_max_len() {
                return Err(HeadParseError::TooLongHeader);
            } else {
                return Ok(None);
            }
        }
        if !buf[..n - 1].ends_with(&[CR]) {
            return Err(HeadParseError::InvalidCRLF);
        }

        // TODO, valid HEADERS_MAX_LEN

        //
        if buf[..n - end_bytes_len].is_empty() {
            return Ok(Some((true, n)));
        }
        let header_colon_index = buf[..n - end_bytes_len]
            .iter()
            .position(|x| x == &COLON)
            .ok_or(HeadParseError::InvalidHeader)?;
        let header_name = &buf[..header_colon_index];
        let header_value = &buf[header_colon_index + 1..n - end_bytes_len];
        let mut n_left_whitespace = 0_usize;
        if header_value[0] == SP {
            n_left_whitespace += 1;
        }

        let header_name =
            HeaderName::from_bytes(header_name).map_err(HeadParseError::InvalidHeaderName)?;
        let header_value = HeaderValue::from_bytes(&header_value[n_left_whitespace..])
            .map_err(HeadParseError::InvalidHeaderValue)?;

        headers.insert(header_name, header_value);
        Ok(Some((false, n)))
    }

    //
    // res
    //
    fn parse_http_version_for_response<R: BufRead>(
        take: &mut Take<R>,
        buf: &mut Vec<u8>,
    ) -> Result<Option<(Version, usize)>, HeadParseError> {
        let end_bytes_len = 1_usize;
        take.set_limit(HTTP_VERSION_LEN as u64 + end_bytes_len as u64);
        let n = take
            .read_until(SP, buf)
            .map_err(HeadParseError::ReadError)?;
        if n < end_bytes_len {
            return Ok(None);
        }
        if !buf[..n].ends_with(&[SP]) {
            if n >= HTTP_VERSION_LEN {
                return Err(HeadParseError::TooLongHttpVersion);
            } else {
                return Ok(None);
            }
        }
        let http_version = match &buf[..n - end_bytes_len] {
            HTTP_VERSION_10 => Version::HTTP_10,
            HTTP_VERSION_11 => Version::HTTP_11,
            _ => return Err(HeadParseError::InvalidHttpVersion),
        };
        Ok(Some((http_version, n)))
    }

    fn parse_status_code<R: BufRead>(
        take: &mut Take<R>,
        buf: &mut Vec<u8>,
    ) -> Result<Option<(StatusCode, usize)>, HeadParseError> {
        let end_bytes_len = 1_usize;
        take.set_limit(STATUS_CODE_LEN as u64 + end_bytes_len as u64);
        let n = take
            .read_until(SP, buf)
            .map_err(HeadParseError::ReadError)?;
        if n < end_bytes_len {
            return Ok(None);
        }
        if !buf[..n].ends_with(&[SP]) {
            if n >= STATUS_CODE_LEN {
                return Err(HeadParseError::TooLongStatusCode);
            } else {
                return Ok(None);
            }
        }
        let status_code = StatusCode::from_bytes(&buf[..n - end_bytes_len])
            .map_err(HeadParseError::InvalidStatusCode)?;

        Ok(Some((status_code, n)))
    }

    fn parse_reason_phrase<R: BufRead>(
        take: &mut Take<R>,
        buf: &mut Vec<u8>,
        config: &HeadParseConfig,
    ) -> Result<Option<(ReasonPhrase, usize)>, HeadParseError> {
        let end_bytes_len = 2_usize;
        take.set_limit(config.get_reason_phrase_max_len() as u64 + end_bytes_len as u64);
        let n = take
            .read_until(LF, buf)
            .map_err(HeadParseError::ReadError)?;
        if n < end_bytes_len {
            return Ok(None);
        }
        if !buf[..n].ends_with(&[LF]) {
            if n >= config.get_reason_phrase_max_len() {
                return Err(HeadParseError::TooLongReasonPhrase);
            } else {
                return Ok(None);
            }
        }
        if !buf[..n - 1].ends_with(&[CR]) {
            return Err(HeadParseError::InvalidCRLF);
        }
        let reason_phrase: ReasonPhrase = if buf[..n - end_bytes_len].is_empty() {
            None
        } else {
            Some(buf[..n - end_bytes_len].to_vec())
        };

        Ok(Some((reason_phrase, n)))
    }

    //
    // req
    //
    fn parse_method<R: BufRead>(
        take: &mut Take<R>,
        buf: &mut Vec<u8>,
        config: &HeadParseConfig,
    ) -> Result<Option<(Method, usize)>, HeadParseError> {
        let end_bytes_len = 1_usize;
        take.set_limit(config.get_method_max_len() as u64 + end_bytes_len as u64);
        let n = take
            .read_until(SP, buf)
            .map_err(HeadParseError::ReadError)?;
        if n < end_bytes_len {
            return Ok(None);
        }
        if !buf[..n].ends_with(&[SP]) {
            if n >= config.get_method_max_len() {
                return Err(HeadParseError::TooLongMethod);
            } else {
                return Ok(None);
            }
        }
        let method =
            Method::from_bytes(&buf[..n - end_bytes_len]).map_err(HeadParseError::InvalidMethod)?;

        Ok(Some((method, n)))
    }

    fn parse_uri<R: BufRead>(
        take: &mut Take<R>,
        buf: &mut Vec<u8>,
        config: &HeadParseConfig,
    ) -> Result<Option<(Uri, usize)>, HeadParseError> {
        let end_bytes_len = 1_usize;
        take.set_limit(config.get_uri_max_len() as u64 + end_bytes_len as u64);
        let n = take
            .read_until(SP, buf)
            .map_err(HeadParseError::ReadError)?;
        if n < end_bytes_len {
            return Ok(None);
        }
        if !buf[..n].ends_with(&[SP]) {
            if n >= config.get_uri_max_len() {
                return Err(HeadParseError::TooLongUri);
            } else {
                return Ok(None);
            }
        }
        let uri = (&buf[..n - end_bytes_len])
            .try_into()
            .map_err(HeadParseError::InvalidUri)?;

        Ok(Some((uri, n)))
    }

    fn parse_http_version_for_request<R: BufRead>(
        take: &mut Take<R>,
        buf: &mut Vec<u8>,
    ) -> Result<Option<(Version, usize)>, HeadParseError> {
        let end_bytes_len = 2_usize;
        take.set_limit(HTTP_VERSION_LEN as u64 + end_bytes_len as u64);
        let n = take
            .read_until(LF, buf)
            .map_err(HeadParseError::ReadError)?;
        if n < end_bytes_len {
            return Ok(None);
        }
        if !buf[..n].ends_with(&[LF]) {
            if n >= HTTP_VERSION_LEN {
                return Err(HeadParseError::TooLongHttpVersion);
            } else {
                return Ok(None);
            }
        }
        if !buf[..n - 1].ends_with(&[CR]) {
            return Err(HeadParseError::InvalidCRLF);
        }
        let http_version = match &buf[..n - end_bytes_len] {
            HTTP_VERSION_10 => Version::HTTP_10,
            HTTP_VERSION_11 => Version::HTTP_11,
            _ => return Err(HeadParseError::InvalidHttpVersion),
        };
        Ok(Some((http_version, n)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        error::Error,
        io::{BufReader, Cursor, Read},
    };

    use crate::request_head_parser::RequestHeadParser;

    #[test]
    fn parse_header_with_multi_colon() -> Result<(), Box<dyn Error>> {
        let mut take = BufReader::new(Cursor::new(b"Foo: Bar:Bar\r\n")).take(0);
        let mut buf = Vec::new();
        let mut headers = HeaderMap::new();

        RequestHeadParser::parse_header(
            &mut take,
            &mut buf,
            &HeadParseConfig::default(),
            &mut headers,
        )?;

        match headers.get("Foo") {
            Some(header_value) => {
                assert_eq!(header_value, "Bar:Bar");
            }
            None => panic!(),
        }

        Ok(())
    }
}
