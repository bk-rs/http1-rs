use async_sleep::impl_async_io::Timer;
use futures_lite::future::block_on;
use futures_util::{io::Cursor, AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};

use async_http1_lite::stream::{Http1ClientStream, Http1ServerStream};

struct MyStream<S>(S)
where
    S: AsyncRead + AsyncWrite;

#[test]
fn client_get_ref() -> Result<(), Box<dyn std::error::Error>> {
    let cursor = Cursor::new(vec![]);

    let stream: Http1ClientStream<_, Timer> = Http1ClientStream::new(cursor);

    assert_eq!(stream.get_ref().get_ref(), &[]);

    Ok(())
}

#[test]
fn client_get_mut() -> Result<(), Box<dyn std::error::Error>> {
    let cursor = Cursor::new(vec![]);

    let mut stream: Http1ClientStream<_, Timer> = Http1ClientStream::new(cursor);

    assert_eq!(stream.get_mut().get_mut(), &mut []);

    Ok(())
}

#[test]
fn client_into_inner() -> Result<(), Box<dyn std::error::Error>> {
    let cursor = Cursor::new(vec![]);

    let stream: Http1ClientStream<_, Timer> = Http1ClientStream::new(cursor);

    assert_eq!(stream.into_inner()?.into_inner(), []);

    Ok(())
}

#[test]
fn client_read_and_write() -> Result<(), Box<dyn std::error::Error>> {
    block_on(async {
        let cursor = Cursor::new(b"foo".to_vec());

        let mut stream: Http1ClientStream<_, Timer> = Http1ClientStream::new(cursor);

        let mut buf = vec![0u8; 5];
        let n = stream.read(&mut buf).await?;
        assert_eq!(n, 3);
        assert_eq!(buf, b"foo\0\0");

        stream.write_all(b"bar").await?;

        Ok(())
    })
}

#[test]
fn client_asyncread_asyncwrite_bound() -> Result<(), Box<dyn std::error::Error>> {
    let cursor = Cursor::new(b"".to_vec());
    let stream: Http1ClientStream<_, Timer> = Http1ClientStream::new(cursor);
    let _ = MyStream(stream);

    Ok(())
}

#[test]
fn server_get_ref() -> Result<(), Box<dyn std::error::Error>> {
    let cursor = Cursor::new(vec![]);

    let stream: Http1ServerStream<_, Timer> = Http1ServerStream::new(cursor);

    assert_eq!(stream.get_ref().get_ref(), &[]);

    Ok(())
}

#[test]
fn server_get_mut() -> Result<(), Box<dyn std::error::Error>> {
    let cursor = Cursor::new(vec![]);

    let mut stream: Http1ServerStream<_, Timer> = Http1ServerStream::new(cursor);

    assert_eq!(stream.get_mut().get_mut(), &mut []);

    Ok(())
}

#[test]
fn server_into_inner() -> Result<(), Box<dyn std::error::Error>> {
    let cursor = Cursor::new(vec![]);

    let stream: Http1ServerStream<_, Timer> = Http1ServerStream::new(cursor);

    assert_eq!(stream.into_inner()?.into_inner(), []);

    Ok(())
}

#[test]
fn server_read_and_write() -> Result<(), Box<dyn std::error::Error>> {
    block_on(async {
        let cursor = Cursor::new(b"foo".to_vec());

        let mut stream: Http1ServerStream<_, Timer> = Http1ServerStream::new(cursor);

        let mut buf = vec![0u8; 5];
        let n = stream.read(&mut buf).await?;
        assert_eq!(n, 3);
        assert_eq!(buf, b"foo\0\0");

        stream.write_all(b"bar").await?;

        Ok(())
    })
}

#[test]
fn server_asyncread_asyncwrite_bound() -> Result<(), Box<dyn std::error::Error>> {
    let cursor = Cursor::new(b"".to_vec());
    let stream: Http1ServerStream<_, Timer> = Http1ServerStream::new(cursor);
    let _ = MyStream(stream);

    Ok(())
}
