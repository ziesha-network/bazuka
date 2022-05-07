#[macro_use]
extern crate lazy_static;

#[cfg(feature = "node")]
use {
    bazuka::blockchain::KvStoreChain,
    bazuka::db::{LevelDbKvStore, LruCacheKvStore},
    bazuka::node::{
        node_create, node_request, IncomingRequest, NodeError, OutgoingRequest, PeerAddress,
    },
    bazuka::wallet::Wallet,
    hyper::server::conn::AddrStream,
    hyper::service::{make_service_fn, service_fn},
    hyper::{Body, Client, Request, Server},
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
    bazuka::blockchain::Blockchain,
    bazuka::blockchain::KvStoreChain,
    bazuka::core::Address,
    bazuka::core::{Signature, Transaction, TransactionData},
    bazuka::db::RamKvStore,
    bazuka::wallet::Wallet,
};

#[cfg(feature = "node")]
#[derive(Debug, Clone, StructOpt)]
#[structopt(name = "Options", about = "Bazuka node software options")]
struct NodeOptions {
    #[structopt(long)]
    host: Option<String>,
    #[structopt(long)]
    port: Option<u16>,
    #[structopt(long, parse(from_os_str))]
    db: Option<PathBuf>,
    #[structopt(long)]
    bootstrap: Vec<String>,
}

lazy_static! {
    static ref WALLET: Wallet = Wallet::new(b"random seed".to_vec());
}

#[cfg(feature = "node")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    println!(
        "Public Ip: {:?}",
        bazuka::node::upnp::get_public_ip().await.ok()
    );

    let (inc_send, inc_recv) = mpsc::unbounded_channel::<IncomingRequest>();
    let (out_send, mut out_recv) = mpsc::unbounded_channel::<OutgoingRequest>();

    let opts = NodeOptions::from_args();
    let address = PeerAddress(
        opts.host
            .unwrap_or_else(|| "127.0.0.1".to_string())
            .parse()
            .unwrap(),
        opts.port.unwrap_or(3030),
    );
    let node_fut = node_create(
        address,
        opts.bootstrap
            .clone()
            .into_iter()
            .map(|b| {
                let mut parts = b.splitn(2, ':');
                let host = parts.next().unwrap();
                let port = parts.next().unwrap();
                PeerAddress(host.parse().unwrap(), port.parse().unwrap())
            })
            .collect(),
        KvStoreChain::new(
            LruCacheKvStore::new(
                LevelDbKvStore::new(
                    &opts
                        .db
                        .unwrap_or_else(|| home::home_dir().unwrap().join(Path::new(".bazuka"))),
                )
                .unwrap(),
                64,
            ),
            genesis::get_genesis_block(),
        )
        .unwrap(),
        Some(WALLET.clone()),
        inc_recv,
        out_send,
    );

    let arc_req_send = Arc::new(inc_send);

    let addr = SocketAddr::from(([0, 0, 0, 0], address.1));
    let make_svc = make_service_fn(|conn: &AddrStream| {
        let client = conn.remote_addr();
        let arc_req_send = Arc::clone(&arc_req_send);
        async move {
            Ok::<_, NodeError>(service_fn(move |req: Request<Body>| {
                let arc_req_send = Arc::clone(&arc_req_send);
                async move { node_request(arc_req_send, client, req).await }
            }))
        }
    });
    let server_future = async {
        Server::bind(&addr).serve(make_svc).await?;
        Ok::<(), NodeError>(())
    };
    let net_loop = async {
        loop {
            if let Some(req) = out_recv.recv().await {
                let client = Client::new();
                let resp = client
                    .request(req.body)
                    .await
                    .map_err(|h| NodeError::ServerError(h));
                if req.resp.send(resp).await.is_err() {
                    println!("Something bad happened!");
                }
            } else {
                break;
            }
        }
        Ok::<(), NodeError>(())
    };

    try_join!(server_future, node_fut, net_loop).unwrap();

    Ok(())
}

#[cfg(not(feature = "node"))]
fn main() {
    let genesis_block = genesis::get_genesis_block();
    let mut chain = KvStoreChain::new(RamKvStore::new(), genesis_block).unwrap();

    println!("Bazuka!");
    println!("Your address is: {}", WALLET.get_address());

    #[cfg(feature = "pow")]
    {
        println!("Chain power: {}", chain.get_power().unwrap());
    }

    chain
        .draft_block(
            0,
            &vec![Transaction {
                src: Address::Treasury,
                data: TransactionData::RegularSend {
                    dst: "0x215d9af3a1bfa2a87929b6e8265e95c61c36f91493f3dbd702215255f68742552"
                        .parse()
                        .unwrap(),
                    amount: 123,
                },
                nonce: 1,
                fee: 0,
                sig: Signature::Unsigned,
            }],
            &WALLET,
        )
        .unwrap();

    chain.rollback_block().unwrap();
    println!(
        "Balance: {:?}",
        chain.get_account(WALLET.get_address()).unwrap()
    );

    let tx = WALLET.create_transaction(Address::Treasury, 123, 0, 1);
    println!("Verify tx signature: {}", tx.verify_signature());
}
