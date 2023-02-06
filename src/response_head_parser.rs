use std::io::{BufRead, Read as _};

use http::{
    response::Parts as ResponseParts, HeaderMap, HeaderValue, Response, StatusCode, Version,
};

use crate::{
    head_parser::{HeadParseConfig, HeadParseError, HeadParseOutput, HeadParser},
    ReasonPhrase,
};

//
//
//
#[derive(Default)]
pub struct ResponseHeadParser {
    pub http_version: Version,
    pub status_code: StatusCode,
    pub reason_phrase: ReasonPhrase,
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
    HttpVersionParsed,
    StatusCodeParsed,
    ReasonPhraseParsed,
    HeadersParsing,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl ResponseHeadParser {
    pub fn to_response_parts(&self) -> ResponseParts {
        let (mut parts, _) = Response::new(()).into_parts();
        parts.status = self.status_code;
        parts.version = self.http_version;
        parts.headers = self.headers.to_owned();
        parts.extensions.insert(self.reason_phrase.to_owned());
        parts
    }

    pub fn to_response<B>(&self, body: B) -> Response<B> {
        let parts = self.to_response_parts();
        Response::from_parts(parts, body)
    }
}

//
//
//
impl HeadParser for ResponseHeadParser {
    fn new() -> Self {
        Self::default()
    }
    fn with_config(config: HeadParseConfig) -> Self {
        let buf = Vec::with_capacity(config.buf_capacity());
        let headers = HeaderMap::with_capacity(config.header_map_capacity());
        ResponseHeadParser {
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

    fn parse<R: BufRead>(&mut self, r: &mut R) -> Result<HeadParseOutput, HeadParseError> {
        let mut take = r.take(0);
        let mut parsed_num_bytes = 0_usize;

        if self.state < State::HttpVersionParsed {
            // http_version
            self.buf.clear();
            match Self::parse_http_version_for_response(&mut take, &mut self.buf)? {
                Some((http_version, n)) => {
                    self.state = State::HttpVersionParsed;

                    self.http_version = http_version;
                    parsed_num_bytes += n;
                }
                None => return Ok(HeadParseOutput::Partial(parsed_num_bytes)),
            }
        }

        if self.state < State::StatusCodeParsed {
            // status_code
            self.buf.clear();
            match Self::parse_status_code(&mut take, &mut self.buf)? {
                Some((status_code, n)) => {
                    self.state = State::StatusCodeParsed;

                    self.status_code = status_code;
                    parsed_num_bytes += n;
                }
                None => return Ok(HeadParseOutput::Partial(parsed_num_bytes)),
            }
        }

        if self.state < State::ReasonPhraseParsed {
            // reason_phrase
            self.buf.clear();
            match Self::parse_reason_phrase(&mut take, &mut self.buf, &self.config)? {
                Some((reason_phrase, n)) => {
                    self.state = State::ReasonPhraseParsed;

                    self.reason_phrase = reason_phrase;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_response() {
        let p = ResponseHeadParser {
            http_version: Version::HTTP_2,
            status_code: StatusCode::CREATED,
            reason_phrase: Some(b"MyCreated".to_vec()),
            headers: {
                let mut h = HeaderMap::new();
                h.insert("x-foo", "bar".parse().unwrap());
                h
            },
            ..Default::default()
        };

        let res = p.to_response("body");
        assert_eq!(res.version(), Version::HTTP_2);
        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(res.headers().get("x-foo").unwrap(), "bar");
        assert_eq!(
            res.extensions().get::<ReasonPhrase>().unwrap(),
            &Some(b"MyCreated".to_vec())
        );
        assert_eq!(res.body(), &"body");
    }
}
