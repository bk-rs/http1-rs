#[cfg(all(not(feature = "futures_io"), feature = "tokio_io"))]
mod stream_tokio_io_tests {
    #![allow(unused_imports)]

    use async_http1_lite::stream::{Http1ClientStream, Http1ServerStream};
}
