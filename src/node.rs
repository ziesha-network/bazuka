use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use tokio::try_join;
use warp::Filter;

#[derive(Debug)]
pub enum NodeError {
    ClientError(reqwest::Error),
}

impl From<reqwest::Error> for NodeError {
    fn from(e: reqwest::Error) -> Self {
        Self::ClientError(e)
    }
}

pub type PeerId = String;
pub struct PeerInfo {
    pub last_seen: u64,
}

pub struct Node {
    peers: Arc<Mutex<HashMap<PeerId, PeerInfo>>>,
}

#[derive(Deserialize, Serialize)]
pub struct PeerPostReq {
    address: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PeerPostResp {
    ok: bool,
}

impl Node {
    pub fn new() -> Node {
        Node {
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn introduce(&self, addr: &str) -> Result<PeerPostResp, NodeError> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/peers", addr))
            .json(&PeerPostReq {
                address: "hahaha".to_string(),
            })
            .send()
            .await?
            .json::<PeerPostResp>()
            .await?;
        Ok(resp)
    }

    async fn request_peers(&self, addr: &str) -> Result<Vec<String>, NodeError> {
        let resp = reqwest::get(format!("{}/peers", addr))
            .await?
            .json::<Vec<String>>()
            .await?;
        Ok(resp)
    }

    async fn heartbeat(&self) -> Result<(), NodeError> {
        loop {
            println!("Lub dub!");
            sleep(Duration::from_millis(1000)).await;
            let peers = self.request_peers("http://127.0.0.1:3030").await?;
            println!("Peers: {:?}", peers);
            let result = self.introduce("http://127.0.0.1:3030").await?;
            println!("Introduced! Result: {:?}", result);
        }
    }

    async fn server(&'static self) -> Result<(), NodeError> {
        let peers_arc_get = Arc::clone(&self.peers);
        let peers_get = warp::path("peers").and(warp::get()).map(move || {
            let peers = peers_arc_get
                .lock()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<String>>();
            warp::reply::json(&peers)
        });

        let peers_arc_post = Arc::clone(&self.peers);
        let peers_post = warp::path("peers")
            .and(warp::post())
            .and(warp::body::json())
            .map(move |body: PeerPostReq| {
                let mut peers = peers_arc_post.lock().unwrap();
                peers
                    .entry(body.address)
                    .or_insert(PeerInfo { last_seen: 0 });

                warp::reply::json(&PeerPostResp { ok: true })
            });

        let routes = peers_get.or(peers_post);
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

        Ok(())
    }

    pub async fn run(&'static self) -> Result<(), NodeError> {
        let server_future = self.server();
        let heartbeat_future = self.heartbeat();

        try_join!(server_future, heartbeat_future)?;

        Ok(())
    }
}
