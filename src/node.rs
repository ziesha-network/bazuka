use super::blockchain::Blockchain;
use super::messages::*;
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

pub struct Node<B: Blockchain> {
    blockchain: B,
    peers: Arc<Mutex<HashMap<PeerAddress, PeerInfo>>>,
}

impl<B: Blockchain> Node<B> {
    pub fn new(blockchain: B) -> Node<B> {
        Node {
            blockchain,
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn introduce(&self, addr: &str) -> Result<PostPeerResponse, NodeError> {
        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/peers", addr))
            .json(&PostPeerRequest {
                address: "hahaha".to_string(),
                info: PeerInfo {
                    last_seen: 0,
                    height: self.blockchain.get_height(),
                },
            })
            .send()
            .await?
            .json::<PostPeerResponse>()
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
            .map(move |body: PostPeerRequest| {
                let mut peers = peers_arc_post.lock().unwrap();
                peers.entry(body.address).or_insert(body.info);

                warp::reply::json(&PostPeerResponse {})
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
