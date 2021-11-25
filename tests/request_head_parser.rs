use std::{
    error::Error,
    io::{BufReader, Cursor},
};

use http::{Method, Version};

use http1_spec::{
    head_parser::{HeadParseOutput, HeadParser},
    request_head_parser::RequestHeadParser,
};

#[test]
fn simple() -> Result<(), Box<dyn Error>> {
    let mut p = RequestHeadParser::with_config(Default::default());

    let o = p.parse(&mut BufReader::new(Cursor::new(b"GET / HTTP/1.1\r\n\r\n")))?;
    assert_eq!(o, HeadParseOutput::Completed(18));

    assert_eq!(p.method, Method::GET);
    assert_eq!(p.uri, "/");
    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.headers.len(), 0);

    Ok(())
}

#[test]
fn version_http2() -> Result<(), Box<dyn Error>> {
    let mut p = RequestHeadParser::with_config(Default::default());

    // curl https://www.google.com/ -v
    let o = p.parse(&mut BufReader::new(Cursor::new(b"GET / HTTP/2\r\n\r\n")))?;
    assert_eq!(o, HeadParseOutput::Completed(16));

    assert_eq!(p.method, Method::GET);
    assert_eq!(p.uri, "/");
    assert_eq!(p.http_version, Version::HTTP_2);
    assert_eq!(p.headers.len(), 0);

    Ok(())
}

#[test]
fn version_http3() -> Result<(), Box<dyn Error>> {
    let mut p = RequestHeadParser::with_config(Default::default());

    // curl-quiche-http3 https://quic.aiortc.org/ -v --http3
    let o = p.parse(&mut BufReader::new(Cursor::new(b"GET / HTTP/3\r\n\r\n")))?;
    assert_eq!(o, HeadParseOutput::Completed(16));

    assert_eq!(p.method, Method::GET);
    assert_eq!(p.uri, "/");
    assert_eq!(p.http_version, Version::HTTP_3);
    assert_eq!(p.headers.len(), 0);

    Ok(())
}

#[test]
fn with_headers() -> Result<(), Box<dyn Error>> {
    let mut p = RequestHeadParser::with_config(Default::default());

    let o = p.parse(&mut BufReader::new(Cursor::new(
        &b"GET / HTTP/1.1\r\nHost: foo.com\r\nCookie: \r\n\r\n"[..],
    )))?;
    assert_eq!(o, HeadParseOutput::Completed(43));

    assert_eq!(p.method, Method::GET);
    assert_eq!(p.uri, "/");
    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.headers.len(), 2);
    assert_eq!(p.headers.get("Host").unwrap().to_str().unwrap(), "foo.com");
    assert_eq!(p.headers.get("Cookie").unwrap().to_str().unwrap(), "");

    Ok(())
}

#[test]
fn full() -> Result<(), Box<dyn Error>> {
    let mut p = RequestHeadParser::with_config(Default::default());

    let bytes = b"PUT /x HTTP/1.0\r\nHost: foo.com\r\nCookie: \r\n\r\n";

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[0..5])))?;
    assert_eq!(o, HeadParseOutput::Partial(4));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/");
    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[4..10])))?;
    assert_eq!(o, HeadParseOutput::Partial(3));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/x");
    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[7..10])))?;
    assert_eq!(o, HeadParseOutput::Partial(0));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/x");
    assert_eq!(p.http_version, Version::HTTP_11);
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[7..20])))?;
    assert_eq!(o, HeadParseOutput::Partial(10));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/x");
    assert_eq!(p.http_version, Version::HTTP_10);
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[17..40])))?;
    assert_eq!(o, HeadParseOutput::Partial(15));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/x");
    assert_eq!(p.http_version, Version::HTTP_10);
    assert_eq!(p.headers.len(), 1);
    assert_eq!(p.headers.get("Host").unwrap().to_str().unwrap(), "foo.com");

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[32..42])))?;
    assert_eq!(o, HeadParseOutput::Partial(10));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/x");
    assert_eq!(p.http_version, Version::HTTP_10);
    assert_eq!(p.headers.len(), 2);
    assert_eq!(p.headers.get("Host").unwrap().to_str().unwrap(), "foo.com");
    assert_eq!(p.headers.get("Cookie").unwrap().to_str().unwrap(), "");

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[42..])))?;
    assert_eq!(o, HeadParseOutput::Completed(2));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/x");
    assert_eq!(p.http_version, Version::HTTP_10);
    assert_eq!(p.headers.len(), 2);
    assert_eq!(p.headers.get("Host").unwrap().to_str().unwrap(), "foo.com");
    assert_eq!(p.headers.get("Cookie").unwrap().to_str().unwrap(), "");

    // again
    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[0..20])))?;
    assert_eq!(o, HeadParseOutput::Partial(17));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/x");
    assert_eq!(p.http_version, Version::HTTP_10);
    assert_eq!(p.headers.len(), 0);

    let o = p.parse(&mut BufReader::new(Cursor::new(&bytes[17..])))?;
    assert_eq!(o, HeadParseOutput::Completed(27));

    assert_eq!(p.method, Method::PUT);
    assert_eq!(p.uri, "/x");
    assert_eq!(p.http_version, Version::HTTP_10);
    assert_eq!(p.headers.len(), 2);
    assert_eq!(p.headers.get("Host").unwrap().to_str().unwrap(), "foo.com");
    assert_eq!(p.headers.get("Cookie").unwrap().to_str().unwrap(), "");

    Ok(())
}
