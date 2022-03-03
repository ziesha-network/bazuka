use super::NodeError;
use hyper::{Body, Client, Method, Request};
use std::net::IpAddr;

pub async fn get_public_ip() -> Result<IpAddr, NodeError> {
    let client = Client::new();
    let req = Request::builder()
        .method(Method::GET)
        .uri("http://ifconfig.io/ip")
        .body(Body::empty())?;
    let body = client.request(req).await?.into_body();
    let resp_bytes = hyper::body::to_bytes(body).await?;
    let resp = std::str::from_utf8(&resp_bytes)?.trim();
    Ok(resp.parse()?)
}
