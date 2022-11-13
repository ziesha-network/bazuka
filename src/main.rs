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
    bazuka::client::{messages::SocialProfiles, Limit, NodeRequest, PeerAddress},
    bazuka::common::*,
    bazuka::config,
    bazuka::db::LevelDbKvStore,
    bazuka::node::{node_create, Firewall},
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
    #[serde(default)]
    miner_token: String,
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
        #[structopt(long)]
        client_only: bool,
        #[structopt(long, parse(from_os_str))]
        db: Option<PathBuf>,
        #[structopt(long)]
        bootstrap: Vec<String>,
        #[structopt(long, default_value = "mainnet")]
        network: String,
        #[structopt(long)]
        discord_handle: Option<String>,
    },
    /// Get status of a node
    Status {},
    /// Get wallet info
    Wallet {},
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
    social_profiles: SocialProfiles,
    listen: Option<SocketAddr>,
    external: Option<SocketAddr>,
    client_only: bool,
    db: Option<PathBuf>,
    bootstrap: Vec<String>,
    network: String,
) -> Result<(), NodeError> {
    let (_pub_key, priv_key) = Signer::generate_keys(&bazuka_config.seed.as_bytes());

    const DEFAULT_PORT: u16 = 8765;

    let listen = listen.unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)));
    let address = if client_only {
        None
    } else {
        let public_ip = bazuka::node::upnp::get_public_ip().await;
        Some(PeerAddress(external.unwrap_or_else(|| {
            SocketAddr::from((public_ip.unwrap(), DEFAULT_PORT))
        })))
    };

    let wallet = Wallet::new(bazuka_config.seed.as_bytes().to_vec());

    println!(
        "{} v{}",
        "Bazuka!".bright_green(),
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("{} {}", "Listening:".bright_yellow(), listen);
    if let Some(addr) = &address {
        println!("{} {}", "Internet endpoint:".bright_yellow(), addr);
    }

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
    println!(
        "{} {}",
        "Miner token:".bright_yellow(),
        bazuka_config.miner_token
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

    // 60 request per minute / 4GB per 15min
    let firewall = Firewall::new(60, 4 * GB);

    // Async loop that is responsible for answering external requests and gathering
    // data from external world through a heartbeat loop.
    let node = node_create(
        config::node::get_node_options(),
        &network,
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
        social_profiles,
        inc_recv,
        out_send,
        Some(firewall),
        Some(bazuka_config.miner_token.clone()),
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
                                limit: Limit::default(),
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
                    let resp = if let Some(time_limit) = req.limit.time {
                        tokio::time::timeout(time_limit, client.request(req.body)).await?
                    } else {
                        client.request(req.body).await
                    }?;
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
    let (req_loop, client) = BazukaClient::connect(sk, PeerAddress(conf.node), conf.network, None);
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
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();

    Ok(())
}

fn generate_miner_token() -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect()
}

#[cfg(not(tarpaulin_include))]
#[cfg(feature = "client")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    let strs = [
        b"promote what daughter renew marine sausage debate sniff crew title regret gym artist nephew oxygen tortoise shop trust fluid rebuild hair swear coral unusual".to_vec(),
        b"0x756e8227e20c9d49e2128bed39520479732337090e534ed350461e89292e9dd0".to_vec(),
        b"zeekafaruk".to_vec(),
        b"fesyhejytdhrerhjdkjes57568rhd4".to_vec(),
        b"Ezagor".to_vec(),
        b"fgdsg546gtt6jtfjtyf465jhtj".to_vec(),
        b"[Dz3011helin520..]".to_vec(),
        b"vevivoseed".to_vec(),
        b"geshhtjutge4647hfhry".to_vec(),
        b"THIS_MY_ZEEKA_SEED_FOR_com_cn_org_xyz_BTC_ETH_EOS_19881001_Z02".to_vec(),
        b"htrdhgrdhjdrthrddghrebeshgy".to_vec(),
        b"pigeon spatial faculty analyst north people feel recycle render wear elder next".to_vec(),
        b"htrjjtf5789744674hjrjfjtfkrt57546".to_vec(),
        b"A_RANDOM_STRING_THAT_IS_LONG_ENOUGH_183264dfhejfjcgefjkef".to_vec(),
        b"asfdasdfsadflskdjafh7834h87fg7238473278f2gyeifbsadkuf3784437783478382".to_vec(),
        b"0x35a8e2b3dbf6c178c0563d829d922dc9e6d736e7489a15eb0fe99a09cf292332".to_vec(),
        b"ZhknPQ4cSoHnr0uppXrXfVF0V6NMIOvLAyBBb3ASMZff5Efwkhle1EjdG6PRVS44vP27xJ2cONNlqzxbcsIdGxeZaKSdBjZ6KgV".to_vec(),
    ];
    for s in strs {
        let wallet = Wallet::new(s);
        println!("{}", wallet.get_address());
    }

    env_logger::init();

    let opts = CliOptions::from_args();

    let conf_path = home::home_dir().unwrap().join(Path::new(".bazuka.yaml"));
    let mut conf: Option<BazukaConfig> = std::fs::File::open(conf_path.clone())
        .ok()
        .map(|f| serde_yaml::from_reader(f).unwrap());

    if let Some(ref mut conf) = &mut conf {
        if conf.miner_token.is_empty() {
            conf.miner_token = generate_miner_token();
        }
        std::fs::write(conf_path.clone(), serde_yaml::to_string(conf).unwrap()).unwrap();
    }

    let mpn_contract_id = config::blockchain::get_blockchain_config().mpn_contract_id;

    match opts {
        #[cfg(feature = "node")]
        CliOptions::Node {
            listen,
            external,
            db,
            bootstrap,
            network,
            discord_handle,
            client_only,
        } => {
            let conf = conf.expect("Bazuka is not initialized!");
            run_node(
                conf.clone(),
                SocialProfiles {
                    discord: discord_handle,
                },
                listen,
                external,
                client_only,
                db,
                bootstrap,
                network,
            )
            .await?;
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
            let miner_token = generate_miner_token();
            if conf.is_none() {
                std::fs::write(
                    conf_path,
                    serde_yaml::to_string(&BazukaConfig {
                        seed,
                        node,
                        network,
                        miner_token,
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
                BazukaClient::connect(sk, PeerAddress(conf.node), conf.network, None);
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
                BazukaClient::connect(sk, PeerAddress(conf.node), conf.network, None);
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
                BazukaClient::connect(sk, PeerAddress(conf.node), conf.network, None);
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
        CliOptions::Wallet {} => {
            let conf = conf.expect("Bazuka is not initialized!");
            let wallet = Wallet::new(conf.seed.as_bytes().to_vec());
            let sk = Signer::generate_keys(conf.seed.as_bytes()).1; // Secret-key of client, not wallet!

            println!(
                "{} {}",
                "Wallet address:".bright_yellow(),
                wallet.get_address()
            );
            println!(
                "{} {}",
                "Wallet zk-address:".bright_yellow(),
                wallet.get_zk_address()
            );

            let (req_loop, client) =
                BazukaClient::connect(sk, PeerAddress(conf.node), conf.network, None);
            try_join!(
                async move {
                    println!(
                        "{} {}",
                        "Balance:".bright_yellow(),
                        client
                            .get_account(wallet.get_address())
                            .await
                            .map(|resp| resp.account.balance.to_string())
                            .unwrap_or("Node not available!".into())
                    );
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
