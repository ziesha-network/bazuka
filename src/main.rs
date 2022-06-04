#[macro_use]
extern crate lazy_static;

#[cfg(feature = "node")]
use {
    bazuka::blockchain::KvStoreChain,
    bazuka::db::LevelDbKvStore,
    bazuka::node::{node_create, IncomingRequest, NodeError, OutgoingRequest, PeerAddress},
    bazuka::wallet::Wallet,
    colored::Colorize,
    hyper::server::conn::AddrStream,
    hyper::service::{make_service_fn, service_fn},
    hyper::{Body, Client, Request, Response, Server},
    std::net::SocketAddr,
    std::path::{Path, PathBuf},
    std::sync::Arc,
    structopt::StructOpt,
    tokio::sync::mpsc,
    tokio::try_join,
};

use bazuka::config::genesis;
#[cfg(not(feature = "node"))]
use {
    bazuka::blockchain::Blockchain, bazuka::blockchain::KvStoreChain, bazuka::core::Address,
    bazuka::db::RamKvStore, bazuka::wallet::Wallet,
};

#[cfg(feature = "node")]
#[derive(StructOpt)]
#[structopt(name = "Bazuka!", about = "Node software for Zeeka Network")]
struct NodeOptions {
    #[structopt(long)]
    listen: Option<SocketAddr>,
    #[structopt(long)]
    external: Option<SocketAddr>,
    #[structopt(long, parse(from_os_str))]
    db: Option<PathBuf>,
    #[structopt(long)]
    bootstrap: Vec<String>,
}

#[cfg(not(tarpaulin_include))]
lazy_static! {
    static ref WALLET: Wallet = Wallet::new(b"random seed".to_vec());
}

#[cfg(not(tarpaulin_include))]
#[cfg(feature = "node")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    env_logger::init();

    let public_ip = bazuka::node::upnp::get_public_ip().await;

    const DEFAULT_PORT: u16 = 3030;

    let opts = NodeOptions::from_args();

    let listen = opts
        .listen
        .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)));
    let address = PeerAddress(
        opts.external
            .unwrap_or_else(|| SocketAddr::from((public_ip.unwrap(), DEFAULT_PORT))),
    );

    println!(
        "{} v{}",
        "Bazuka!".bright_green(),
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("{} {}", "Listening:".bright_yellow(), listen);
    println!("{} {}", "Internet endpoint:".bright_yellow(), address);

    let (inc_send, inc_recv) = mpsc::unbounded_channel::<IncomingRequest>();
    let (out_send, mut out_recv) = mpsc::unbounded_channel::<OutgoingRequest>();

    // Use hardcoded seed bootstrap nodes if none provided via cli opts
    let bootstrap_nodes = {
        match opts.bootstrap.len() {
            0 => bazuka::node::seeds::seed_bootstrap_nodes(),
            _ => opts
                .bootstrap
                .clone()
                .into_iter()
                .map(|b| PeerAddress(b.parse().unwrap()))
                .collect(),
        }
    };

    // Async loop that is responsible for answering external requests and gathering
    // data from external world through a heartbeat loop.
    let node = node_create(
        address,
        bootstrap_nodes,
        KvStoreChain::new(
            LevelDbKvStore::new(
                &opts
                    .db
                    .unwrap_or_else(|| home::home_dir().unwrap().join(Path::new(".bazuka"))),
                64,
            )
            .unwrap(),
            genesis::get_genesis_block(),
        )
        .unwrap(),
        0,
        Some(WALLET.clone()),
        inc_recv,
        out_send,
    );

    // Async loop that is responsible for getting incoming HTTP requests through a
    // socket and redirecting it to the node channels.
    let server_loop = async {
        let arc_inc_send = Arc::new(inc_send);
        Server::bind(&listen)
            .serve(make_service_fn(|conn: &AddrStream| {
                let client = conn.remote_addr();
                let arc_inc_send = Arc::clone(&arc_inc_send);
                async move {
                    Ok::<_, NodeError>(service_fn(move |req: Request<Body>| {
                        let arc_inc_send = Arc::clone(&arc_inc_send);
                        async move {
                            let (resp_snd, mut resp_rcv) =
                                mpsc::channel::<Result<Response<Body>, NodeError>>(1);
                            let req = IncomingRequest {
                                socket_addr: client,
                                body: req,
                                resp: resp_snd,
                            };
                            arc_inc_send
                                .send(req)
                                .map_err(|_| NodeError::NotListeningError)?;
                            resp_rcv.recv().await.ok_or(NodeError::NotAnsweringError)?
                        }
                    }))
                }
            }))
            .await?;
        Ok::<(), NodeError>(())
    };

    // Async loop that is responsible for redirecting node requests from its outgoing
    // channel to the Internet and piping back the responses.
    let client_loop = async {
        while let Some(req) = out_recv.recv().await {
            let resp = async {
                let client = Client::new();
                let resp = client.request(req.body).await?;
                Ok::<_, NodeError>(resp)
            }
            .await;
            if let Err(e) = req.resp.send(resp).await {
                log::error!("Node not listening to its HTTP request answer: {}", e);
            }
        }
        Ok::<(), NodeError>(())
    };

    try_join!(server_loop, client_loop, node).unwrap();

    Ok(())
}

#[cfg(not(tarpaulin_include))]
#[cfg(not(feature = "node"))]
fn main() {
    env_logger::init();

    let mut genesis_block = genesis::get_genesis_block();
    genesis_block.block.header.proof_of_work.target = 0x00ffffff;

    let mut chain = KvStoreChain::new(RamKvStore::new(), genesis_block).unwrap();

    let mut nonce = 1;

    let abc = Wallet::new(Vec::from("ABC"));

    loop {
        log::info!("Creating txs...");
        let mut txs = Vec::new();
        for _ in 0..500 {
            txs.push(abc.create_transaction(Address::Treasury, 0, 0, nonce));
            nonce += 1;
        }

        log::info!("Creating block...");
        let blk = chain.draft_block(0, &txs, &WALLET).unwrap().block;

        log::info!("Applying block ({} txs)...", blk.body.len());
        chain.extend(chain.get_height().unwrap(), &[blk]).unwrap();
    }
}
