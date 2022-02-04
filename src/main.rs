use futures::join;
use tokio::time::{sleep, Duration};
use warp::Filter;

async fn request_peers(addr: &str) -> Result<Vec<String>, reqwest::Error> {
    let resp = reqwest::get(format!("{}/peers", addr))
        .await?
        .json::<Vec<String>>()
        .await?;
    Ok(resp)
}

async fn heartbeat() {
    loop {
        println!("Lub dub!");
        sleep(Duration::from_millis(1000)).await;
        let peers = request_peers("http://127.0.0.1:3030").await.unwrap();
        println!("Peers: {:?}", peers);
    }
}

#[tokio::main]
async fn main() {
    let peers = warp::path!("peers").map(|| warp::reply::json(&["a", "b", "c"]));

    let http_future = warp::serve(peers).run(([127, 0, 0, 1], 3030));

    let heartbeat_future = heartbeat();

    join!(http_future, heartbeat_future);
}
