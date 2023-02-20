use core::{
    cmp::min,
    ops::{Deref, DerefMut},
    time::Duration,
};
use std::io::{BufReader, Error as IoError, ErrorKind as IoErrorKind};

use async_sleep::{rw::AsyncReadWithTimeoutExt as _, Sleepble};
use async_trait::async_trait;
use futures_util::AsyncRead;
use http::{Request, Response, Version};
use http1_spec::{
    body_framing::{BodyFraming, BodyFramingDetector},
    body_parser::{BodyParseOutput, BodyParser},
    content_length_body_parser::ContentLengthBodyParser,
    head_parser::{HeadParseConfig, HeadParseOutput, HeadParser},
    request_head_parser::RequestHeadParser,
    response_head_parser::ResponseHeadParser,
    ReasonPhrase,
};

use crate::{body::DecoderBody, stream::Http1StreamDecoder};

//
//
//
pub struct Http1Decoder<HP>
where
    HP: HeadParser,
{
    head_parser: HP,
    content_length_body_parser: ContentLengthBodyParser,
    buf: Vec<u8>,
    offset_read: usize,
    offset_parsed: usize,
    read_timeout: Duration,
    state: State,
    require_read: bool,
}
#[derive(Debug, PartialEq, Eq)]
enum State {
    Idle,
    ReadingHead,
    ReadBody(BodyFraming),
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}
impl<HP> Http1Decoder<HP>
where
    HP: HeadParser,
{
    //
    fn new(buf_capacity: usize, config: Option<HeadParseConfig>) -> Self {
        Self {
            head_parser: HP::with_config(config.unwrap_or_default()),
            content_length_body_parser: ContentLengthBodyParser::new(),
            buf: vec![0u8; buf_capacity],
            offset_read: 0,
            offset_parsed: 0,
            read_timeout: Duration::from_secs(5),
            state: Default::default(),
            require_read: true,
        }
    }

    //
    fn set_read_timeout(&mut self, dur: Duration) {
        self.read_timeout = dur;
    }
    pub fn has_unparsed_bytes(&self) -> bool {
        self.offset_read > self.offset_parsed
    }

    //
    async fn read<S: AsyncRead + Unpin, SLEEP: Sleepble>(
        &mut self,
        stream: &mut S,
    ) -> Result<(), IoError> {
        if !self.require_read {
            return Ok(());
        }

        //
        if self.offset_read >= self.buf.len() {
            return Err(IoError::new(IoErrorKind::InvalidInput, "override buf"));
        }

        //
        let n_read = match stream
            .read_with_timeout::<SLEEP>(&mut self.buf[self.offset_read..], self.read_timeout)
            .await
        {
            Ok(n) if n == 0 => return Err(IoError::new(IoErrorKind::UnexpectedEof, "read 0")),
            Ok(n) => n,
            Err(err) => return Err(err),
        };
        self.offset_read += n_read;
        Ok(())
    }

    fn rotate_offset(&mut self) {
        let n = self.offset_parsed;
        self.buf.rotate_left(n);
        self.offset_read -= n;
        self.offset_parsed = 0;
    }

    async fn read_head0<S: AsyncRead + Unpin, SLEEP: Sleepble>(
        &mut self,
        stream: &mut S,
    ) -> Result<BodyFraming, IoError> {
        if self.state == State::Idle {
            self.rotate_offset();
        }

        let body_framing = loop {
            self.read::<_, SLEEP>(stream).await?;

            let mut buf_reader = BufReader::new(&self.buf[self.offset_parsed..self.offset_read]);

            match self.head_parser.parse(&mut buf_reader) {
                Ok(HeadParseOutput::Completed(n_parsed)) => {
                    self.offset_parsed += n_parsed;
                    if self.offset_parsed == self.offset_read {
                        self.require_read = true;
                    } else {
                        self.require_read = false;
                    }

                    let headers = self.head_parser.get_headers();
                    let version = self.head_parser.get_version();

                    let body_framing = (headers, version).detect()?;
                    match &body_framing {
                        BodyFraming::Neither => {
                            self.state = State::Idle;
                        }
                        BodyFraming::ContentLength(n) => {
                            if n == &0 {
                                self.state = State::Idle;
                            } else {
                                self.state = State::ReadBody(body_framing.clone());
                            }
                        }
                        BodyFraming::Chunked => {
                            if version != &Version::HTTP_11 {
                                return Err(IoError::new(
                                    IoErrorKind::InvalidInput,
                                    "Only valid in HTTP/1.1",
                                ));
                            }
                            return Err(IoError::new(
                                IoErrorKind::InvalidInput,
                                "unimplemented now",
                            ));
                        }
                    }

                    break body_framing;
                }
                Ok(HeadParseOutput::Partial(n_parsed)) => {
                    self.offset_parsed += n_parsed;
                    self.require_read = true;

                    self.state = State::ReadingHead;

                    continue;
                }
                Err(err) => return Err(err.into()),
            }
        };

        Ok(body_framing)
    }

    async fn read_body0<S: AsyncRead + Unpin, SLEEP: Sleepble>(
        &mut self,
        stream: &mut S,
    ) -> Result<DecoderBody, IoError> {
        #[allow(clippy::single_match)]
        match self.state {
            State::ReadBody(_) => {
                self.read::<_, SLEEP>(stream).await?;
            }
            _ => {}
        }

        match &mut self.state {
            State::Idle => Ok(DecoderBody::Completed(Vec::<u8>::new())),
            State::ReadingHead => Err(IoError::new(IoErrorKind::Other, "state should is ReadBody")),
            State::ReadBody(body_framing) => match body_framing.clone() {
                BodyFraming::Neither => unreachable!(),
                BodyFraming::ContentLength(content_length) => {
                    debug_assert!(content_length > 0);

                    self.content_length_body_parser.set_length(content_length);
                    let mut buf_reader =
                        BufReader::new(&self.buf[self.offset_parsed..self.offset_read]);
                    let mut body_buf =
                        vec![0u8; min(self.offset_read - self.offset_parsed, content_length)];
                    match self
                        .content_length_body_parser
                        .parse(&mut buf_reader, &mut body_buf)
                    {
                        Ok(BodyParseOutput::Completed(n_parsed)) => {
                            self.offset_parsed += n_parsed;
                            if self.offset_parsed == self.offset_read {
                                self.require_read = true;
                            } else {
                                self.require_read = false;
                            }

                            self.state = State::Idle;

                            Ok(DecoderBody::Completed(body_buf))
                        }
                        Ok(BodyParseOutput::Partial(n_parsed)) => {
                            self.offset_parsed += n_parsed;
                            if self.offset_parsed == self.offset_read {
                                self.require_read = true;
                            } else {
                                self.require_read = false;
                            }

                            body_framing.update_content_length_value(content_length - n_parsed)?;

                            Ok(DecoderBody::Partial(body_buf))
                        }
                        Err(err) => Err(err.into()),
                    }
                }
                BodyFraming::Chunked => {
                    Err(IoError::new(IoErrorKind::InvalidInput, "unimplemented now"))
                }
            },
        }
    }
}

//
//
//
pub type Http1RequestDecoderInner = Http1Decoder<RequestHeadParser>;
pub struct Http1RequestDecoder {
    inner: Http1RequestDecoderInner,
}
impl Deref for Http1RequestDecoder {
    type Target = Http1RequestDecoderInner;

    fn deref(&self) -> &Http1RequestDecoderInner {
        &self.inner
    }
}
impl DerefMut for Http1RequestDecoder {
    fn deref_mut(&mut self) -> &mut Http1RequestDecoderInner {
        &mut self.inner
    }
}
impl Http1RequestDecoder {
    pub fn new(buf_capacity: usize, config: Option<HeadParseConfig>) -> Self {
        Self {
            inner: Http1RequestDecoderInner::new(buf_capacity, config),
        }
    }
}

#[async_trait]
impl<S, SLEEP> Http1StreamDecoder<S, SLEEP, Request<()>> for Http1RequestDecoder
where
    S: AsyncRead + Unpin + Send,
    SLEEP: Sleepble,
{
    async fn read_head(&mut self, stream: &mut S) -> Result<(Request<()>, BodyFraming), IoError> {
        let body_framing = self.read_head0::<_, SLEEP>(stream).await?;

        let mut request = Request::new(());
        *request.method_mut() = self.inner.head_parser.method.to_owned();
        *request.uri_mut() = self.inner.head_parser.uri.to_owned();
        *request.version_mut() = self.inner.head_parser.http_version.to_owned();
        *request.headers_mut() = self.inner.head_parser.headers.to_owned();

        Ok((request, body_framing))
    }
    async fn read_body(&mut self, stream: &mut S) -> Result<DecoderBody, IoError> {
        self.read_body0::<_, SLEEP>(stream).await
    }

    fn set_read_timeout(&mut self, dur: Duration) {
        self.inner.set_read_timeout(dur)
    }
}

//
//
//
pub type Http1ResponseDecoderInner = Http1Decoder<ResponseHeadParser>;
pub struct Http1ResponseDecoder {
    inner: Http1ResponseDecoderInner,
}
impl Deref for Http1ResponseDecoder {
    type Target = Http1ResponseDecoderInner;

    fn deref(&self) -> &Http1ResponseDecoderInner {
        &self.inner
    }
}
impl DerefMut for Http1ResponseDecoder {
    fn deref_mut(&mut self) -> &mut Http1ResponseDecoderInner {
        &mut self.inner
    }
}
impl Http1ResponseDecoder {
    pub fn new(buf_capacity: usize, config: Option<HeadParseConfig>) -> Self {
        Self {
            inner: Http1ResponseDecoderInner::new(buf_capacity, config),
        }
    }
}

#[async_trait]
impl<S, SLEEP> Http1StreamDecoder<S, SLEEP, (Response<()>, ReasonPhrase)> for Http1ResponseDecoder
where
    S: AsyncRead + Unpin + Send,
    SLEEP: Sleepble,
{
    async fn read_head(
        &mut self,
        stream: &mut S,
    ) -> Result<((Response<()>, ReasonPhrase), BodyFraming), IoError> {
        let body_framing = self.read_head0::<_, SLEEP>(stream).await?;

        let mut response = Response::new(());
        *response.version_mut() = self.inner.head_parser.http_version.to_owned();
        *response.status_mut() = self.inner.head_parser.status_code.to_owned();
        *response.headers_mut() = self.inner.head_parser.headers.to_owned();

        let reason_phrase = self.inner.head_parser.reason_phrase.to_owned();

        Ok(((response, reason_phrase), body_framing))
    }
    async fn read_body(&mut self, stream: &mut S) -> Result<DecoderBody, IoError> {
        self.read_body0::<_, SLEEP>(stream).await
    }

    fn set_read_timeout(&mut self, dur: Duration) {
        self.inner.set_read_timeout(dur)
    }
}
