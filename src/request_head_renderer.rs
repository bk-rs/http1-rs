use std::io;

use http::{request::Parts, Request, Version};

use crate::{
    head_renderer::HeadRenderer, COLON, CRLF, HTTP_VERSION_10, HTTP_VERSION_11, HTTP_VERSION_2,
    HTTP_VERSION_3, SP,
};

#[derive(Default)]
pub struct RequestHeadRenderer {}

impl HeadRenderer<Request<()>> for RequestHeadRenderer {
    fn new() -> Self {
        Self::default()
    }

    fn render(&self, head: Request<()>, buf: &mut Vec<u8>) -> io::Result<()> {
        let (parts, _) = head.into_parts();
        HeadRenderer::<Parts>::render(self, parts, buf)
    }
}

impl HeadRenderer<Parts> for RequestHeadRenderer {
    fn new() -> Self {
        Self::default()
    }

    fn render(&self, parts: Parts, buf: &mut Vec<u8>) -> io::Result<()> {
        let version_bytes = match parts.version {
            Version::HTTP_10 => HTTP_VERSION_10,
            Version::HTTP_11 => HTTP_VERSION_11,
            Version::HTTP_2 => HTTP_VERSION_2,
            Version::HTTP_3 => HTTP_VERSION_3,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "unimplemented")),
        };

        buf.extend_from_slice(parts.method.as_str().as_bytes());
        buf.extend_from_slice(&[SP]);
        buf.extend_from_slice(parts.uri.to_string().as_bytes());
        buf.extend_from_slice(&[SP]);
        buf.extend_from_slice(version_bytes);
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
