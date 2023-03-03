#[cfg(feature = "node")]
use {
    bazuka::blockchain::{Blockchain, KvStoreChain},
    bazuka::client::{messages::SocialProfiles, Limit, NodeRequest},
    bazuka::common::*,
    bazuka::db::{KvStore, LevelDbKvStore, ReadOnlyLevelDbKvStore},
    bazuka::node::{node_create, Firewall},
    hyper::server::conn::AddrStream,
    hyper::service::{make_service_fn, service_fn},
    hyper::{Body, Client, Request, Response, Server, StatusCode},
    std::sync::Arc,
    tokio::sync::mpsc,
};

#[cfg(feature = "client")]
use {
    bazuka::client::{BazukaClient, NodeError, PeerAddress},
    bazuka::config,
    bazuka::core::{
        Address, Amount, ChainSourcedTx, DelegateId, Money, MpnAddress, MpnSourcedTx, TokenId,
        ZieshaAddress,
    },
    bazuka::wallet::{TxBuilder, Wallet},
    colored::Colorize,
    rand::Rng,
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
    std::net::SocketAddr,
    std::path::{Path, PathBuf},
    structopt::StructOpt,
    tokio::try_join,
};

#[cfg(feature = "client")]
const DEFAULT_PORT: u16 = 8765;

#[cfg(feature = "client")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct BazukaConfig {
    listen: SocketAddr,
    external: PeerAddress,
    network: String,
    miner_token: String,
    bootstrap: Vec<PeerAddress>,
    db: PathBuf,
}

#[cfg(feature = "client")]
impl BazukaConfig {
    fn random_node(&self) -> PeerAddress {
        PeerAddress(SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)))
        /*self.bootstrap
        .choose(&mut rand::thread_rng())
        .unwrap_or(&PeerAddress(SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT))))
        .clone()*/
    }
}

#[derive(StructOpt)]
#[allow(clippy::large_enum_variant)]
#[cfg(feature = "client")]
enum WalletOptions {
    /// Add a new token to the wallet
    AddToken {
        #[structopt(long)]
        id: TokenId,
    },
    /// Creates a new token
    NewToken {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        name: String,
        #[structopt(long)]
        symbol: String,
        #[structopt(long)]
        supply: Amount,
        #[structopt(long, default_value = "0")]
        decimals: u8,
        #[structopt(long)]
        mintable: bool,
        #[structopt(long, default_value = "0")]
        fee: Amount,
    },
    /// Send money
    Send {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        from: ZieshaAddress,
        #[structopt(long)]
        to: ZieshaAddress,
        #[structopt(long)]
        token: Option<usize>,
        #[structopt(long)]
        amount: Amount,
        #[structopt(long, default_value = "0")]
        fee: Amount,
    },
    /// Register your validator
    RegisterValidator {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long, default_value = "0")]
        fee: Amount,
    },
    /// Delegate to a validator
    Delegate {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        to: Address,
        #[structopt(long)]
        amount: Amount,
        #[structopt(long)]
        since: Option<u32>,
        #[structopt(long)]
        count: u32,
        #[structopt(long, default_value = "0")]
        fee: Amount,
    },
    /// Reclaim funds inside an ended delegatation back to your account
    ReclaimDelegate {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        delegate_id: DelegateId,
        #[structopt(long, default_value = "0")]
        fee: Amount,
    },
    /// Resets wallet nonces
    Reset {},
    /// Get info and balances of the wallet
    Info {},
    /// Resend pending transactions
    ResendPending {
        #[structopt(long)]
        fill_gaps: bool,
        #[structopt(long)]
        shift: bool,
    },
}

#[derive(StructOpt)]
#[allow(clippy::large_enum_variant)]
#[cfg(feature = "node")]
enum NodeCliOptions {
    /// Start the node
    Start {
        #[structopt(long)]
        client_only: bool,
        #[structopt(long)]
        discord_handle: Option<String>,
    },
    /// Get status of a node
    Status {},
}

#[derive(StructOpt)]
#[allow(clippy::large_enum_variant)]
#[cfg(feature = "node")]
enum ChainCliOptions {
    /// Rollback the blockchain
    Rollback {},
    /// Query the underlying database
    DbQuery { prefix: String },
    /// Check health of the blockchain
    HealthCheck {},
}

#[derive(StructOpt)]
#[allow(clippy::large_enum_variant)]
#[cfg(feature = "client")]
#[structopt(name = "Bazuka!", about = "Node software for Ziesha Network")]
enum CliOptions {
    #[cfg(not(feature = "client"))]
    Init,
    #[cfg(feature = "client")]
    /// Initialize node/wallet
    Init {
        #[structopt(long, default_value = "mainnet")]
        network: String,
        #[structopt(long)]
        bootstrap: Vec<PeerAddress>,
        #[structopt(long)]
        mnemonic: Option<bip39::Mnemonic>,
        #[structopt(long)]
        listen: Option<SocketAddr>,
        #[structopt(long)]
        external: Option<PeerAddress>,
        #[structopt(long)]
        db: Option<PathBuf>,
    },

    #[cfg(feature = "node")]
    /// Node subcommand
    Node(NodeCliOptions),

    /// Wallet subcommand
    Wallet(WalletOptions),

    /// Chain subcommand
    Chain(ChainCliOptions),
}

#[cfg(feature = "node")]
async fn run_node(
    bazuka_config: BazukaConfig,
    wallet: Wallet,
    social_profiles: SocialProfiles,
    client_only: bool,
) -> Result<(), NodeError> {
    let address = if client_only {
        None
    } else {
        Some(bazuka_config.external)
    };

    let wallet = TxBuilder::new(&wallet.seed());

    println!(
        "{} v{}",
        "Bazuka!".bright_green(),
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("{} {}", "Listening:".bright_yellow(), bazuka_config.listen);
    if let Some(addr) = &address {
        println!("{} {}", "Internet endpoint:".bright_yellow(), addr);
    }
    println!("{} {}", "Network:".bright_yellow(), bazuka_config.network);

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

    let bootstrap_nodes = bazuka_config.bootstrap.clone();

    let bazuka_dir = bazuka_config.db.clone();

    // 60 request per minute / 4GB per 15min
    let firewall = Firewall::new(360, 4 * GB);

    // Async loop that is responsible for answering external requests and gathering
    // data from external world through a heartbeat loop.
    let node = node_create(
        config::node::get_node_options(),
        &bazuka_config.network,
        address,
        bootstrap_nodes,
        KvStoreChain::new(
            LevelDbKvStore::new(&bazuka_dir, 64).unwrap(),
            config::blockchain::get_blockchain_config(),
        )
        .unwrap(),
        0,
        wallet,
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
        Server::bind(&bazuka_config.listen)
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

#[cfg(feature = "client")]
fn generate_miner_token() -> String {
    use rand::distributions::Alphanumeric;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect()
}

#[cfg(not(tarpaulin_include))]
#[cfg(feature = "client")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    env_logger::init();

    let opts = CliOptions::from_args();

    let conf_path = home::home_dir().unwrap().join(Path::new(".bazuka.yaml"));
    let wallet_path = home::home_dir().unwrap().join(Path::new(".bazuka-wallet"));

    let mut conf: Option<BazukaConfig> = std::fs::File::open(conf_path.clone())
        .ok()
        .map(|f| serde_yaml::from_reader(f).unwrap());
    let wallet = Wallet::open(wallet_path.clone()).unwrap();

    if let Some(ref mut conf) = &mut conf {
        if conf.miner_token.is_empty() {
            conf.miner_token = generate_miner_token();
        }
        std::fs::write(conf_path.clone(), serde_yaml::to_string(conf).unwrap()).unwrap();
    }

    let mpn_contract_id = config::blockchain::get_blockchain_config().mpn_contract_id;
    let mpn_log4_account_capacity =
        config::blockchain::get_blockchain_config().mpn_log4_account_capacity;

    match opts {
        #[cfg(feature = "node")]
        CliOptions::Chain(chain_opts) => {
            let conf = conf.expect("Bazuka is not initialized!");
            match chain_opts {
                ChainCliOptions::Rollback {} => {
                    let mut chain = KvStoreChain::new(
                        LevelDbKvStore::new(&conf.db, 64).unwrap(),
                        config::blockchain::get_blockchain_config(),
                    )
                    .unwrap();
                    chain.rollback().unwrap();
                }
                ChainCliOptions::DbQuery { prefix } => {
                    let rdb = ReadOnlyLevelDbKvStore::read_only(&conf.db, 64).unwrap();
                    let db = rdb.snapshot();
                    for (k, v) in db.pairs(prefix.into()).unwrap().into_iter() {
                        println!("{} -> {}", k, v);
                    }
                }
                ChainCliOptions::HealthCheck {} => {
                    let rdb = ReadOnlyLevelDbKvStore::read_only(&conf.db, 64).unwrap();
                    let db = rdb.snapshot();
                    let chain =
                        KvStoreChain::new(db, config::blockchain::get_blockchain_config()).unwrap();
                    let mut fork = chain.fork_on_ram();
                    while fork.get_height().unwrap() != 0 {
                        fork.rollback().unwrap();
                    }
                    let rollback_validity_check = fork
                        .db()
                        .pairs("".into())
                        .unwrap()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .is_empty();
                    let mut sum_mpn: Amount = 0.into();
                    for mpn_acc in chain.get_mpn_accounts(0, 10000).unwrap() {
                        for money in mpn_acc.1.tokens.values() {
                            if money.token_id == TokenId::Ziesha {
                                sum_mpn += money.amount;
                            }
                        }
                    }
                    let mpn_contract_balance_check = sum_mpn
                        == chain
                            .get_contract_balance(mpn_contract_id, TokenId::Ziesha)
                            .unwrap();
                    let currency_in_circulation_check = chain.currency_in_circulation().unwrap()
                        == Amount::from(2000000000000000000);
                    println!(
                        "Rollback validity check: {}",
                        if rollback_validity_check {
                            "PASS".bright_green()
                        } else {
                            "FAIL".bright_red()
                        }
                    );
                    println!(
                        "MPN contract balance check: {}",
                        if mpn_contract_balance_check {
                            "PASS".bright_green()
                        } else {
                            "FAIL".bright_red()
                        }
                    );
                    println!(
                        "Currency in circulation check: {}",
                        if currency_in_circulation_check {
                            "PASS".bright_green()
                        } else {
                            "FAIL".bright_red()
                        }
                    );
                }
            }
        }
        #[cfg(feature = "node")]
        CliOptions::Node(node_opts) => match node_opts {
            NodeCliOptions::Start {
                discord_handle,
                client_only,
            } => {
                let conf = conf.expect("Bazuka is not initialized!");
                let wallet = wallet.expect("Wallet is not initialized!");
                run_node(
                    conf.clone(),
                    wallet.clone(),
                    SocialProfiles {
                        discord: discord_handle,
                    },
                    client_only,
                )
                .await?;
            }
            NodeCliOptions::Status {} => {
                let (conf, wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let wallet = TxBuilder::new(&wallet.seed());
                let (req_loop, client) = BazukaClient::connect(
                    wallet.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                try_join!(
                    async move {
                        println!("{:#?}", client.stats().await?);
                        Ok::<(), NodeError>(())
                    },
                    req_loop
                )
                .unwrap();
            }
        },
        #[cfg(feature = "client")]
        CliOptions::Init {
            network,
            bootstrap,
            mnemonic,
            external,
            listen,
            db,
        } => {
            if wallet.is_none() {
                let w = Wallet::create(&mut rand_mnemonic::thread_rng(), mnemonic);
                w.save(wallet_path).unwrap();
                println!("Wallet generated!");
                println!("{} {}", "Mnemonic phrase:".bright_yellow(), w.mnemonic());
                println!(
                    "{}",
                    "WRITE DOWN YOUR MNEMONIC PHRASE IN A SAFE PLACE!"
                        .italic()
                        .bold()
                        .bright_green()
                );
            } else {
                println!("Wallet is already initialized!");
            }

            if conf.is_none() {
                let miner_token = generate_miner_token();
                let public_ip = bazuka::client::utils::get_public_ip().await.unwrap();
                std::fs::write(
                    conf_path,
                    serde_yaml::to_string(&BazukaConfig {
                        network,
                        miner_token,
                        bootstrap,
                        listen: listen
                            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT))),
                        external: external.unwrap_or_else(|| {
                            PeerAddress(SocketAddr::from((public_ip, DEFAULT_PORT)))
                        }),
                        db: db.unwrap_or_else(|| {
                            home::home_dir().unwrap().join(Path::new(".bazuka"))
                        }),
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
        CliOptions::Wallet(wallet_opts) => match wallet_opts {
            WalletOptions::AddToken { id } => {
                bazuka::cli::wallet::add_token(id);
            }
            WalletOptions::NewToken {
                memo,
                name,
                symbol,
                supply,
                decimals,
                mintable,
                fee,
            } => {
                bazuka::cli::wallet::new_token(memo, name, symbol, supply, decimals, mintable, fee)
                    .await;
            }
            WalletOptions::Send {
                memo,
                from,
                to,
                amount,
                fee,
                token,
            } => {
                bazuka::cli::wallet::send(memo, from, to, amount, fee, token).await;
            }
            WalletOptions::Reset {} => {
                bazuka::cli::wallet::reset();
            }
            WalletOptions::RegisterValidator { memo, fee } => {
                bazuka::cli::wallet::register_validator(memo, fee).await;
            }
            WalletOptions::ReclaimDelegate { .. } => {
                unimplemented!();
            }
            WalletOptions::Delegate {
                memo,
                amount,
                since,
                count,
                to,
                fee,
            } => {
                bazuka::cli::wallet::delegate(memo, amount, since, count, to, fee).await;
            }
            WalletOptions::ResendPending { fill_gaps, shift } => {
                bazuka::cli::wallet::resend_pending(fill_gaps, shift).await;
            }
            WalletOptions::Info {} => {
                bazuka::cli::wallet::info().await;
            }
        },
    }

    Ok(())
}

#[cfg(not(feature = "client"))]
fn main() {}
