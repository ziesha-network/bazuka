use std::collections::HashMap;
use std::sync::Mutex;
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
    peers: Mutex<HashMap<PeerId, PeerInfo>>,
}

impl Node {
    pub fn new() -> Node {
        Node {
            peers: Mutex::new(HashMap::new()),
        }
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
        }
    }

    async fn server(&self) -> Result<(), NodeError> {
        let peers = warp::path!("peers").map(|| warp::reply::json(&["a", "b", "c"]));

        warp::serve(peers).run(([127, 0, 0, 1], 3030)).await;

        Ok(())
    }

    pub async fn run(&'static self) -> Result<(), NodeError> {
        let server_future = self.server();
        let heartbeat_future = self.heartbeat();

        try_join!(server_future, heartbeat_future)?;

        Ok(())
    }
}
