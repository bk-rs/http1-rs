use std::io::{self, BufReader, Cursor};
use std::str;

use http1_spec::{
    body_parser::{BodyParseError, BodyParseOutput, BodyParser},
    chunked_body_parser::ChunkedBodyParser,
};

#[test]
fn simple() -> io::Result<()> {
    // https://en.wikipedia.org/wiki/Chunked_transfer_encoding

    let mut p = ChunkedBodyParser::new();

    let mut body_buf = vec![];
    let o = p.parse(
        &mut BufReader::new(Cursor::new(
            b"4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\n\r\nfoo".to_vec(),
        )),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Completed(43));

    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia in\r\n\r\nchunks.".to_vec());

    Ok(())
}

#[test]
fn partial() -> io::Result<()> {
    let mut p = ChunkedBodyParser::new();

    let mut body_buf = vec![];
    let o = p.parse(
        &mut BufReader::new(Cursor::new(b"4\r\nWiki\r\n5\r\np")),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(13));

    assert_eq!(body_buf, b"Wikip".to_vec());

    Ok(())
}

#[test]
fn invalid_crlf_with_data_end() -> io::Result<()> {
    let mut p = ChunkedBodyParser::new();

    let mut body_buf = vec![];
    let err = p
        .parse(
            &mut BufReader::new(Cursor::new(b"4\r\nWikix\n")),
            &mut body_buf,
        )
        .err()
        .unwrap();
    match err {
        BodyParseError::InvalidCRLF => {}
        _ => assert!(false, "err not match"),
    }

    Ok(())
}

#[test]
fn invalid_crlf_with_all_end() -> io::Result<()> {
    let mut p = ChunkedBodyParser::new();

    let mut body_buf = vec![];
    let err = p
        .parse(
            &mut BufReader::new(Cursor::new(b"4\r\nWiki\r\n0\r\nx\n")),
            &mut body_buf,
        )
        .err()
        .unwrap();
    match err {
        BodyParseError::InvalidCRLF => {}
        _ => assert!(false, "err not match"),
    }

    Ok(())
}

#[test]
fn full() -> io::Result<()> {
    let mut p = ChunkedBodyParser::new();

    let mut body_buf = vec![];

    let bytes = b"4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\n\r\nfoo";

    //
    body_buf.clear();

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[0..4])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(4));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"W".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[4..8])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(3));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wiki".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[7..11])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(2));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wiki".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[9..13])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(4));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikip".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[13..18])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(4));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[17..21])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(2));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[19..23])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(4));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia ".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[23..27])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(4));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia in\r\n".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[27..37])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(9));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia in\r\n\r\nchunks.".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[36..40])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(2));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia in\r\n\r\nchunks.".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[38..42])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(3));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia in\r\n\r\nchunks.".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[41..])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Completed(2));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia in\r\n\r\nchunks.".to_vec());

    // again
    body_buf.clear();

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[0..20])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(19));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[19..39])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Partial(19));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia in\r\n\r\nchunks.".to_vec());

    let o = p.parse(
        &mut BufReader::new(Cursor::new(&bytes[38..])),
        &mut body_buf,
    )?;
    assert_eq!(o, BodyParseOutput::Completed(5));
    println!("{:?}", str::from_utf8(&body_buf));
    assert_eq!(body_buf, b"Wikipedia in\r\n\r\nchunks.".to_vec());

    Ok(())
}
