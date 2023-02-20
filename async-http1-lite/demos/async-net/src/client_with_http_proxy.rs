/*
cargo run -p async-http1-lite-demo-async-net --bin client_with_http_proxy 127.0.0.1 8118 httpbin.org 80 /ip
*/

use std::env;
use std::io;

use async_net::TcpStream;
use futures_lite::future::block_on;

use async_http1_lite::{Http1ClientStream, Request, Version};

fn main() -> io::Result<()> {
    block_on(run())
}

async fn run() -> io::Result<()> {
    let proxy_domain = env::args()
        .nth(1)
        .unwrap_or_else(|| env::var("PROXY_DOMAIN").unwrap_or("127.0.0.1".to_owned()));
    let proxy_port: u16 = env::args()
        .nth(2)
        .unwrap_or_else(|| env::var("PROXY_PORT").unwrap_or("8118".to_owned()))
        .parse()
        .unwrap();
    let domain = env::args()
        .nth(3)
        .unwrap_or_else(|| env::var("DOMAIN").unwrap_or("httpbin.org".to_owned()));
    let port: u16 = env::args()
        .nth(4)
        .unwrap_or_else(|| env::var("PORT").unwrap_or("443".to_owned()))
        .parse()
        .unwrap();
    let uri = env::args()
        .nth(5)
        .unwrap_or_else(|| env::var("URI").unwrap_or("/ip".to_owned()));

    println!(
        "client_with_http_proxy {} {} {} {} {}",
        proxy_domain, proxy_port, domain, port, uri
    );

    //
    let addr = format!("{}:{}", proxy_domain, proxy_port);
    let stream = TcpStream::connect(addr).await?;

    //
    let mut stream = Http1ClientStream::new(stream);

    let proxy_request = Request::builder()
        .method("CONNECT")
        .uri(format!("{}:{}", domain, port))
        .version(Version::HTTP_11)
        .header("Host", format!("{}:{}", domain, port))
        .header("User-Agent", "curl/7.71.1")
        .header("Proxy-Connection", "Keep-Alive")
        .body(vec![])
        .unwrap();
    println!("{:?}", proxy_request);

    stream.write_request(proxy_request).await?;

    let (proxy_response, _) = stream.read_response().await?;

    println!("{:?}", proxy_response);

    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("Host", domain)
        .header("User-Agent", "curl/7.71.1")
        .header("Accept", "*/*")
        .body(vec![])
        .unwrap();
    println!("{:?}", request);

    stream.write_request(request).await?;

    let (response, _) = stream.read_response().await?;

    println!("{:?}", response);

    println!("done");

    Ok(())
}
