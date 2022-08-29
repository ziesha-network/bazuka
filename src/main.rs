use bazuka::wallet::Wallet;

#[cfg(not(any(feature = "node", feature = "client")))]
use {
    bazuka::blockchain::{Blockchain, KvStoreChain, TransactionStats},
    bazuka::config,
    bazuka::core::{Address, Money},
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
    hyper::{Body, Client, Request, Response, Server, StatusCode},
    std::path::{Path, PathBuf},
    std::sync::Arc,
    tokio::sync::mpsc,
    tokio::try_join,
};

#[cfg(feature = "client")]
use {
    bazuka::client::{BazukaClient, NodeError},
    bazuka::core::{ContractId, Money, Signer, ZkSigner},
    bazuka::crypto::{SignatureScheme, ZkSignatureScheme},
    serde::{Deserialize, Serialize},
    std::net::SocketAddr,
    structopt::StructOpt,
};

#[cfg(feature = "client")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct BazukaConfig {
    seed: String,
    node: SocketAddr,
    network: String,
}

#[derive(StructOpt)]
#[cfg(feature = "client")]
#[structopt(name = "Bazuka!", about = "Node software for Zeeka Network")]
enum CliOptions {
    #[cfg(not(feature = "client"))]
    Init,
    #[cfg(feature = "client")]
    /// Initialize node/wallet
    Init {
        #[structopt(long)]
        seed: String,
        #[structopt(long)]
        node: SocketAddr,
        #[structopt(long, default_value = "mainnet")]
        network: String,
    },
    #[cfg(not(feature = "node"))]
    Node,
    #[cfg(feature = "node")]
    /// Run node
    Node {
        #[structopt(long)]
        listen: Option<SocketAddr>,
        #[structopt(long)]
        external: Option<SocketAddr>,
        #[structopt(long, parse(from_os_str))]
        db: Option<PathBuf>,
        #[structopt(long)]
        bootstrap: Vec<String>,
        #[structopt(long, default_value = "mainnet")]
        network: String,
    },
    /// Get status of a node
    Status {},
    /// Deposit funds to a Zero-Contract
    Deposit {
        #[structopt(long)]
        contract: String,
        #[structopt(long)]
        index: u32,
        #[structopt(long)]
        amount: Money,
        #[structopt(long, default_value = "0")]
        fee: Money,
    },
    /// Withdraw funds from a Zero-Contract
    Withdraw {
        #[structopt(long)]
        contract: String,
        #[structopt(long)]
        index: u32,
        #[structopt(long)]
        amount: Money,
        #[structopt(long, default_value = "0")]
        fee: Money,
    },
    /// Send funds through a regular-transaction
    Rsend {
        #[structopt(long)]
        to: String,
        #[structopt(long)]
        amount: Money,
        #[structopt(long, default_value = "0")]
        fee: Money,
    },
    /// Send funds through a zero-transaction
    Zsend {
        #[structopt(long)]
        from_index: u32,
        #[structopt(long)]
        to_index: u32,
        #[structopt(long)]
        to: String,
        #[structopt(long)]
        amount: Money,
        #[structopt(long, default_value = "0")]
        fee: Money,
    },
}

#[cfg(feature = "node")]
async fn run_node(
    bazuka_config: BazukaConfig,
    listen: Option<SocketAddr>,
    external: Option<SocketAddr>,
    db: Option<PathBuf>,
    bootstrap: Vec<String>,
    network: String,
) -> Result<(), NodeError> {
    let (pub_key, priv_key) = Signer::generate_keys(&bazuka_config.seed.as_bytes());

    let public_ip = bazuka::node::upnp::get_public_ip().await;

    const DEFAULT_PORT: u16 = 8765;

    let listen = listen.unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)));
    let address = PeerAddress(
        external.unwrap_or_else(|| SocketAddr::from((public_ip.unwrap(), DEFAULT_PORT))),
    );

    let wallet = Wallet::new(bazuka_config.seed.as_bytes().to_vec());

    println!(
        "{} v{}",
        "Bazuka!".bright_green(),
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("{} {}", "Listening:".bright_yellow(), listen);
    println!("{} {}", "Internet endpoint:".bright_yellow(), address);
    println!("{} {}", "Peer public-key:".bright_yellow(), pub_key);

    println!(
        "{} {}",
        "Wallet address:".bright_yellow(),
        wallet.get_address()
    );
    println!(
        "{} {}",
        "Wallet zk address:".bright_yellow(),
        wallet.get_zk_address()
    );

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
        match network.as_ref() {
            "debug" => config::node::get_debug_options(),
            "chaos" => config::node::get_chaos_options(),
            "mainnet" => config::node::get_mainnet_options(),
            _ => panic!("Network is not supported!"),
        },
        address,
        priv_key,
        bootstrap_nodes,
        KvStoreChain::new(
            LevelDbKvStore::new(&bazuka_dir, 64).unwrap(),
            config::blockchain::get_blockchain_config(),
        )
        .unwrap(),
        0,
        Some(wallet),
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
                            Ok::<Response<Body>, NodeError>(
                                match resp_rcv.recv().await.ok_or(NodeError::NotAnsweringError)? {
                                    Ok(resp) => resp,
                                    Err(e) => {
                                        let mut resp =
                                            Response::new(Body::from(format!("Error: {}", e)));
                                        *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                        resp
                                    }
                                },
                            )
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
            tokio::spawn(async move {
                let resp = async {
                    let client = Client::new();
                    let resp = client.request(req.body).await?;
                    Ok::<_, NodeError>(resp)
                }
                .await;
                if let Err(e) = req.resp.send(resp).await {
                    log::debug!("Node not listening to its HTTP request answer: {}", e);
                }
            });
        }
        Ok::<(), NodeError>(())
    };

    try_join!(server_loop, client_loop, node).unwrap();

    Ok(())
}

#[cfg(not(tarpaulin_include))]
#[cfg(feature = "client")]
async fn deposit_withdraw(
    conf: BazukaConfig,
    mpn_contract_id: ContractId,
    contract: String,
    index: u32,
    amount: Money,
    fee: Money,
    withdraw: bool,
) -> Result<(), NodeError> {
    let sk = Signer::generate_keys(conf.seed.as_bytes()).1; // Secret-key of client, not wallet!
    let wallet = Wallet::new(conf.seed.as_bytes().to_vec());
    let (req_loop, client) = BazukaClient::connect(sk, PeerAddress(conf.node), conf.network);
    try_join!(
        async move {
            let acc = client.get_account(wallet.get_address()).await?.account;
            let pay = wallet.pay_contract(
                if contract == "mpn" {
                    mpn_contract_id
                } else {
                    contract.parse().unwrap()
                },
                index,
                acc.nonce + 1,
                amount,
                fee,
                withdraw,
            );
            println!("{:#?}", client.transact_contract_payment(pay).await?);
            println!("{:#?}", client.get_zero_mempool().await?);
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();

    Ok(())
}

#[cfg(not(tarpaulin_include))]
#[cfg(feature = "client")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    env_logger::init();

    let opts = CliOptions::from_args();

    let conf_path = home::home_dir().unwrap().join(Path::new(".bazuka.yaml"));
    let conf: Option<BazukaConfig> = std::fs::File::open(conf_path.clone())
        .ok()
        .map(|f| serde_yaml::from_reader(f).unwrap());

    let mpn_contract_id = config::blockchain::get_blockchain_config().mpn_contract_id;

    match opts {
        #[cfg(feature = "node")]
        CliOptions::Node {
            listen,
            external,
            db,
            bootstrap,
            network,
        } => {
            let conf = conf.expect("Bazuka is not initialized!");
            run_node(conf.clone(), listen, external, db, bootstrap, network).await?;
        }
        #[cfg(not(feature = "node"))]
        CliOptions::Node { .. } => {
            println!("Node feature not turned on!");
        }
        #[cfg(feature = "client")]
        CliOptions::Init {
            seed,
            node,
            network,
        } => {
            if conf.is_none() {
                std::fs::write(
                    conf_path,
                    serde_yaml::to_string(&BazukaConfig {
                        seed,
                        node,
                        network,
                    })
                    .unwrap(),
                )
                .unwrap();
            } else {
                println!("Bazuka is already initialized!");
            }
        }
        #[cfg(not(feature = "client"))]
        CliOptions::Init { .. } => {
            println!("Client feature not turned on!");
        }
        CliOptions::Status {} => {
            let conf = conf.expect("Bazuka is not initialized!");
            let sk = Signer::generate_keys(conf.seed.as_bytes()).1; // Secret-key of client, not wallet!
            let (req_loop, client) =
                BazukaClient::connect(sk, PeerAddress(conf.node), conf.network);
            try_join!(
                async move {
                    println!("{:#?}", client.stats().await?);
                    Ok::<(), NodeError>(())
                },
                req_loop
            )
            .unwrap();
        }
        CliOptions::Deposit {
            contract,
            index,
            amount,
            fee,
        } => {
            let conf = conf.expect("Bazuka is not initialized!");
            deposit_withdraw(conf, mpn_contract_id, contract, index, amount, fee, false).await?;
        }
        CliOptions::Withdraw {
            contract,
            index,
            amount,
            fee,
        } => {
            let conf = conf.expect("Bazuka is not initialized!");
            deposit_withdraw(conf, mpn_contract_id, contract, index, amount, fee, true).await?;
        }
        CliOptions::Rsend { to, amount, fee } => {
            let conf = conf.expect("Bazuka is not initialized!");
            let sk = Signer::generate_keys(conf.seed.as_bytes()).1; // Secret-key of client, not wallet!
            let wallet = Wallet::new(conf.seed.as_bytes().to_vec());
            let (req_loop, client) =
                BazukaClient::connect(sk, PeerAddress(conf.node), conf.network);
            try_join!(
                async move {
                    let acc = client.get_account(wallet.get_address()).await?.account;
                    let tx =
                        wallet.create_transaction(to.parse().unwrap(), amount, fee, acc.nonce + 1);
                    println!("{:#?}", client.transact(tx).await?);
                    Ok::<(), NodeError>(())
                },
                req_loop
            )
            .unwrap();
        }
        CliOptions::Zsend {
            from_index,
            to_index,
            to,
            amount,
            fee,
        } => {
            let conf = conf.expect("Bazuka is not initialized!");
            let sk = Signer::generate_keys(conf.seed.as_bytes()).1; // Secret-key of client, not wallet!
            let wallet = Wallet::new(conf.seed.as_bytes().to_vec());
            let (req_loop, client) =
                BazukaClient::connect(sk, PeerAddress(conf.node), conf.network);
            try_join!(
                async move {
                    let to: <ZkSigner as ZkSignatureScheme>::Pub = to.parse().unwrap();
                    let acc = client.get_mpn_account(from_index).await?.account;
                    let tx = wallet
                        .create_mpn_transaction(from_index, to_index, to, amount, fee, acc.nonce);
                    println!("{:#?}", client.zero_transact(tx).await?);
                    Ok::<(), NodeError>(())
                },
                req_loop
            )
            .unwrap();
        }
    }

    Ok(())
}

#[cfg(not(feature = "client"))]
fn main() {
    env_logger::init();

    let mut conf = config::blockchain::get_blockchain_config();
    conf.genesis.block.header.proof_of_work.target = bazuka::consensus::pow::Difficulty(0x00ffffff);

    let mut chain = KvStoreChain::new(RamKvStore::new(), conf).unwrap();

    let mut nonce = 1;

    let abc = Wallet::new(Vec::from("ABC"));

    loop {
        log::info!("Creating txs...");
        let mut txs = HashMap::new();
        for _ in 0..7400 {
            txs.insert(
                abc.create_transaction(Address::Treasury, Money(0), Money(0), nonce),
                TransactionStats { first_seen: 0 },
            );
            nonce += 1;
        }

        log::info!("Creating block...");
        let blk = chain
            .draft_block(0, &mut txs, &abc, true)
            .unwrap()
            .unwrap()
            .block;

        log::info!("Applying block ({} txs)...", blk.body.len());
        chain.extend(chain.get_height().unwrap(), &[blk]).unwrap();
    }
}
