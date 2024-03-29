/*
cargo run -p async-http1-lite-demo-async-net --bin async_http1_lite_demo_async_net_client_with_http_proxy 127.0.0.1 8118 httpbin.org 80 /ip
*/

use std::env;

use async_net::TcpStream;
use async_sleep::impl_async_io::Timer;
use futures_lite::future::block_on;

use async_http1_lite::{
    http::{Request, Version},
    Http1ClientStream,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
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

    println!("client_with_http_proxy {proxy_domain} {proxy_port} {domain} {port} {uri}");

    //
    let addr = format!("{proxy_domain}:{proxy_port}");
    let stream = TcpStream::connect(addr).await?;

    //
    let mut stream: Http1ClientStream<_, Timer> = Http1ClientStream::new(stream);

    let proxy_request = Request::builder()
        .method("CONNECT")
        .uri(format!("{domain}:{port}"))
        .version(Version::HTTP_11)
        .header("Host", format!("{domain}:{port}"))
        .header("User-Agent", "async-http1-lite")
        .header("Proxy-Connection", "Keep-Alive")
        .body(vec![])
        .unwrap();
    println!("{proxy_request:?}");

    stream.write_request(proxy_request).await?;

    let (proxy_response, _) = stream.read_response().await?;

    let (proxy_response_parts, proxy_response_body) = proxy_response.into_parts();
    println!("{proxy_response_parts:?}");
    println!("{:?}", String::from_utf8(proxy_response_body));

    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .header("Host", domain)
        .header("User-Agent", "async-http1-lite")
        .header("Accept", "*/*")
        .body(vec![])
        .unwrap();
    println!("{request:?}");

    stream.write_request(request).await?;

    let (response, _) = stream.read_response().await?;

    let (response_parts, response_body) = response.into_parts();
    println!("{response_parts:?}");
    println!("{:?}", String::from_utf8(response_body));

    Ok(())
}
