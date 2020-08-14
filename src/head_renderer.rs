use std::io;

use http::{request, response, Request, Response};

use crate::ReasonPhrase;

pub trait HeadRenderer<H: Head> {
    fn new() -> Self;
    fn render(&self, head: H, buf: &mut Vec<u8>) -> io::Result<()>;
}

pub trait Head {}

impl Head for Request<()> {}
impl Head for request::Parts {}

impl Head for (Response<()>, ReasonPhrase) {}
impl Head for (response::Parts, ReasonPhrase) {}
