use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use async_sleep::Sleepble;
use async_trait::async_trait;
use futures_util::{AsyncRead, AsyncWrite};
use http::{Request, Response};
use http1_spec::{body_framing::BodyFraming, head_renderer::Head, ReasonPhrase};

use crate::body::{DecoderBody, EncoderBody};
use crate::decoder::{Http1RequestDecoder, Http1ResponseDecoder};
use crate::encoder::{Http1RequestEncoder, Http1ResponseEncoder};

//
//
//
#[async_trait]
pub trait Http1StreamDecoder<S, SLEEP, H>
where
    S: AsyncRead + Unpin,
    SLEEP: Sleepble,
    H: Head,
{
    async fn read_head(&mut self, stream: &mut S) -> Result<(H, BodyFraming), IoError>;
    async fn read_body(&mut self, stream: &mut S) -> Result<DecoderBody, IoError>;

    fn set_read_timeout(&mut self, dur: Duration);
}

#[async_trait]
pub trait Http1StreamEncoder<S, SLEEP, H>
where
    S: AsyncWrite + Unpin,
    SLEEP: Sleepble,
    H: Head,
{
    async fn write_head(
        &mut self,
        stream: &mut S,
        head: H,
        body_framing: BodyFraming,
    ) -> Result<(), IoError>;
    async fn write_body(&mut self, stream: &mut S, body: EncoderBody) -> Result<(), IoError>;

    fn set_write_timeout(&mut self, dur: Duration);
}

//
//
//
pub struct Http1Stream<S, SLEEP, D, DH, E, EH>
where
    S: AsyncRead + AsyncWrite + Unpin,
    SLEEP: Sleepble,
    D: Http1StreamDecoder<S, SLEEP, DH>,
    DH: Head,
    E: Http1StreamEncoder<S, SLEEP, EH>,
    EH: Head,
{
    stream: S,
    decoder: D,
    encoder: E,
    phantom: PhantomData<(SLEEP, DH, EH)>,
}
impl<S, SLEEP, D, DH, E, EH> Http1Stream<S, SLEEP, D, DH, E, EH>
where
    S: AsyncRead + AsyncWrite + Unpin,
    SLEEP: Sleepble,
    D: Http1StreamDecoder<S, SLEEP, DH>,
    DH: Head,
    E: Http1StreamEncoder<S, SLEEP, EH>,
    EH: Head,
{
    pub(crate) fn new(stream: S, decoder: D, encoder: E) -> Self {
        Self {
            stream,
            decoder,
            encoder,
            phantom: PhantomData,
        }
    }

    //
    pub fn set_write_timeout(&mut self, dur: Duration) {
        self.encoder.set_write_timeout(dur)
    }

    pub fn set_read_timeout(&mut self, dur: Duration) {
        self.decoder.set_read_timeout(dur)
    }

    //
    pub async fn write_head(&mut self, head: EH, body_framing: BodyFraming) -> Result<(), IoError> {
        self.encoder
            .write_head(&mut self.stream, head, body_framing)
            .await
    }

    pub async fn write_body(&mut self, body: EncoderBody) -> Result<(), IoError> {
        self.encoder.write_body(&mut self.stream, body).await
    }

    //
    pub async fn read_head(&mut self) -> Result<(DH, BodyFraming), IoError> {
        self.decoder.read_head(&mut self.stream).await
    }
    pub async fn read_body(&mut self) -> Result<DecoderBody, IoError> {
        self.decoder.read_body(&mut self.stream).await
    }
}

//
//
//
pub type Http1ClientStreamInner<S, SLEEP> = Http1Stream<
    S,
    SLEEP,
    Http1ResponseDecoder,
    (Response<()>, ReasonPhrase),
    Http1RequestEncoder,
    Request<()>,
>;
pub struct Http1ClientStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    inner: Http1ClientStreamInner<S, SLEEP>,
}
impl<S, SLEEP> Deref for Http1ClientStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    type Target = Http1ClientStreamInner<S, SLEEP>;

    fn deref(&self) -> &Http1ClientStreamInner<S, SLEEP> {
        &self.inner
    }
}
impl<S, SLEEP> DerefMut for Http1ClientStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    fn deref_mut(&mut self) -> &mut Http1ClientStreamInner<S, SLEEP> {
        &mut self.inner
    }
}
impl<S, SLEEP> Http1ClientStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    pub fn new(stream: S) -> Self {
        Self::with(
            stream,
            Http1ResponseDecoder::new(8 * 1024, None),
            Http1RequestEncoder::new(8 * 1024),
        )
    }
    pub fn with(stream: S, decoder: Http1ResponseDecoder, encoder: Http1RequestEncoder) -> Self {
        Self {
            inner: Http1ClientStreamInner::new(stream, decoder, encoder),
        }
    }

    pub fn get_ref(&self) -> &S {
        &self.inner.stream
    }
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner.stream
    }
    pub fn into_inner(self) -> Result<S, IoError> {
        if self.decoder.has_unparsed_bytes() {
            return Err(IoError::new(IoErrorKind::Other, "has unparsed bytes"));
        }
        Ok(self.inner.stream)
    }

    pub async fn write_request(&mut self, request: Request<Vec<u8>>) -> Result<(), IoError> {
        let (parts, body) = request.into_parts();
        let head = Request::from_parts(parts, ());

        let body_framing = BodyFraming::ContentLength(body.len());

        self.write_head(head, body_framing.clone()).await?;
        match body_framing {
            BodyFraming::Neither => {}
            BodyFraming::ContentLength(n) if n == 0 => {}
            _ => {
                self.write_body(EncoderBody::Completed(body)).await?;
            }
        }

        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<(Response<Vec<u8>>, ReasonPhrase), IoError> {
        let ((response, reason_phrase), body_framing) = self.read_head().await?;

        let mut body = Vec::new();
        match body_framing {
            BodyFraming::Neither => {}
            BodyFraming::ContentLength(n) if n == 0 => {}
            _ => loop {
                match self.read_body().await? {
                    DecoderBody::Completed(bytes) => {
                        body.extend_from_slice(&bytes);
                        break;
                    }
                    DecoderBody::Partial(bytes) => {
                        body.extend_from_slice(&bytes);
                    }
                }
            },
        }

        let (parts, _) = response.into_parts();
        let response = Response::from_parts(parts, body);

        Ok((response, reason_phrase))
    }
}

impl<S, SLEEP> AsyncRead for Http1ClientStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, IoError>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}

impl<S, SLEEP> AsyncWrite for Http1ClientStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, IoError>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), IoError>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), IoError>> {
        Pin::new(&mut self.get_mut().stream).poll_close(cx)
    }
}

//
//
//
pub type Http1ServerStreamInner<S, SLEEP> = Http1Stream<
    S,
    SLEEP,
    Http1RequestDecoder,
    Request<()>,
    Http1ResponseEncoder,
    (Response<()>, ReasonPhrase),
>;
pub struct Http1ServerStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    inner: Http1ServerStreamInner<S, SLEEP>,
}
impl<S, SLEEP> Deref for Http1ServerStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    type Target = Http1ServerStreamInner<S, SLEEP>;

    fn deref(&self) -> &Http1ServerStreamInner<S, SLEEP> {
        &self.inner
    }
}
impl<S, SLEEP> DerefMut for Http1ServerStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    fn deref_mut(&mut self) -> &mut Http1ServerStreamInner<S, SLEEP> {
        &mut self.inner
    }
}
impl<S, SLEEP> Http1ServerStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble,
{
    pub fn new(stream: S) -> Self {
        Self::with(
            stream,
            Http1RequestDecoder::new(8 * 1024, None),
            Http1ResponseEncoder::new(8 * 1024),
        )
    }
    pub fn with(stream: S, decoder: Http1RequestDecoder, encoder: Http1ResponseEncoder) -> Self {
        Self {
            inner: Http1ServerStreamInner::new(stream, decoder, encoder),
        }
    }

    pub fn get_ref(&self) -> &S {
        &self.inner.stream
    }
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.inner.stream
    }
    pub fn into_inner(self) -> Result<S, IoError> {
        if self.decoder.has_unparsed_bytes() {
            return Err(IoError::new(IoErrorKind::Other, "has unparsed bytes"));
        }
        Ok(self.inner.stream)
    }

    pub async fn write_response(
        &mut self,
        response: Response<Vec<u8>>,
        reason_phrase: ReasonPhrase,
    ) -> Result<(), IoError> {
        let (parts, body) = response.into_parts();
        let head = Response::from_parts(parts, ());

        let body_framing = BodyFraming::ContentLength(body.len());

        self.write_head((head, reason_phrase), body_framing.clone())
            .await?;

        match body_framing {
            BodyFraming::Neither => {}
            BodyFraming::ContentLength(n) if n == 0 => {}
            _ => {
                self.write_body(EncoderBody::Completed(body)).await?;
            }
        }

        Ok(())
    }

    pub async fn read_request(&mut self) -> Result<Request<Vec<u8>>, IoError> {
        let (request, body_framing) = self.read_head().await?;

        let mut body = Vec::new();
        match body_framing {
            BodyFraming::Neither => {}
            BodyFraming::ContentLength(n) if n == 0 => {}
            _ => loop {
                match self.read_body().await? {
                    DecoderBody::Completed(bytes) => {
                        body.extend_from_slice(&bytes);
                        break;
                    }
                    DecoderBody::Partial(bytes) => {
                        body.extend_from_slice(&bytes);
                    }
                }
            },
        }

        let (parts, _) = request.into_parts();
        let request = Request::from_parts(parts, body);

        Ok(request)
    }
}

impl<S, SLEEP> AsyncRead for Http1ServerStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, IoError>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}

impl<S, SLEEP> AsyncWrite for Http1ServerStream<S, SLEEP>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    SLEEP: Sleepble + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, IoError>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), IoError>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), IoError>> {
        Pin::new(&mut self.get_mut().stream).poll_close(cx)
    }
}
