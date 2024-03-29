pub use http;

//
//
//
pub mod body_framing;
pub mod body_parser;
pub mod chunked_body_parser;
pub mod content_length_body_parser;
pub mod head_parser;
pub mod head_renderer;
pub mod request_head_parser;
pub mod request_head_renderer;
pub mod response_head_parser;
pub mod response_head_renderer;

//
//
//
pub const SP: u8 = b' ';
pub const HTTP_VERSION_10: &[u8] = b"HTTP/1.0";
pub const HTTP_VERSION_11: &[u8] = b"HTTP/1.1";
pub const HTTP_VERSION_20: &[u8] = b"HTTP/2.0";
pub const HTTP_VERSION_2: &[u8] = b"HTTP/2";
pub const HTTP_VERSION_30: &[u8] = b"HTTP/3.0";
pub const HTTP_VERSION_3: &[u8] = b"HTTP/3";
pub const COLON: u8 = b':';
pub const CR: u8 = b'\r';
pub const LF: u8 = b'\n';
pub const CRLF: &[u8] = b"\r\n";

pub type ReasonPhrase = Option<Vec<u8>>;

pub const CHUNKED: &str = "chunked";
