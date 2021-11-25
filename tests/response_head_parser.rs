use std::{
    error::Error,
    io::{BufReader, Cursor},
};

use http::{StatusCode, Version};

use http1_spec::{
    head_parser::{HeadParseOutput, HeadParser},
    response_head_parser::ResponseHeadParser,
};

#[test]
fn simple() -> Result<(), Box<dyn Error>> {
    let mut p = ResponseHeadParser::with_config(Default::default());

    let o = p.parse(&mut BufReader::new(Cursor::new(b"HTTP/1.1 200 OK\r\n\r\n")))?;
    assert_eq!(o, HeadParseOutput::Completed(19));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::OK);
    assert_eq!(p.reason_phrase, Some(b"OK"[..].to_vec()));
    assert_eq!(p.headers.len(), 0);

    Ok(())
}

#[test]
fn version_http2() -> Result<(), Box<dyn Error>> {
    let mut p = ResponseHeadParser::with_config(Default::default());

    // curl https://www.google.com/ -v
    let o = p.parse(&mut BufReader::new(Cursor::new(b"HTTP/2 200 OK\r\n\r\n")))?;
    assert_eq!(o, HeadParseOutput::Completed(17));

    assert_eq!(p.http_version, Version::HTTP_2);
    assert_eq!(p.status_code, StatusCode::OK);
    assert_eq!(p.reason_phrase, Some(b"OK"[..].to_vec()));
    assert_eq!(p.headers.len(), 0);

    Ok(())
}

#[test]
fn version_http3() -> Result<(), Box<dyn Error>> {
    let mut p = ResponseHeadParser::with_config(Default::default());

    // curl-quiche-http3 https://quic.aiortc.org/ -v --http3
    let o = p.parse(&mut BufReader::new(Cursor::new(b"HTTP/3 200 OK\r\n\r\n")))?;
    assert_eq!(o, HeadParseOutput::Completed(17));

    assert_eq!(p.http_version, Version::HTTP_3);
    assert_eq!(p.status_code, StatusCode::OK);
    assert_eq!(p.reason_phrase, Some(b"OK"[..].to_vec()));
    assert_eq!(p.headers.len(), 0);

    Ok(())
}

#[test]
fn reason_missing() -> Result<(), Box<dyn Error>> {
    let mut p = ResponseHeadParser::with_config(Default::default());

    let o = p.parse(&mut BufReader::new(Cursor::new(b"HTTP/1.0 201 \r\n\r\n")))?;
    assert_eq!(o, HeadParseOutput::Completed(17));

    assert_eq!(p.http_version, Version::HTTP_10);
    assert_eq!(p.status_code, StatusCode::CREATED);
    assert_eq!(p.reason_phrase, None);
    assert_eq!(p.headers.len(), 0);

    Ok(())
}

#[test]
fn with_headers() -> Result<(), Box<dyn Error>> {
    let mut p = ResponseHeadParser::with_config(Default::default());

    let o = p.parse(&mut BufReader::new(Cursor::new(
        b"HTTP/1.1 200 OK\r\nFoo: bar\r\n\r\n",
    )))?;
    assert_eq!(o, HeadParseOutput::Completed(29));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::OK);
    assert_eq!(p.reason_phrase, Some(b"OK"[..].to_vec()));
    assert_eq!(p.headers.len(), 1);
    assert_eq!(p.headers.get("Foo").unwrap().to_str().unwrap(), "bar");

    Ok(())
}

#[test]
fn full() -> Result<(), Box<dyn Error>> {
    let mut p = ResponseHeadParser::with_config(Default::default());

    let bytes = b"HTTP/1.1 202 Accepted\r\nFoo: bar\r\nX-V: 1\r\n\r\n";

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[0..10])))?;
    assert_eq!(o, HeadParseOutput::Partial(9));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::OK);
    assert_eq!(p.reason_phrase, None);
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[9..20])))?;
    assert_eq!(o, HeadParseOutput::Partial(4));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::ACCEPTED);
    assert_eq!(p.reason_phrase, None);
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[13..20])))?;
    assert_eq!(o, HeadParseOutput::Partial(0));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::ACCEPTED);
    assert_eq!(p.reason_phrase, None);
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[13..30])))?;
    assert_eq!(o, HeadParseOutput::Partial(10));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::ACCEPTED);
    assert_eq!(p.reason_phrase, Some(b"Accepted"[..].to_vec()));
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[23..35])))?;
    assert_eq!(o, HeadParseOutput::Partial(10));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::ACCEPTED);
    assert_eq!(p.reason_phrase, Some(b"Accepted"[..].to_vec()));
    assert_eq!(p.headers.len(), 1);
    assert_eq!(p.headers.get("Foo").unwrap().to_str().unwrap(), "bar");

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[33..41])))?;
    assert_eq!(o, HeadParseOutput::Partial(8));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::ACCEPTED);
    assert_eq!(p.reason_phrase, Some(b"Accepted"[..].to_vec()));
    assert_eq!(p.headers.len(), 2);
    assert_eq!(p.headers.get("Foo").unwrap().to_str().unwrap(), "bar");
    assert_eq!(p.headers.get("X-V").unwrap().to_str().unwrap(), "1");

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[41..])))?;
    assert_eq!(o, HeadParseOutput::Completed(2));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::ACCEPTED);
    assert_eq!(p.reason_phrase, Some(b"Accepted"[..].to_vec()));
    assert_eq!(p.headers.len(), 2);
    assert_eq!(p.headers.get("Foo").unwrap().to_str().unwrap(), "bar");
    assert_eq!(p.headers.get("X-V").unwrap().to_str().unwrap(), "1");

    // again
    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[0..30])))?;
    assert_eq!(o, HeadParseOutput::Partial(23));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::ACCEPTED);
    assert_eq!(p.reason_phrase, Some(b"Accepted"[..].to_vec()));
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[23..])))?;
    assert_eq!(o, HeadParseOutput::Completed(20));

    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.status_code, StatusCode::ACCEPTED);
    assert_eq!(p.reason_phrase, Some(b"Accepted"[..].to_vec()));
    assert_eq!(p.headers.len(), 2);
    assert_eq!(p.headers.get("Foo").unwrap().to_str().unwrap(), "bar");
    assert_eq!(p.headers.get("X-V").unwrap().to_str().unwrap(), "1");

    Ok(())
}
