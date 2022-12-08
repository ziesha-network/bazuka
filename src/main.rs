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
    bazuka::core::{ChainSourcedTx, Money, MpnAddress, MpnSourcedTx},
    bazuka::wallet::{TxBuilder, Wallet},
    colored::Colorize,
    rand::Rng,
    serde::{Deserialize, Serialize},
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
#[cfg(feature = "client")]
enum WalletOptions {
    /// Creates a new MPN-account
    NewAccount {
        #[structopt(long)]
        index: Option<u32>,
        #[structopt(long, default_value = "0")]
        initial: Money,
        #[structopt(long, default_value = "0")]
        fee: Money,
    },
    /// Deposit funds to a the MPN-contract
    Deposit {
        #[structopt(long)]
        to: MpnAddress,
        #[structopt(long)]
        amount: Money,
        #[structopt(long, default_value = "0")]
        fee: Money,
    },
    /// Withdraw funds from the MPN-contract
    Withdraw {
        #[structopt(long)]
        from: u32,
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
        to: MpnAddress,
        #[structopt(long)]
        amount: Money,
        #[structopt(long, default_value = "0")]
        fee: Money,
    },
    /// Resets wallet nonces
    Reset {},
    /// Get info and balances of the wallet
    Info {},
}

#[derive(StructOpt)]
#[cfg(feature = "node")]
enum NodeCliOptions {
    /// Start the node
    Start {
        #[structopt(long)]
        client_only: bool,
        #[structopt(long)]
        discord_handle: Option<String>,
        #[structopt(long)]
        min_fee: Option<String>,
    },
    /// Get status of a node
    Status {},
}

#[derive(StructOpt)]
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

#[cfg(feature = "client")]
#[allow(dead_code)]
async fn resend_all_wallet_txs(conf: BazukaConfig, wallet: &Wallet) -> Result<(), NodeError> {
    let tx_builder = TxBuilder::new(&wallet.seed());
    let (req_loop, client) = BazukaClient::connect(
        tx_builder.get_priv_key(),
        conf.random_node(),
        conf.network,
        None,
    );
    try_join!(
        async move {
            for tx in wallet.chain_sourced_txs.iter() {
                match tx {
                    ChainSourcedTx::TransactionAndDelta(tx) => {
                        client.transact(tx.clone()).await?;
                    }
                    ChainSourcedTx::MpnDeposit(tx) => {
                        client.transact_contract_deposit(tx.clone()).await?;
                    }
                }
            }
            for acc in wallet.mpn_sourced_txs.values() {
                for tx in acc.iter() {
                    match tx {
                        MpnSourcedTx::MpnTransaction(tx) => {
                            client.zero_transact(tx.clone()).await?;
                        }
                        MpnSourcedTx::MpnWithdraw(tx) => {
                            client.transact_contract_withdraw(tx.clone()).await?;
                        }
                    }
                }
            }
            Ok::<(), NodeError>(())
        },
        req_loop
    )
    .unwrap();

    Ok(())
}

#[cfg(feature = "node")]
async fn run_node(
    bazuka_config: BazukaConfig,
    wallet: Wallet,
    social_profiles: SocialProfiles,
    client_only: bool,
    min_fee: Money,
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
    let firewall = Firewall::new(60, 4 * GB);

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
        Some(min_fee),
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
    use std::str::FromStr;

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
                    for (k, v) in db.pairs(prefix.into()).unwrap() {
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
                    if !fork.db().pairs("".into()).unwrap().is_empty() {
                        println!(
                            "{} {}",
                            "Error:".bright_red(),
                            "Rollback data are corrupted!"
                        );
                    }
                    println!(
                        "Currency in Circulation: {}",
                        chain.currency_in_circulation().unwrap()
                    );
                    let mut sum_mpn: Money = 0.into();
                    for mpn_acc in chain.get_mpn_accounts(0, 10000).unwrap() {
                        sum_mpn += mpn_acc.1.balance;
                    }
                    println!("MPN accounts balance: {}", sum_mpn);
                    let mpn_acc = chain.get_contract_account(mpn_contract_id).unwrap();
                    println!("MPN contract balance: {}", mpn_acc.balance);
                }
            }
        }
        #[cfg(feature = "node")]
        CliOptions::Node(node_opts) => match node_opts {
            NodeCliOptions::Start {
                discord_handle,
                client_only,
                min_fee,
            } => {
                let conf = conf.expect("Bazuka is not initialized!");
                let wallet = wallet.expect("Wallet is not initialized!");
                let min_fee: Money = FromStr::from_str(&min_fee.unwrap_or("0.00001".to_string())).unwrap();
                run_node(
                    conf.clone(),
                    wallet.clone(),
                    SocialProfiles {
                        discord: discord_handle,
                    },
                    client_only,
                    min_fee
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
            WalletOptions::NewAccount {
                index,
                initial,
                fee,
            } => {
                let mut rng = rand::thread_rng();
                let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let tx_builder = TxBuilder::new(&wallet.seed());
                let index = index.unwrap_or_else(|| rng.gen()) & 0x3FFFFFFF;
                let (req_loop, client) = BazukaClient::connect(
                    tx_builder.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                try_join!(
                    async move {
                        let curr_nonce = client
                            .get_account(tx_builder.get_address())
                            .await?
                            .account
                            .nonce;
                        let new_nonce = wallet.new_r_nonce().unwrap_or(curr_nonce + 1);
                        let mpn_addr =MpnAddress{index,pub_key:tx_builder.get_zk_address()};
                        let pay =
                            tx_builder.deposit_mpn(mpn_contract_id, mpn_addr.clone(), new_nonce, initial, fee);
                        wallet.add_mpn_index(index);
                        wallet.add_deposit(pay.clone());
                        wallet.save(wallet_path).unwrap();
                        println!("{:#?}", client.transact_contract_deposit(pay).await?);
                        println!("New MPN-account created! Wait for your account to be confirmed by the network!");
                        println!("{} {}", "Account address:".bright_yellow(), mpn_addr);
                        Ok::<(), NodeError>(())
                    },
                    req_loop
                )
                .unwrap();
            }
            WalletOptions::Deposit { to, amount, fee } => {
                let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let tx_builder = TxBuilder::new(&wallet.seed());
                let (req_loop, client) = BazukaClient::connect(
                    tx_builder.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                try_join!(
                    async move {
                        let curr_nonce = client
                            .get_account(tx_builder.get_address())
                            .await?
                            .account
                            .nonce;
                        let new_nonce = wallet.new_r_nonce().unwrap_or(curr_nonce + 1);
                        let pay =
                            tx_builder.deposit_mpn(mpn_contract_id, to, new_nonce, amount, fee);
                        wallet.add_deposit(pay.clone());
                        wallet.save(wallet_path).unwrap();
                        println!("{:#?}", client.transact_contract_deposit(pay).await?);
                        Ok::<(), NodeError>(())
                    },
                    req_loop
                )
                .unwrap();
            }
            WalletOptions::Withdraw { from, amount, fee } => {
                let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let tx_builder = TxBuilder::new(&wallet.seed());
                let (req_loop, client) = BazukaClient::connect(
                    tx_builder.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                try_join!(
                    async move {
                        let curr_nonce = client.get_mpn_account(from).await?.account.nonce;
                        let new_nonce = wallet.new_z_nonce(from).unwrap_or(curr_nonce);
                        let pay =
                            tx_builder.withdraw_mpn(mpn_contract_id, from, new_nonce, amount, fee);
                        wallet.add_withdraw(pay.clone());
                        wallet.save(wallet_path).unwrap();
                        println!("{:#?}", client.transact_contract_withdraw(pay).await?);
                        Ok::<(), NodeError>(())
                    },
                    req_loop
                )
                .unwrap();
            }
            WalletOptions::Rsend { to, amount, fee } => {
                let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let tx_builder = TxBuilder::new(&wallet.seed());
                let (req_loop, client) = BazukaClient::connect(
                    tx_builder.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                try_join!(
                    async move {
                        let curr_nonce = client
                            .get_account(tx_builder.get_address())
                            .await?
                            .account
                            .nonce;
                        let new_nonce = wallet.new_r_nonce().unwrap_or(curr_nonce + 1);
                        let tx = tx_builder.create_transaction(
                            to.parse().unwrap(),
                            amount,
                            fee,
                            new_nonce,
                        );
                        wallet.add_rsend(tx.clone());
                        wallet.save(wallet_path).unwrap();
                        println!("{:#?}", client.transact(tx).await?);
                        Ok::<(), NodeError>(())
                    },
                    req_loop
                )
                .unwrap();
            }
            WalletOptions::Zsend {
                from_index,
                to,
                amount,
                fee,
            } => {
                let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let tx_builder = TxBuilder::new(&wallet.seed());
                let (req_loop, client) = BazukaClient::connect(
                    tx_builder.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                try_join!(
                    async move {
                        let curr_nonce = client.get_mpn_account(from_index).await?.account.nonce;
                        let new_nonce = wallet.new_z_nonce(from_index).unwrap_or(curr_nonce);
                        let tx = tx_builder
                            .create_mpn_transaction(from_index, to, amount, fee, new_nonce);
                        wallet.add_zsend(tx.clone());
                        wallet.save(wallet_path).unwrap();
                        println!("{:#?}", client.zero_transact(tx).await?);
                        Ok::<(), NodeError>(())
                    },
                    req_loop
                )
                .unwrap();
            }
            WalletOptions::Reset {} => {
                let mut wallet = wallet.expect("Bazuka is not initialized!");
                wallet.reset();
                wallet.save(wallet_path).unwrap();
            }
            WalletOptions::Info {} => {
                let (conf, wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let tx_builder = TxBuilder::new(&wallet.seed());

                println!(
                    "{} {}",
                    "Wallet address:".bright_yellow(),
                    tx_builder.get_address()
                );

                let (req_loop, client) = BazukaClient::connect(
                    tx_builder.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                try_join!(
                    async move {
                        let acc = client.get_account(tx_builder.get_address()).await;
                        let curr_nonce = wallet.new_r_nonce().map(|n| n - 1);
                        println!(
                            "{} {}",
                            "Main chain balance:".bright_yellow(),
                            acc.map(|resp| format!(
                                "{}{}",
                                resp.account.balance,
                                curr_nonce
                                    .map(|n| if n > resp.account.nonce {
                                        format!(
                                            " (Pending transactions: {})",
                                            n - resp.account.nonce
                                        )
                                    } else {
                                        "".into()
                                    })
                                    .unwrap_or_default()
                            ))
                            .unwrap_or("Node not available!".into()),
                        );
                        println!();
                        println!("{}", "MPN Accounts\n---------".bright_yellow());
                        for ind in wallet.mpn_indices() {
                            let resp = client.get_mpn_account(ind).await.map(|resp| resp.account);
                            println!("{}", format!("#{}:", ind).bright_yellow());
                            if let Ok(resp) = resp {
                                if !resp.address.is_on_curve() {
                                    println!("\tWaiting to be created...")
                                } else {
                                    println!("\tBalance: {}", resp.balance);
                                    println!(
                                        "\tAddress: {}",
                                        MpnAddress {
                                            pub_key: bazuka::crypto::jubjub::PublicKey(
                                                resp.address.compress()
                                            ),
                                            index: ind
                                        }
                                    );
                                }
                            } else {
                                println!("\tNode not available!");
                            }
                        }
                        Ok::<(), NodeError>(())
                    },
                    req_loop
                )
                .unwrap();
            }
        },
    }

    Ok(())
}

#[cfg(not(feature = "client"))]
fn main() {}
