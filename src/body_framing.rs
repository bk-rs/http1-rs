use std::io;

use http::{
    header::{CONTENT_LENGTH, TRANSFER_ENCODING},
    HeaderMap, HeaderValue, Version,
};

use crate::CHUNKED;

//
//
//
// ref https://github.com/apple/swift-nio/blob/2.20.2/Sources/NIOHTTP1/HTTPEncoder.swift#L89
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BodyFraming {
    ContentLength(usize),
    Chunked,
    Neither,
}

impl BodyFraming {
    pub fn update_content_length_value(&mut self, value: usize) -> io::Result<()> {
        match self {
            Self::ContentLength(n) => {
                *n = value;
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Not in ContentLength",
            )),
        }
    }
}

pub trait BodyFramingDetector {
    fn detect(&self) -> io::Result<BodyFraming>;
}
impl BodyFramingDetector for (&HeaderMap<HeaderValue>, &Version) {
    fn detect(&self) -> io::Result<BodyFraming> {
        let (headers, version) = *self;

        if let Some(header_value) = headers.get(CONTENT_LENGTH) {
            let value_str = header_value
                .to_str()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
            let value: usize = value_str
                .parse()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
            return Ok(BodyFraming::ContentLength(value));
        }

        if version == &Version::HTTP_11 {
            if let Some(header_value) = headers.get(TRANSFER_ENCODING) {
                if header_value == CHUNKED {
                    return Ok(BodyFraming::Chunked);
                }
            }
        }

        Ok(BodyFraming::Neither)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect() -> io::Result<()> {
        let mut header_map = HeaderMap::new();

        header_map.insert("Content-Length", "1".parse().unwrap());
        assert_eq!(
            (&header_map, &Version::HTTP_11).detect()?,
            BodyFraming::ContentLength(1)
        );

        header_map.clear();
        header_map.insert("Transfer-Encoding", "chunked".parse().unwrap());
        assert_eq!(
            (&header_map, &Version::HTTP_10).detect()?,
            BodyFraming::Neither
        );
        assert_eq!(
            (&header_map, &Version::HTTP_11).detect()?,
            BodyFraming::Chunked
        );

        header_map.clear();
        assert_eq!(
            (&header_map, &Version::HTTP_11).detect()?,
            BodyFraming::Neither
        );

        Ok(())
    }
}
