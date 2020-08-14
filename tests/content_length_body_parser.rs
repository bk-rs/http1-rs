use std::io::{self, BufReader, Cursor};

use http1_spec::{
    body_parser::{BodyParseOutput, BodyParser},
    content_length_body_parser::ContentLengthBodyParser,
};

#[test]
fn simple() -> io::Result<()> {
    let mut p = ContentLengthBodyParser::new();
    p.set_length(5);

    let mut body_buf = vec![0u8; 5];
    let o = p.parse(&mut BufReader::new(Cursor::new(b"abcdefgh")), &mut body_buf)?;
    assert_eq!(o, BodyParseOutput::Completed(5));

    assert_eq!(p.get_length(), 0);
    assert_eq!(body_buf, b"abcde".to_vec());

    Ok(())
}

#[test]
fn partial() -> io::Result<()> {
    let mut p = ContentLengthBodyParser::new();
    p.set_length(10);

    let mut body_buf = vec![0u8; 10];
    let o = p.parse(&mut BufReader::new(Cursor::new(b"abcdefgh")), &mut body_buf)?;
    assert_eq!(o, BodyParseOutput::Partial(8));

    assert_eq!(p.get_length(), 2);
    assert_eq!(body_buf, b"abcdefgh\0\0".to_vec());

    Ok(())
}
