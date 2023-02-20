/*
cargo run -p async-http1-lite-demo-async-net --bin async-http1-lite-demo-async-net_client httpbin.org 80 /ip
*/

use std::env;

use async_net::TcpStream;
use async_sleep::impl_async_io::Timer;
use futures_lite::future::block_on;

use async_http1_lite::{http::Request, Http1ClientStream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    block_on(run())
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

    println!("client {} {} {}", domain, port, uri);

    //
    let addr = format!("{}:{}", domain, port);
    let stream = TcpStream::connect(addr).await?;

    //
    let mut stream: Http1ClientStream<_, Timer> = Http1ClientStream::new(stream);

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