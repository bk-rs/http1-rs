/*
cargo run -p async-http1-lite-demo-tokio --bin async-http1-lite-demo-tokio_client httpbin.org 80 /ip
*/

use std::env;

use tokio::net::TcpStream;

use async_http1_lite::{http::Request, Http1ClientStream};
use async_sleep::impl_tokio::Sleep;
use tokio_util::compat::TokioAsyncReadCompatExt as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let domain = env::args()
        .nth(1)
        .unwrap_or_else(|| env::var("DOMAIN").unwrap_or("httpbin.org".to_owned()));
    let port: u16 = env::args()
        .nth(2)
        .unwrap_or_else(|| env::var("PORT").unwrap_or("80".to_owned()))
        .parse()
        .unwrap();
    let uri = env::args()
        .nth(3)
        .unwrap_or_else(|| env::var("URI").unwrap_or("/ip".to_owned()));

    println!("client {domain} {port} {uri}");

    //
    let addr = format!("{domain}:{port}");
    let stream = TcpStream::connect(addr).await?;
    let stream = stream.compat();

    //
    let mut stream: Http1ClientStream<_, Sleep> = Http1ClientStream::new(stream);

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
