cfg_if::cfg_if! {
    if #[cfg(all(feature = "futures_io", not(feature = "tokio_io")))] {
        pub mod body;
        pub mod decoder;
        pub mod encoder;
        pub mod stream;

        pub use body::{DecoderBody, EncoderBody};
        pub use decoder::{Http1RequestDecoder, Http1ResponseDecoder};
        pub use encoder::{Http1RequestEncoder, Http1ResponseEncoder};
        pub use stream::{Http1ClientStream, Http1ServerStream};
    } else if #[cfg(all(not(feature = "futures_io"), feature = "tokio_io"))] {
        pub mod body;
        pub mod decoder;
        pub mod encoder;
        pub mod stream;

        pub use body::{DecoderBody, EncoderBody};
        pub use decoder::{Http1RequestDecoder, Http1ResponseDecoder};
        pub use encoder::{Http1RequestEncoder, Http1ResponseEncoder};
        pub use stream::{Http1ClientStream, Http1ServerStream};
    }
}

//
//
//
pub use http::{Method, Request, Response, StatusCode, Version};
pub use http1_spec;
