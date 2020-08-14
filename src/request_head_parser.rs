use std::io::{BufRead, Read};
use std::result;

use http::{HeaderMap, HeaderValue, Method, Uri, Version};

use crate::head_parser::{HeadParseConfig, HeadParseError, HeadParseOutput, HeadParser};

//
//
//
#[derive(Default)]
pub struct RequestHeadParser {
    pub method: Method,
    pub uri: Uri,
    pub http_version: Version,
    pub headers: HeaderMap<HeaderValue>,
    //
    config: HeadParseConfig,
    //
    state: State,
    buf: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    Idle,
    MethodParsed,
    UriParsed,
    HttpVersionParsed,
    HeadersParsing,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

//
//
//
impl HeadParser for RequestHeadParser {
    fn new() -> Self {
        Self::default()
    }
    fn with_config(config: HeadParseConfig) -> Self {
        let buf = Vec::with_capacity(config.buf_capacity());
        let headers = HeaderMap::with_capacity(config.header_map_capacity());
        RequestHeadParser {
            config,
            buf,
            headers,
            ..Default::default()
        }
    }

    fn get_headers(&self) -> &HeaderMap<HeaderValue> {
        &self.headers
    }
    fn get_version(&self) -> &Version {
        &self.http_version
    }

    fn parse<R: BufRead>(&mut self, r: &mut R) -> result::Result<HeadParseOutput, HeadParseError> {
        let mut take = r.take(0);
        let mut parsed_num_bytes = 0_usize;

        if self.state < State::MethodParsed {
            // status_code
            self.buf.clear();
            match Self::parse_method(&mut take, &mut self.buf, &self.config)? {
                Some((method, n)) => {
                    self.state = State::MethodParsed;

                    self.method = method;
                    parsed_num_bytes += n;
                }
                None => return Ok(HeadParseOutput::Partial(parsed_num_bytes)),
            }
        }

        if self.state < State::UriParsed {
            // reason_phrase
            self.buf.clear();
            match Self::parse_uri(&mut take, &mut self.buf, &self.config)? {
                Some((uri, n)) => {
                    self.state = State::UriParsed;

                    self.uri = uri;
                    parsed_num_bytes += n;
                }
                None => return Ok(HeadParseOutput::Partial(parsed_num_bytes)),
            }
        }

        if self.state < State::HttpVersionParsed {
            // http_version
            self.buf.clear();
            match Self::parse_http_version_for_request(&mut take, &mut self.buf)? {
                Some((http_version, n)) => {
                    self.state = State::HttpVersionParsed;

                    self.http_version = http_version;
                    parsed_num_bytes += n;
                }
                None => return Ok(HeadParseOutput::Partial(parsed_num_bytes)),
            }
        }

        // headers
        if self.state < State::HeadersParsing {
            self.headers.clear();
        }
        loop {
            if self.state <= State::HeadersParsing {
                self.buf.clear();
                match Self::parse_header(&mut take, &mut self.buf, &self.config, &mut self.headers)?
                {
                    Some((is_all_completed, n)) => {
                        parsed_num_bytes += n;

                        if is_all_completed {
                            self.state = State::Idle;

                            return Ok(HeadParseOutput::Completed(parsed_num_bytes));
                        } else {
                            self.state = State::HeadersParsing;

                            continue;
                        }
                    }
                    None => return Ok(HeadParseOutput::Partial(parsed_num_bytes)),
                }
            } else {
                unreachable!()
            }
        }
    }
}
