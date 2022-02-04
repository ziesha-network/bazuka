use futures::join;
use tokio::time::{sleep, Duration};
use warp::Filter;

async fn heartbeat() {
    loop {
        println!("Lub dub!");
        sleep(Duration::from_millis(1000)).await;
        let resp = reqwest::get("http://127.0.0.1:3030/hello/bazuka")
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        println!("{}", resp);
    }
}

#[tokio::main]
async fn main() {
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    let http_future = warp::serve(hello).run(([127, 0, 0, 1], 3030));

    let heartbeat_future = heartbeat();

    join!(http_future, heartbeat_future);
}
