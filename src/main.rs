use tokio::time::{sleep, Duration};
use tokio::try_join;
use warp::Filter;

async fn request_peers(addr: &str) -> Result<Vec<String>, reqwest::Error> {
    let resp = reqwest::get(format!("{}/peers", addr))
        .await?
        .json::<Vec<String>>()
        .await?;
    Ok(resp)
}

async fn heartbeat() -> Result<(), reqwest::Error> {
    loop {
        println!("Lub dub!");
        sleep(Duration::from_millis(1000)).await;
        let peers = request_peers("http://127.0.0.1:3030").await?;
        println!("Peers: {:?}", peers);
    }
}

async fn server() -> Result<(), reqwest::Error> {
    let peers = warp::path!("peers").map(|| warp::reply::json(&["a", "b", "c"]));

    warp::serve(peers).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}

#[tokio::main]
async fn main() {
    let server_future = server();
    let heartbeat_future = heartbeat();

    if let Err(e) = try_join!(server_future, heartbeat_future) {
        println!("Node crashed! Error: {}", e);
    }
}
