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
        Amount, ChainSourcedTx, Money, MpnAddress, MpnSourcedTx, TokenId, ZieshaAddress,
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
    /// Resets wallet nonces
    Reset {},
    /// Get info and balances of the wallet
    Info {},
    /// Resend pending transactions
    ResendPending {},
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
    let mpn_log4_account_capacity =
        config::blockchain::get_blockchain_config().mpn_log4_account_capacity;
    try_join!(
        async move {
            let curr_nonce = client
                .get_account(tx_builder.get_address())
                .await?
                .account
                .nonce;
            let curr_mpn_nonce = client
                .get_mpn_account(
                    MpnAddress {
                        pub_key: tx_builder.get_zk_address(),
                    }
                    .account_index(mpn_log4_account_capacity),
                )
                .await?
                .account
                .nonce;
            for tx in wallet.chain_sourced_txs.iter() {
                if tx.nonce() >= curr_nonce {
                    match tx {
                        ChainSourcedTx::TransactionAndDelta(tx) => {
                            client.transact(tx.clone()).await?;
                        }
                        ChainSourcedTx::MpnDeposit(tx) => {
                            client.transact_contract_deposit(tx.clone()).await?;
                        }
                    }
                }
            }
            for acc in wallet.mpn_sourced_txs.values() {
                for tx in acc.iter() {
                    if tx.nonce() >= curr_mpn_nonce {
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
                    let rollback_validity_check = fork.db().pairs("".into()).unwrap().is_empty();
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
                let public_ip = bazuka::client::utils::get_public_ip().await.unwrap();
                std::fs::write(
                    conf_path,
                    serde_yaml::to_string(&BazukaConfig {
                        network,
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
                let mut wallet = wallet.expect("Bazuka is not initialized!");

                wallet.add_token(id);
                wallet.save(wallet_path).unwrap();
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
                        let (pay, token_id) = tx_builder.create_token(
                            memo.unwrap_or_default(),
                            name,
                            symbol,
                            supply,
                            decimals,
                            mintable.then(|| tx_builder.get_address()),
                            Money {
                                amount: fee,
                                token_id: TokenId::Ziesha,
                            },
                            new_nonce,
                        );
                        wallet.add_token(token_id);
                        wallet.add_rsend(pay.clone());
                        wallet.save(wallet_path).unwrap();
                        println!("Token-Id: {}", token_id);
                        println!("{:#?}", client.transact(pay).await?);
                        Ok::<(), NodeError>(())
                    },
                    req_loop
                )
                .unwrap();
            }
            WalletOptions::Send {
                memo,
                from,
                to,
                amount,
                fee,
                token,
            } => {
                let (conf, mut wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let tx_builder = TxBuilder::new(&wallet.seed());
                let (req_loop, client) = BazukaClient::connect(
                    tx_builder.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                let tkn = if let Some(token) = token {
                    if token >= wallet.get_tokens().len() {
                        panic!("Wrong token selected!");
                    } else {
                        wallet.get_tokens()[token]
                    }
                } else {
                    TokenId::Ziesha
                };
                match from {
                    ZieshaAddress::ChainAddress(from) => {
                        if tx_builder.get_address() != from {
                            panic!("Source address doesn't exist in your wallet!");
                        }
                        match to {
                            ZieshaAddress::ChainAddress(to) => {
                                try_join!(
                                    async move {
                                        let curr_nonce = client
                                            .get_account(tx_builder.get_address())
                                            .await?
                                            .account
                                            .nonce;
                                        let new_nonce =
                                            wallet.new_r_nonce().unwrap_or(curr_nonce + 1);
                                        let tx = tx_builder.create_transaction(
                                            memo.unwrap_or_default(),
                                            to,
                                            Money {
                                                amount,
                                                token_id: tkn,
                                            },
                                            Money {
                                                amount: fee,
                                                token_id: TokenId::Ziesha,
                                            },
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
                            ZieshaAddress::MpnAddress(to) => {
                                try_join!(
                                    async move {
                                        let curr_nonce = client
                                            .get_account(tx_builder.get_address())
                                            .await?
                                            .account
                                            .nonce;
                                        let dst_acc = client
                                            .get_mpn_account(
                                                to.account_index(mpn_log4_account_capacity),
                                            )
                                            .await?
                                            .account;
                                        let to_token_index = if let Some(ind) = dst_acc
                                            .find_token_index(
                                                config::blockchain::MPN_LOG4_TOKEN_CAPACITY,
                                                tkn,
                                                true,
                                            ) {
                                            ind
                                        } else {
                                            panic!(
                                                "Cannot find empty token slot in your MPN account!"
                                            );
                                        };
                                        let new_nonce =
                                            wallet.new_r_nonce().unwrap_or(curr_nonce + 1);
                                        let pay = tx_builder.deposit_mpn(
                                            memo.unwrap_or_default(),
                                            mpn_contract_id,
                                            to,
                                            to_token_index,
                                            new_nonce,
                                            Money {
                                                amount,
                                                token_id: tkn,
                                            },
                                            Money {
                                                amount: fee,
                                                token_id: TokenId::Ziesha,
                                            },
                                        );
                                        wallet.add_deposit(pay.clone());
                                        wallet.save(wallet_path).unwrap();
                                        println!(
                                            "{:#?}",
                                            client.transact_contract_deposit(pay).await?
                                        );
                                        Ok::<(), NodeError>(())
                                    },
                                    req_loop
                                )
                                .unwrap();
                            }
                        }
                    }
                    ZieshaAddress::MpnAddress(from) => {
                        if tx_builder.get_zk_address() != from.pub_key {
                            panic!("Source address doesn't exist in your wallet!");
                        }
                        match to {
                            ZieshaAddress::ChainAddress(to) => {
                                try_join!(
                                    async move {
                                        let acc = client
                                            .get_mpn_account(
                                                from.account_index(mpn_log4_account_capacity),
                                            )
                                            .await?
                                            .account;
                                        let token_index = if let Some(ind) = acc.find_token_index(
                                            config::blockchain::MPN_LOG4_TOKEN_CAPACITY,
                                            tkn,
                                            false,
                                        ) {
                                            ind
                                        } else {
                                            panic!("Token not found in your account!");
                                        };
                                        let fee_token_index = if let Some(ind) = acc
                                            .find_token_index(
                                                config::blockchain::MPN_LOG4_TOKEN_CAPACITY,
                                                TokenId::Ziesha,
                                                false,
                                            ) {
                                            ind
                                        } else {
                                            panic!("Token not found in your account!");
                                        };
                                        let new_nonce =
                                            wallet.new_z_nonce(&from).unwrap_or(acc.nonce);
                                        let pay = tx_builder.withdraw_mpn(
                                            memo.unwrap_or_default(),
                                            mpn_contract_id,
                                            new_nonce,
                                            token_index,
                                            Money {
                                                amount,
                                                token_id: tkn,
                                            },
                                            fee_token_index,
                                            Money {
                                                amount: fee,
                                                token_id: TokenId::Ziesha,
                                            },
                                            to.to_string().parse().unwrap(), // TODO: WTH :D
                                        );
                                        wallet.add_withdraw(pay.clone());
                                        wallet.save(wallet_path).unwrap();
                                        println!(
                                            "{:#?}",
                                            client.transact_contract_withdraw(pay).await?
                                        );
                                        Ok::<(), NodeError>(())
                                    },
                                    req_loop
                                )
                                .unwrap();
                            }
                            ZieshaAddress::MpnAddress(to) => {
                                try_join!(
                                    async move {
                                        if memo.is_some() {
                                            panic!(
                                                "Cannot assign a memo to a MPN-to-MPN transaction!"
                                            );
                                        }
                                        let acc = client
                                            .get_mpn_account(
                                                from.account_index(mpn_log4_account_capacity),
                                            )
                                            .await?
                                            .account;
                                        let dst_acc = client
                                            .get_mpn_account(
                                                to.account_index(mpn_log4_account_capacity),
                                            )
                                            .await?
                                            .account;
                                        let to_token_index = if let Some(ind) = dst_acc
                                            .find_token_index(
                                                config::blockchain::MPN_LOG4_TOKEN_CAPACITY,
                                                tkn,
                                                true,
                                            ) {
                                            ind
                                        } else {
                                            panic!("Token not found in your account!");
                                        };
                                        let token_index = if let Some(ind) = acc.find_token_index(
                                            config::blockchain::MPN_LOG4_TOKEN_CAPACITY,
                                            tkn,
                                            false,
                                        ) {
                                            ind
                                        } else {
                                            panic!("Token not found in your account!");
                                        };
                                        let fee_token_index = if let Some(ind) = acc
                                            .find_token_index(
                                                config::blockchain::MPN_LOG4_TOKEN_CAPACITY,
                                                TokenId::Ziesha,
                                                false,
                                            ) {
                                            ind
                                        } else {
                                            panic!("Token not found in your account!");
                                        };
                                        let new_nonce =
                                            wallet.new_z_nonce(&from).unwrap_or(acc.nonce);
                                        let tx = tx_builder.create_mpn_transaction(
                                            token_index,
                                            to,
                                            to_token_index,
                                            Money {
                                                amount,
                                                token_id: tkn,
                                            },
                                            fee_token_index,
                                            Money {
                                                amount: fee,
                                                token_id: TokenId::Ziesha,
                                            },
                                            new_nonce,
                                        );
                                        wallet.add_zsend(tx.clone());
                                        wallet.save(wallet_path).unwrap();
                                        println!("{:#?}", client.zero_transact(tx).await?);
                                        Ok::<(), NodeError>(())
                                    },
                                    req_loop
                                )
                                .unwrap();
                            }
                        }
                    }
                }
            }
            WalletOptions::Reset {} => {
                let mut wallet = wallet.expect("Bazuka is not initialized!");
                wallet.reset();
                wallet.save(wallet_path).unwrap();
            }
            WalletOptions::ResendPending {} => {
                let (conf, wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                resend_all_wallet_txs(conf, &wallet).await?;
            }
            WalletOptions::Info {} => {
                let (conf, wallet) = conf.zip(wallet).expect("Bazuka is not initialized!");
                let tx_builder = TxBuilder::new(&wallet.seed());

                let (req_loop, client) = BazukaClient::connect(
                    tx_builder.get_priv_key(),
                    conf.random_node(),
                    conf.network,
                    None,
                );
                try_join!(
                    async move {
                        let acc = client.get_account(tx_builder.get_address()).await;
                        let mut token_balances = HashMap::new();
                        let mut token_indices = HashMap::new();
                        for (i, tkn) in wallet.get_tokens().iter().enumerate() {
                            token_indices.insert(*tkn, i);
                            if let Ok(inf) =
                                client.get_balance(tx_builder.get_address(), *tkn).await
                            {
                                token_balances.insert(*tkn, inf);
                            }
                        }

                        let curr_nonce = wallet.new_r_nonce().map(|n| n - 1);
                        println!();
                        println!("{}", "Main-chain\n---------".bright_green());
                        println!(
                            "{}\t{}",
                            "Address:".bright_yellow(),
                            tx_builder.get_address()
                        );
                        if let Ok(resp) = acc {
                            for (i, id) in wallet.get_tokens().iter().enumerate() {
                                if let Some(inf) = token_balances.get(id) {
                                    println!(
                                        "{}\t{}{}",
                                        format!("#{} <{}>:", i, inf.name).bright_yellow(),
                                        inf.balance,
                                        if *id == TokenId::Ziesha {
                                            bazuka::config::SYMBOL.to_string()
                                        } else {
                                            format!(" {} (Token-Id: {})", inf.symbol, id)
                                        }
                                    );
                                } else {
                                    println!("{}\t{}", format!("#{}:", i).bright_yellow(), "N/A");
                                }
                            }
                            if let Some(nonce) = curr_nonce {
                                if nonce > resp.account.nonce {
                                    println!(
                                        "(Pending transactions: {})",
                                        nonce - resp.account.nonce
                                    );
                                }
                            }
                        } else {
                            println!("{} {}", "Error:".bright_red(), "Node not available!");
                        }

                        println!();

                        let mpn_address = MpnAddress {
                            pub_key: tx_builder.get_zk_address(),
                        };

                        for (i, addr) in [mpn_address].into_iter().enumerate() {
                            println!(
                                "{}",
                                format!("MPN Account #{}\n---------", i).bright_green()
                            );
                            let resp = client
                                .get_mpn_account(addr.account_index(mpn_log4_account_capacity))
                                .await
                                .map(|resp| resp.account);
                            if let Ok(resp) = resp {
                                let curr_z_nonce = wallet.new_z_nonce(&addr);
                                if !resp.address.is_on_curve() {
                                    println!(
                                        "{}\t{}",
                                        "Address:".bright_yellow(),
                                        MpnAddress {
                                            pub_key: tx_builder.get_zk_address(),
                                        }
                                    );
                                    println!("Waiting to be activated... (Send some funds to it!)")
                                } else {
                                    let acc_pk =
                                        bazuka::crypto::jubjub::PublicKey(resp.address.compress());
                                    if acc_pk != tx_builder.get_zk_address() {
                                        println!(
                                            "{} {}",
                                            "Error:".bright_red(),
                                            "Slot acquired by someone else!"
                                        );
                                        continue;
                                    }
                                    println!(
                                        "{}\t{}",
                                        "Address:".bright_yellow(),
                                        MpnAddress { pub_key: acc_pk }
                                    );
                                    for (_, money) in resp.tokens.iter() {
                                        if let Some(inf) = token_balances.get(&money.token_id) {
                                            let token_index = token_indices[&money.token_id];
                                            println!(
                                                "{}\t{}{}",
                                                format!("#{} <{}>:", token_index, inf.name)
                                                    .bright_yellow(),
                                                money.amount,
                                                if money.token_id == TokenId::Ziesha {
                                                    bazuka::config::SYMBOL.to_string()
                                                } else {
                                                    format!(" {}", inf.symbol)
                                                }
                                            );
                                        }
                                    }
                                }
                                if let Some(nonce) = curr_z_nonce {
                                    if nonce > resp.nonce {
                                        println!("(Pending transactions: {})", nonce - resp.nonce);
                                    }
                                }
                            } else {
                                println!("{} {}", "Error:".bright_red(), "Node not available!");
                            }
                            println!();
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
