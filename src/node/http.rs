use super::NodeError;
use hyper::{Body, Client, Method, Request};

pub async fn json_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
    addr: &str,
    req: Req,
) -> Result<Resp, NodeError> {
    let client = Client::new();
    let req = Request::builder()
        .method(Method::POST)
        .uri(addr)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&req)?))?;
    let body = client.request(req).await?.into_body();
    let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
    Ok(resp)
}

pub async fn json_get<Resp: serde::de::DeserializeOwned>(addr: &str) -> Result<Resp, NodeError> {
    let client = Client::new();
    let req = Request::builder()
        .method(Method::GET)
        .uri(addr)
        .body(Body::empty())?;
    let body = client.request(req).await?.into_body();
    let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
    Ok(resp)
}
