pub mod body_framing;
pub mod body_parser;
pub mod content_length_body_parser;
pub mod head_parser;
pub mod head_renderer;
pub mod request_head_parser;
pub mod request_head_renderer;
pub mod response_head_parser;
pub mod response_head_renderer;

pub use http;

//
//
//
pub const SP: u8 = b' ';
pub const HTTP_VERSION_10: &[u8] = b"HTTP/1.0";
pub const HTTP_VERSION_11: &[u8] = b"HTTP/1.1";
pub const COLON: u8 = b':';
pub const CR: u8 = b'\r';
pub const LF: u8 = b'\n';
pub const CRLF: &[u8] = b"\r\n";

pub type ReasonPhrase = Option<Vec<u8>>;

pub const CHUNKED: &str = "chunked";
