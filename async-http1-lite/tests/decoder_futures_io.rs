#[cfg(all(feature = "futures_io", not(feature = "tokio_io")))]
mod decoder_futures_io_tests {
    use std::io;

    use futures_lite::future::block_on;
    use futures_lite::io::Cursor;
    use http::{Method, Version};
    use http1_spec::body_framing::BodyFraming;

    use async_http1_lite::{decoder::Http1RequestDecoder, stream::Http1StreamDecoder};

    #[test]
    fn request_simple() -> io::Result<()> {
        block_on(async {
            let mut stream = Cursor::new(
                "GET / HTTP/1.1\r\nHost: foo.com\r\n\r\nPOST /x HTTP/1.0\r\nHost: bar.com\r\n\r\n",
            );

            let mut decoder = Http1RequestDecoder::new(1024, None);

            let (request, body_framing) = decoder.read_head(&mut stream).await?;
            assert_eq!(request.method(), Method::GET);
            assert_eq!(request.uri(), "/");
            assert_eq!(request.version(), Version::HTTP_11);
            assert_eq!(request.headers().len(), 1);
            assert_eq!(
                request.headers().get("host").unwrap().to_str().unwrap(),
                "foo.com"
            );
            assert_eq!(body_framing, BodyFraming::Neither);

            let (request, body_framing) = decoder.read_head(&mut stream).await?;
            assert_eq!(request.method(), Method::POST);
            assert_eq!(request.uri(), "/x");
            assert_eq!(request.version(), Version::HTTP_10);
            assert_eq!(request.headers().len(), 1);
            assert_eq!(
                request.headers().get("host").unwrap().to_str().unwrap(),
                "bar.com"
            );
            assert_eq!(body_framing, BodyFraming::Neither);

            let err = decoder.read_head(&mut stream).await.err().unwrap();
            assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
            assert_eq!(err.to_string(), "read 0");

            Ok(())
        })
    }
}
