use std::io::{BufRead, Read as _};

use http::{request::Parts as RequestParts, HeaderMap, HeaderValue, Method, Request, Uri, Version};

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

impl RequestHeadParser {
    pub fn to_request_parts(&self) -> RequestParts {
        let (mut parts, _) = Request::new(()).into_parts();
        parts.method = self.method.to_owned();
        parts.uri = self.uri.to_owned();
        parts.version = self.http_version;
        parts.headers = self.headers.to_owned();
        parts
    }

    pub fn to_request<B>(&self, body: B) -> Request<B> {
        let parts = self.to_request_parts();
        Request::from_parts(parts, body)
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

    fn parse<R: BufRead>(&mut self, r: &mut R) -> Result<HeadParseOutput, HeadParseError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_request() {
        let p = RequestHeadParser {
            method: Method::POST,
            uri: Uri::try_from("/path").unwrap(),
            http_version: Version::HTTP_2,
            headers: {
                let mut h = HeaderMap::new();
                h.insert("x-foo", "bar".parse().unwrap());
                h
            },
            ..Default::default()
        };

        let req = p.to_request("body");
        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.uri(), &Uri::try_from("/path").unwrap());
        assert_eq!(req.version(), Version::HTTP_2);
        assert_eq!(req.headers().get("x-foo").unwrap(), "bar");
        assert_eq!(req.body(), &"body");
    }
}
