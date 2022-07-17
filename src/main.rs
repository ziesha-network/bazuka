#[macro_use]
extern crate lazy_static;

use bazuka::wallet::Wallet;

#[cfg(not(any(feature = "node", feature = "client")))]
use {
    bazuka::blockchain::{Blockchain, KvStoreChain, TransactionStats},
    bazuka::config,
    bazuka::core::Address,
    bazuka::db::RamKvStore,
    std::collections::HashMap,
};

#[cfg(feature = "node")]
use {
    bazuka::blockchain::KvStoreChain,
    bazuka::client::{NodeRequest, PeerAddress},
    bazuka::config,
    bazuka::db::LevelDbKvStore,
    bazuka::node::node_create,
    colored::Colorize,
    hyper::server::conn::AddrStream,
    hyper::service::{make_service_fn, service_fn},
    hyper::{Body, Client, Request, Response, Server},
    std::path::{Path, PathBuf},
    std::sync::Arc,
    tokio::sync::mpsc,
    tokio::try_join,
};

#[cfg(feature = "client")]
use {
    bazuka::client::{BazukaClient, NodeError},
    bazuka::core::Signer,
    bazuka::crypto::SignatureScheme,
    std::net::SocketAddr,
    structopt::StructOpt,
};

#[derive(StructOpt)]
#[cfg(feature = "client")]
#[structopt(name = "Bazuka!", about = "Node software for Zeeka Network")]
enum CliOptions {
    #[cfg(not(feature = "node"))]
    Node,
    #[cfg(feature = "node")]
    Node {
        #[structopt(long)]
        listen: Option<SocketAddr>,
        #[structopt(long)]
        external: Option<SocketAddr>,
        #[structopt(long, parse(from_os_str))]
        db: Option<PathBuf>,
        #[structopt(long)]
        bootstrap: Vec<String>,
    },
    Status {
        #[structopt(long)]
        node: SocketAddr,
    },
}

#[cfg(not(tarpaulin_include))]
lazy_static! {
    static ref WALLET: Wallet = Wallet::new(b"random seed".to_vec());
}

#[cfg(feature = "node")]
async fn run_node(
    listen: Option<SocketAddr>,
    external: Option<SocketAddr>,
    db: Option<PathBuf>,
    bootstrap: Vec<String>,
) -> Result<(), NodeError> {
    let (pub_key, priv_key) = Signer::generate_keys(b"mynode");

    let public_ip = bazuka::node::upnp::get_public_ip().await;

    const DEFAULT_PORT: u16 = 3030;

    let listen = listen.unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)));
    let address = PeerAddress(
        external.unwrap_or_else(|| SocketAddr::from((public_ip.unwrap(), DEFAULT_PORT))),
    );

    println!(
        "{} v{}",
        "Bazuka!".bright_green(),
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("{} {}", "Listening:".bright_yellow(), listen);
    println!("{} {}", "Internet endpoint:".bright_yellow(), address);
    println!("{} {}", "Peer public-key:".bright_yellow(), pub_key);

    let (inc_send, inc_recv) = mpsc::unbounded_channel::<NodeRequest>();
    let (out_send, mut out_recv) = mpsc::unbounded_channel::<NodeRequest>();

    // Use hardcoded seed bootstrap nodes if none provided via cli opts
    let bootstrap_nodes = {
        match bootstrap.len() {
            0 => bazuka::node::seeds::seed_bootstrap_nodes(),
            _ => bootstrap
                .clone()
                .into_iter()
                .map(|b| PeerAddress(b.parse().unwrap()))
                .collect(),
        }
    };

    let bazuka_dir = db.unwrap_or_else(|| home::home_dir().unwrap().join(Path::new(".bazuka")));
    // Async loop that is responsible for answering external requests and gathering
    // data from external world through a heartbeat loop.
    let node = node_create(
        config::node::get_node_options(),
        address,
        priv_key,
        bootstrap_nodes,
        KvStoreChain::new(
            LevelDbKvStore::new(&bazuka_dir, 64).unwrap(),
            config::blockchain::get_blockchain_config(),
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
                            let req = NodeRequest {
                                socket_addr: Some(client),
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
#[cfg(feature = "client")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    env_logger::init();

    let opts = CliOptions::from_args();
    match opts {
        #[cfg(feature = "node")]
        CliOptions::Node {
            listen,
            external,
            db,
            bootstrap,
        } => {
            run_node(listen, external, db, bootstrap).await?;
        }
        #[cfg(not(feature = "node"))]
        CliOptions::Node { .. } => {
            panic!("Not feature not turned on!");
        }
        CliOptions::Status { node } => {
            let sk = Signer::generate_keys(b"smth").1; // Secret-key of client, not wallet!
            let (req_loop, client) = BazukaClient::connect(sk, PeerAddress(node));
            let main = async {
                println!("{:#?}", client.stats().await?);
                Ok::<(), NodeError>(())
            };
            try_join!(main, req_loop).unwrap();
        }
    }

    Ok(())
}

#[cfg(not(feature = "client"))]
fn main() {
    env_logger::init();

    let mut conf = config::blockchain::get_blockchain_config();
    conf.genesis.block.header.proof_of_work.target = 0x00ffffff;

    let mut chain = KvStoreChain::new(RamKvStore::new(), conf).unwrap();

    let mut nonce = 1;

    let abc = Wallet::new(Vec::from("ABC"));

    loop {
        log::info!("Creating txs...");
        let mut txs = HashMap::new();
        for _ in 0..7400 {
            txs.insert(
                abc.create_transaction(Address::Treasury, 0, 0, nonce),
                TransactionStats { first_seen: 0 },
            );
            nonce += 1;
        }

        log::info!("Creating block...");
        let blk = chain.draft_block(0, &mut txs, &WALLET, true).unwrap().block;

        log::info!("Applying block ({} txs)...", blk.body.len());
        chain.extend(chain.get_height().unwrap(), &[blk]).unwrap();
    }
}
