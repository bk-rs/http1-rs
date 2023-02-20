#[cfg(all(not(feature = "futures_io"), feature = "tokio_io"))]
mod decoder_tokio_io_tests {
    #![allow(unused_imports)]

    use async_http1_lite::decoder::{Http1RequestDecoder, Http1ResponseDecoder};
}
