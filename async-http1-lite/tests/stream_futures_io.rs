#[cfg(all(feature = "futures_io", not(feature = "tokio_io")))]
mod stream_futures_io_tests {
    use std::io;

    use futures_lite::future::block_on;
    use futures_lite::io::Cursor;
    use futures_lite::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    use async_http1_lite::stream::{Http1ClientStream, Http1ServerStream};

    struct MyStream<S>(S)
    where
        S: AsyncRead + AsyncWrite;

    #[test]
    fn client_get_ref() -> Result<(), IoError> {
        let cursor = Cursor::new(vec![]);

        let stream = Http1ClientStream::new(cursor);

        assert_eq!(stream.get_ref().get_ref(), &[]);

        Ok(())
    }

    #[test]
    fn client_get_mut() -> Result<(), IoError> {
        let cursor = Cursor::new(vec![]);

        let mut stream = Http1ClientStream::new(cursor);

        assert_eq!(stream.get_mut().get_mut(), &mut []);

        Ok(())
    }

    #[test]
    fn client_into_inner() -> Result<(), IoError> {
        let cursor = Cursor::new(vec![]);

        let stream = Http1ClientStream::new(cursor);

        assert_eq!(stream.into_inner()?.into_inner(), []);

        Ok(())
    }

    #[test]
    fn client_read_and_write() -> Result<(), IoError> {
        block_on(async {
            let cursor = Cursor::new(b"foo".to_vec());

            let mut stream = Http1ClientStream::new(cursor);

            let mut buf = vec![0u8; 5];
            let n = stream.read(&mut buf).await?;
            assert_eq!(n, 3);
            assert_eq!(buf, b"foo\0\0");

            stream.write(b"bar").await?;

            Ok(())
        })
    }

    #[test]
    fn client_asyncread_asyncwrite_bound() -> Result<(), IoError> {
        let cursor = Cursor::new(b"".to_vec());
        let stream = Http1ClientStream::new(cursor);
        MyStream(stream);

        Ok(())
    }

    #[test]
    fn server_get_ref() -> Result<(), IoError> {
        let cursor = Cursor::new(vec![]);

        let stream = Http1ServerStream::new(cursor);

        assert_eq!(stream.get_ref().get_ref(), &[]);

        Ok(())
    }

    #[test]
    fn server_get_mut() -> Result<(), IoError> {
        let cursor = Cursor::new(vec![]);

        let mut stream = Http1ServerStream::new(cursor);

        assert_eq!(stream.get_mut().get_mut(), &mut []);

        Ok(())
    }

    #[test]
    fn server_into_inner() -> Result<(), IoError> {
        let cursor = Cursor::new(vec![]);

        let stream = Http1ServerStream::new(cursor);

        assert_eq!(stream.into_inner()?.into_inner(), []);

        Ok(())
    }

    #[test]
    fn server_read_and_write() -> Result<(), IoError> {
        block_on(async {
            let cursor = Cursor::new(b"foo".to_vec());

            let mut stream = Http1ServerStream::new(cursor);

            let mut buf = vec![0u8; 5];
            let n = stream.read(&mut buf).await?;
            assert_eq!(n, 3);
            assert_eq!(buf, b"foo\0\0");

            stream.write(b"bar").await?;

            Ok(())
        })
    }

    #[test]
    fn server_asyncread_asyncwrite_bound() -> Result<(), IoError> {
        let cursor = Cursor::new(b"".to_vec());
        let stream = Http1ServerStream::new(cursor);
        MyStream(stream);

        Ok(())
    }
}
