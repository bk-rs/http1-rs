use std::io;

use http::{response::Parts, Response, Version};

use crate::{
    head_renderer::HeadRenderer, ReasonPhrase, COLON, CRLF, HTTP_VERSION_10, HTTP_VERSION_11, SP,
};

#[derive(Default)]
pub struct ResponseHeadRenderer {}

impl HeadRenderer<(Response<()>, ReasonPhrase)> for ResponseHeadRenderer {
    fn new() -> Self {
        Self::default()
    }

    fn render(&self, head: (Response<()>, ReasonPhrase), buf: &mut Vec<u8>) -> io::Result<()> {
        let (head, reason_phrase) = head;

        let (parts, _) = head.into_parts();
        HeadRenderer::<(Parts, ReasonPhrase)>::render(self, (parts, reason_phrase), buf)
    }
}

impl HeadRenderer<(Parts, ReasonPhrase)> for ResponseHeadRenderer {
    fn new() -> Self {
        Self::default()
    }

    fn render(&self, head: (Parts, ReasonPhrase), buf: &mut Vec<u8>) -> io::Result<()> {
        let (parts, reason_phrase) = head;

        let version_bytes = match parts.version {
            Version::HTTP_10 => HTTP_VERSION_10,
            Version::HTTP_11 => HTTP_VERSION_11,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "unimplemented")),
        };

        buf.extend_from_slice(version_bytes);
        buf.extend_from_slice(&[SP]);
        buf.extend_from_slice(parts.status.as_str().as_bytes());
        buf.extend_from_slice(&[SP]);
        if let Some(reason_phrase) = reason_phrase.or_else(|| {
            parts
                .status
                .canonical_reason()
                .map(|x| x.as_bytes().to_vec())
        }) {
            buf.extend_from_slice(&reason_phrase[..]);
        }
        buf.extend_from_slice(CRLF);

        for (k, v) in &parts.headers {
            buf.extend_from_slice(k.to_string().as_bytes());
            buf.extend_from_slice(&[COLON]);
            buf.extend_from_slice(v.as_bytes());
            buf.extend_from_slice(CRLF);
        }

        buf.extend_from_slice(CRLF);

        Ok(())
    }
}
