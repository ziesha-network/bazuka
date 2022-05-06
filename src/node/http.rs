use async_trait::async_trait;

use super::{NodeError, PeerAddress};
use futures::future::join_all;
use hyper::{Body, Client, Method, Request};

#[async_trait(?Send)]
pub trait Network {
    async fn bincode_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
    ) -> Result<Resp, NodeError>;
    async fn bincode_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
    ) -> Result<Resp, NodeError>;
    async fn json_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
    ) -> Result<Resp, NodeError>;
    async fn json_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
    ) -> Result<Resp, NodeError>;
}

pub struct Internet;

impl Internet {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait(?Send)]
impl Network for Internet {
    async fn bincode_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
    ) -> Result<Resp, NodeError> {
        let client = Client::new();
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("{}?{}", addr, serde_qs::to_string(&req)?))
            .body(Body::empty())?;
        let body = client.request(req).await?.into_body();
        let resp: Resp = bincode::deserialize(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }

    #[allow(dead_code)]
    async fn bincode_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
    ) -> Result<Resp, NodeError> {
        let client = Client::new();
        let req = Request::builder()
            .method(Method::POST)
            .uri(&addr)
            .header("content-type", "application/octet-stream")
            .body(Body::from(bincode::serialize(&req)?))?;
        let body = client.request(req).await?.into_body();
        let resp: Resp = bincode::deserialize(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }

    async fn json_post<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
    ) -> Result<Resp, NodeError> {
        let client = Client::new();
        let req = Request::builder()
            .method(Method::POST)
            .uri(&addr)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&req)?))?;
        let body = client.request(req).await?.into_body();
        let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }

    #[allow(dead_code)]
    async fn json_get<Req: serde::Serialize, Resp: serde::de::DeserializeOwned>(
        &self,
        addr: String,
        req: Req,
    ) -> Result<Resp, NodeError> {
        let client = Client::new();
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("{}?{}", addr, serde_qs::to_string(&req)?))
            .body(Body::empty())?;
        let body = client.request(req).await?.into_body();
        let resp: Resp = serde_json::from_slice(&hyper::body::to_bytes(body).await?)?;
        Ok(resp)
    }
}

pub async fn group_request<F, R>(
    peers: &[PeerAddress],
    f: F,
) -> Vec<(PeerAddress, <R as futures::Future>::Output)>
where
    F: Fn(PeerAddress) -> R,
    R: futures::Future,
{
    peers
        .iter()
        .cloned()
        .zip(
            join_all(peers.iter().cloned().map(f).collect::<Vec<_>>())
                .await
                .into_iter(),
        )
        .collect()
}
