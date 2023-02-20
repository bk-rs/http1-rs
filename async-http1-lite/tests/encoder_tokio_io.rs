#[cfg(all(not(feature = "futures_io"), feature = "tokio_io"))]
mod encoder_tokio_io_tests {
    #![allow(unused_imports)]

    use async_http1_lite::encoder::{Http1RequestEncoder, Http1ResponseEncoder};
}
