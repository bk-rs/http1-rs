use async_sleep::impl_async_io::Timer;
use futures_lite::future::block_on;
use futures_util::io::Cursor;
use http::Request;
use http1_spec::body_framing::BodyFraming;

use async_http1_lite::{
    body::EncoderBody, encoder::Http1RequestEncoder, stream::Http1StreamEncoder,
};

#[test]
fn request_simple() -> Result<(), Box<dyn std::error::Error>> {
    block_on(async {
        let mut stream = Cursor::new(vec![]);
        let request = Request::builder()
            .method("GET")
            .uri("/")
            .header("Host", "example.com")
            .body(())
            .unwrap();

        let mut encoder = Http1RequestEncoder::new(1024);
        Http1StreamEncoder::<_, Timer, _>::write_head(
            &mut encoder,
            &mut stream,
            request,
            BodyFraming::Neither,
        )
        .await?;

        assert_eq!(
            stream.into_inner(),
            b"GET / HTTP/1.1\r\nhost:example.com\r\n\r\n".to_vec()
        );

        Ok(())
    })
}

#[test]
fn request_with_body() -> Result<(), Box<dyn std::error::Error>> {
    block_on(async {
        let mut stream = Cursor::new(vec![]);
        let request = Request::builder()
            .method("GET")
            .uri("/")
            .header("Host", "example.com")
            .body(())
            .unwrap();

        let mut encoder = Http1RequestEncoder::new(1024);
        Http1StreamEncoder::<_, Timer, _>::write_head(
            &mut encoder,
            &mut stream,
            request,
            BodyFraming::ContentLength(3),
        )
        .await?;
        Http1StreamEncoder::<_, Timer, _>::write_body(
            &mut encoder,
            &mut stream,
            EncoderBody::Completed(b"foo".to_vec()),
        )
        .await?;

        assert_eq!(
            stream.into_inner(),
            b"GET / HTTP/1.1\r\nhost:example.com\r\ncontent-length:3\r\n\r\nfoo".to_vec()
        );

        Ok(())
    })
}
