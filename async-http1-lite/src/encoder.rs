use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    time::Duration,
};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use async_sleep::{rw::AsyncWriteWithTimeoutExt as _, Sleepble};
use async_trait::async_trait;
use futures_util::AsyncWrite;
use http::{
    header::{CONTENT_LENGTH, TRANSFER_ENCODING},
    request::Parts as RequestParts,
    response::Parts as ResponseParts,
    HeaderMap, HeaderValue, Request, Response, Version,
};
use http1_spec::{
    body_framing::BodyFraming,
    head_renderer::{Head, HeadRenderer},
    request_head_renderer::RequestHeadRenderer,
    response_head_renderer::ResponseHeadRenderer,
    ReasonPhrase, CHUNKED,
};

use crate::{body::EncoderBody, stream::Http1StreamEncoder};

//
//
//
pub struct Http1Encoder<H, HR>
where
    H: Head,
    HR: HeadRenderer<H>,
{
    head_renderer: HR,
    buf: Vec<u8>,
    write_timeout: Duration,
    state: State,
    phantom: PhantomData<H>,
}
#[derive(Debug, PartialEq, Eq)]
enum State {
    Idle,
    WriteBody(BodyFraming),
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}
impl<H, HR> Http1Encoder<H, HR>
where
    H: Head,
    HR: HeadRenderer<H>,
{
    //
    fn new(buf_capacity: usize) -> Self {
        Self {
            head_renderer: HR::new(),
            buf: Vec::with_capacity(buf_capacity),
            write_timeout: Duration::from_secs(5),
            state: Default::default(),
            phantom: PhantomData,
        }
    }

    //
    fn set_write_timeout(&mut self, dur: Duration) {
        self.write_timeout = dur;
    }

    //
    fn update_headers(
        &self,
        headers: &mut HeaderMap<HeaderValue>,
        version: &Version,
        body_framing: &BodyFraming,
    ) -> Result<(), IoError> {
        match body_framing {
            BodyFraming::Neither => {
                headers.remove(CONTENT_LENGTH);
                headers.remove(TRANSFER_ENCODING);
            }
            BodyFraming::ContentLength(n) => {
                if n == &0 {
                    headers.remove(CONTENT_LENGTH);
                    headers.remove(TRANSFER_ENCODING);
                } else {
                    headers.insert(
                        CONTENT_LENGTH,
                        HeaderValue::from_str(&format!("{n}"))
                            .map_err(|err| IoError::new(IoErrorKind::Other, err))?,
                    );
                    if version == &Version::HTTP_11 {
                        if let Some(header_value) = headers.get(&TRANSFER_ENCODING) {
                            if header_value == CHUNKED {
                                headers.remove(TRANSFER_ENCODING);
                            }
                        }
                    }
                }
            }
            BodyFraming::Chunked => {
                if version != &Version::HTTP_11 {
                    return Err(IoError::new(IoErrorKind::InvalidInput, "unimplemented now"));
                }
                headers.remove(CONTENT_LENGTH);
                headers.insert(
                    TRANSFER_ENCODING,
                    HeaderValue::from_str(CHUNKED)
                        .map_err(|err| IoError::new(IoErrorKind::Other, err))?,
                );
            }
        }

        Ok(())
    }

    fn encode_head(&mut self, head: H) -> Result<(), IoError> {
        self.head_renderer.render(head, &mut self.buf)
    }

    async fn write_head0<S: AsyncWrite + Unpin, SLEEP: Sleepble>(
        &self,
        stream: &mut S,
    ) -> Result<usize, IoError> {
        stream
            .write_with_timeout::<SLEEP>(&self.buf, self.write_timeout)
            .await
    }

    async fn write_body0<S: AsyncWrite + Unpin, SLEEP: Sleepble>(
        &mut self,
        stream: &mut S,
        body: EncoderBody,
    ) -> Result<(), IoError> {
        match &mut self.state {
            State::Idle => {
                return Err(IoError::new(
                    IoErrorKind::Other,
                    "state should is WriteBody",
                ));
            }
            State::WriteBody(body_framing) => match body_framing.clone() {
                BodyFraming::Neither => {}
                BodyFraming::ContentLength(content_length) => {
                    if content_length == 0 {
                        return Ok(());
                    }

                    let bytes = match &body {
                        EncoderBody::Completed(bytes) => {
                            if bytes.len() != content_length {
                                return Err(IoError::new(
                                    IoErrorKind::InvalidInput,
                                    "bytes len mismatch",
                                ));
                            }
                            bytes
                        }
                        EncoderBody::Partial(bytes) => {
                            if bytes.len() >= content_length {
                                return Err(IoError::new(
                                    IoErrorKind::InvalidInput,
                                    "bytes len mismatch",
                                ));
                            }
                            bytes
                        }
                    };

                    let bytes_len = bytes.len();
                    stream
                        .write_with_timeout::<SLEEP>(bytes, self.write_timeout)
                        .await?;

                    match &body {
                        EncoderBody::Completed(_) => {
                            self.state = State::Idle;
                        }
                        EncoderBody::Partial(_) => {
                            body_framing.update_content_length_value(content_length - bytes_len)?;
                        }
                    };
                }
                BodyFraming::Chunked => {
                    return Err(IoError::new(IoErrorKind::InvalidInput, "unimplemented now"))
                }
            },
        }

        Ok(())
    }
}

//
//
//
pub type Http1RequestEncoderInner = Http1Encoder<RequestParts, RequestHeadRenderer>;
pub struct Http1RequestEncoder {
    inner: Http1RequestEncoderInner,
}
impl Deref for Http1RequestEncoder {
    type Target = Http1RequestEncoderInner;

    fn deref(&self) -> &Http1RequestEncoderInner {
        &self.inner
    }
}
impl DerefMut for Http1RequestEncoder {
    fn deref_mut(&mut self) -> &mut Http1RequestEncoderInner {
        &mut self.inner
    }
}
impl Http1RequestEncoder {
    pub fn new(buf_capacity: usize) -> Self {
        Self {
            inner: Http1RequestEncoderInner::new(buf_capacity),
        }
    }
}

#[async_trait]
impl<S, SLEEP> Http1StreamEncoder<S, SLEEP, Request<()>> for Http1RequestEncoder
where
    S: AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    async fn write_head(
        &mut self,
        stream: &mut S,
        head: Request<()>,
        body_framing: BodyFraming,
    ) -> Result<(), IoError> {
        if self.state != State::Idle {
            return Err(IoError::new(IoErrorKind::Other, "state should is Idle"));
        }

        self.buf.clear();

        let (mut parts, _) = head.into_parts();

        self.update_headers(&mut parts.headers, &parts.version, &body_framing)?;

        self.encode_head(parts)?;

        self.write_head0::<_, SLEEP>(stream).await?;

        match body_framing {
            BodyFraming::Neither => {
                self.state = State::Idle;
            }
            BodyFraming::ContentLength(n) if n == 0 => {
                self.state = State::Idle;
            }
            _ => {
                self.state = State::WriteBody(body_framing);
            }
        }

        Ok(())
    }
    async fn write_body(&mut self, stream: &mut S, body: EncoderBody) -> Result<(), IoError> {
        self.write_body0::<_, SLEEP>(stream, body).await
    }

    fn set_write_timeout(&mut self, dur: Duration) {
        self.inner.set_write_timeout(dur)
    }
}

//
//
//
pub type Http1ResponseEncoderInner =
    Http1Encoder<(ResponseParts, ReasonPhrase), ResponseHeadRenderer>;
pub struct Http1ResponseEncoder {
    inner: Http1ResponseEncoderInner,
}
impl Deref for Http1ResponseEncoder {
    type Target = Http1ResponseEncoderInner;

    fn deref(&self) -> &Http1ResponseEncoderInner {
        &self.inner
    }
}
impl DerefMut for Http1ResponseEncoder {
    fn deref_mut(&mut self) -> &mut Http1ResponseEncoderInner {
        &mut self.inner
    }
}
impl Http1ResponseEncoder {
    pub fn new(buf_capacity: usize) -> Self {
        Self {
            inner: Http1ResponseEncoderInner::new(buf_capacity),
        }
    }
}

#[async_trait]
impl<S, SLEEP> Http1StreamEncoder<S, SLEEP, (Response<()>, ReasonPhrase)> for Http1ResponseEncoder
where
    S: AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    async fn write_head(
        &mut self,
        stream: &mut S,
        head: (Response<()>, ReasonPhrase),
        body_framing: BodyFraming,
    ) -> Result<(), IoError> {
        if self.state != State::Idle {
            return Err(IoError::new(IoErrorKind::Other, "state should is Idle"));
        }

        self.buf.clear();

        let (head, reason_phrase) = head;
        let (mut parts, _) = head.into_parts();

        self.update_headers(&mut parts.headers, &parts.version, &body_framing)?;

        self.encode_head((parts, reason_phrase))?;

        self.write_head0::<_, SLEEP>(stream).await?;

        match body_framing {
            BodyFraming::Neither => {
                self.state = State::Idle;
            }
            BodyFraming::ContentLength(n) if n == 0 => {
                self.state = State::Idle;
            }
            _ => {
                self.state = State::WriteBody(body_framing);
            }
        }

        Ok(())
    }
    async fn write_body(&mut self, stream: &mut S, body: EncoderBody) -> Result<(), IoError> {
        self.write_body0::<_, SLEEP>(stream, body).await
    }

    fn set_write_timeout(&mut self, dur: Duration) {
        self.inner.set_write_timeout(dur)
    }
}
