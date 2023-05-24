#[cfg(feature = "node")]
use {
    bazuka::blockchain::Blockchain,
    bazuka::client::{messages::SocialProfiles, Limit, NodeRequest},
    bazuka::common::*,
    bazuka::db::KvStore,
    bazuka::node::{node_create, Firewall},
    hyper::server::conn::AddrStream,
    hyper::service::{make_service_fn, service_fn},
    hyper::{Body, Client, Request, Response, Server, StatusCode},
    std::sync::Arc,
    tokio::sync::mpsc,
};

#[cfg(feature = "client")]
use {
    bazuka::client::{NodeError, PeerAddress},
    bazuka::config,
    bazuka::core::{Address, Decimal, GeneralAddress, TokenId},
    bazuka::mpn::MpnWorker,
    bazuka::wallet::WalletCollection,
    serde::{Deserialize, Serialize},
    std::net::SocketAddr,
    std::path::{Path, PathBuf},
    structopt::StructOpt,
    tokio::try_join,
};

use {
    colored::Colorize,
    std::io::{self, Write},
    std::panic,
};

pub mod chain;
pub mod init;
pub mod wallet;
pub use init::*;

#[cfg(feature = "node")]
pub mod node;

#[cfg(feature = "client")]
const DEFAULT_PORT: u16 = 8765;
const BAZUKA_NOT_INITILIZED: &str = "Bazuka is not initialized";

const CURRENT_NETWORK: &str = "deruny-2";

#[cfg(feature = "client")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BazukaConfigMpnWorker {
    address: String,
}

#[cfg(feature = "client")]
#[derive(Debug)]
pub struct InvalidMpnWorker;

impl TryInto<MpnWorker> for BazukaConfigMpnWorker {
    type Error = InvalidMpnWorker;
    fn try_into(self) -> Result<MpnWorker, InvalidMpnWorker> {
        Ok(MpnWorker {
            address: self.address.parse().map_err(|_| InvalidMpnWorker)?,
        })
    }
}

#[cfg(feature = "client")]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BazukaConfig {
    listen: SocketAddr,
    external: PeerAddress,
    bootstrap: Vec<PeerAddress>,
    db: PathBuf,
    mpn_workers: Vec<BazukaConfigMpnWorker>,
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
        supply: Decimal,
        #[structopt(long, default_value = "0")]
        decimals: u8,
        #[structopt(long)]
        mintable: bool,
        #[structopt(long, default_value = "0")]
        fee: Decimal,
    },
    /// Send money
    Send {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        from: GeneralAddress,
        #[structopt(long)]
        to: GeneralAddress,
        #[structopt(long)]
        token_id: Option<TokenId>,
        #[structopt(long)]
        amount: Decimal,
        #[structopt(long, default_value = "0")]
        fee: Decimal,
    },
    /// Register your validator
    RegisterValidator {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        commission: f32,
        #[structopt(long, default_value = "0")]
        fee: Decimal,
    },
    /// Delegate to a validator
    Delegate {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        to: Address,
        #[structopt(long)]
        amount: Decimal,
        #[structopt(long, default_value = "0")]
        fee: Decimal,
    },
    /// Automatically re-delegate a ratio of staking rewards
    AutoDelegate {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        to: Address,
        #[structopt(long)]
        ratio: f32,
        #[structopt(long, default_value = "0")]
        fee: Decimal,
    },
    /// Reclaim funds inside an ended delegatation back to your account
    Undelegate {
        #[structopt(long)]
        memo: Option<String>,
        #[structopt(long)]
        from: Address,
        #[structopt(long)]
        amount: Decimal,
        #[structopt(long, default_value = "0")]
        fee: Decimal,
    },
    /// Resets wallet nonces
    Reset {},
    /// Get info and balances of the wallet
    Info {
        #[structopt(long)]
        validator: bool,
    },
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
        #[structopt(long)]
        ram: bool,
        #[structopt(long)]
        dev: bool,
        #[structopt(long)]
        small_mpn: bool,
    },
    /// Get status of a node
    Status {},
    /// Add a new mpn worker
    AddMpnWorker { address: Address },
}

#[derive(StructOpt)]
#[allow(clippy::large_enum_variant)]
#[cfg(feature = "client")]
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
async fn run_node<K: KvStore, B: Blockchain<K>>(
    blockchain: B,
    bazuka_config: BazukaConfig,
    wallet: WalletCollection,
    social_profiles: SocialProfiles,
    client_only: bool,
    network: String,
) -> Result<(), NodeError> {
    let address = if client_only {
        None
    } else {
        Some(bazuka_config.external)
    };

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
    println!("{} {}", "Network:".bright_yellow(), network);

    let (inc_send, inc_recv) = mpsc::unbounded_channel::<NodeRequest>();
    let (out_send, mut out_recv) = mpsc::unbounded_channel::<NodeRequest>();

    let bootstrap_nodes = bazuka_config.bootstrap.clone();

    // 60 request per minute / 4GB per 15min
    let firewall = Firewall::new(360, 4 * GB);

    // Async loop that is responsible for answering external requests and gathering
    // data from external world through a heartbeat loop.
    let node = node_create(
        config::node::get_node_options(),
        &network,
        address,
        bootstrap_nodes,
        blockchain,
        0,
        wallet.clone().validator().tx_builder(),
        wallet.clone().user(0).tx_builder(),
        social_profiles,
        inc_recv,
        out_send,
        Some(firewall),
        bazuka_config
            .mpn_workers
            .iter()
            .map(|w| w.clone().try_into().unwrap())
            .collect(),
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
                                mpsc::unbounded_channel::<Result<Response<Body>, NodeError>>();
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
                if let Err(e) = req.resp.send(resp) {
                    log::debug!("Node not listening to its HTTP request answer: {}", e);
                }
            });
        }
        Ok::<(), NodeError>(())
    };

    try_join!(server_loop, client_loop, node).unwrap();

    Ok(())
}

pub async fn initialize_cli() {
    let opts = CliOptions::from_args();

    let conf_path = home::home_dir().unwrap().join(Path::new(".bazuka.yaml"));
    let conf: Option<BazukaConfig> = std::fs::File::open(conf_path.clone())
        .ok()
        .map(|f| serde_yaml::from_reader(f).unwrap());
    let wallet_path = home::home_dir().unwrap().join(Path::new(".bazuka-wallet"));
    let wallet = WalletCollection::open(wallet_path.clone()).unwrap();

    panic::set_hook(Box::new(|panic_info| {
        let default_message = "Unknown panic".to_string();

        let message = panic_info
            .payload()
            .downcast_ref::<String>()
            .unwrap_or(&default_message);

        let stderr = io::stderr();
        let mut stderr_handle = stderr.lock();
        write!(stderr_handle, "{} ", "Error:".bold().red()).unwrap();
        stderr_handle.write_all(message.as_bytes()).unwrap();
        stderr_handle.write_all(b"\n").unwrap();
    }));

    match opts {
        CliOptions::Chain(chain_opts) => match chain_opts {
            ChainCliOptions::Rollback {} => {
                crate::cli::chain::rollback(&conf.expect(BAZUKA_NOT_INITILIZED)).await;
            }
            ChainCliOptions::DbQuery { prefix } => {
                crate::cli::chain::db_query(prefix, &conf.expect(BAZUKA_NOT_INITILIZED));
            }
            ChainCliOptions::HealthCheck {} => {
                crate::cli::chain::health_check(&conf.expect(BAZUKA_NOT_INITILIZED));
            }
        },
        #[cfg(feature = "node")]
        CliOptions::Node(node_opts) => match node_opts {
            NodeCliOptions::Start {
                discord_handle,
                client_only,
                dev,
                ram,
                small_mpn,
            } => {
                crate::cli::node::start(
                    discord_handle,
                    client_only,
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    ram,
                    dev,
                    small_mpn,
                )
                .await;
            }
            NodeCliOptions::Status {} => {
                crate::cli::node::status(
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                )
                .await;
            }
            NodeCliOptions::AddMpnWorker { address } => {
                crate::cli::node::add_mpn_worker(
                    &conf_path,
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    address,
                )
                .await;
            }
        },
        #[cfg(feature = "client")]
        CliOptions::Init {
            bootstrap,
            mnemonic,
            external,
            listen,
            db,
        } => {
            crate::cli::init(
                bootstrap,
                mnemonic,
                external,
                listen,
                db,
                conf,
                &conf_path,
                wallet,
                &wallet_path,
            )
            .await
        }
        #[cfg(not(feature = "client"))]
        CliOptions::Init { .. } => {
            println!("Client feature not turned on!");
        }
        CliOptions::Wallet(wallet_opts) => match wallet_opts {
            WalletOptions::AddToken { id } => {
                crate::cli::wallet::add_token(
                    id,
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    &wallet_path,
                );
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
                crate::cli::wallet::new_token(
                    memo,
                    name,
                    symbol,
                    supply,
                    decimals,
                    mintable,
                    fee,
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    &wallet_path,
                )
                .await;
            }
            WalletOptions::Send {
                memo,
                from,
                to,
                amount,
                fee,
                token_id,
            } => {
                crate::cli::wallet::send(
                    memo,
                    from,
                    to,
                    amount,
                    fee,
                    token_id,
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    &wallet_path,
                )
                .await;
            }
            WalletOptions::Reset {} => {
                crate::cli::wallet::reset(wallet.expect(BAZUKA_NOT_INITILIZED), &wallet_path);
            }
            WalletOptions::RegisterValidator {
                memo,
                commission,
                fee,
            } => {
                crate::cli::wallet::register_validator(
                    memo,
                    commission,
                    fee,
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    &wallet_path,
                )
                .await;
            }
            WalletOptions::Undelegate {
                memo,
                amount,
                from,
                fee,
            } => {
                crate::cli::wallet::undelegate(
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    &wallet_path,
                    memo,
                    amount,
                    from,
                    fee,
                )
                .await;
            }
            WalletOptions::AutoDelegate {
                memo,
                to,
                ratio,
                fee,
            } => {
                crate::cli::wallet::auto_delegate(
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    &wallet_path,
                    memo,
                    to,
                    ratio.try_into().unwrap(),
                    fee,
                )
                .await;
            }
            WalletOptions::Delegate {
                memo,
                amount,
                to,
                fee,
            } => {
                crate::cli::wallet::delegate(
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    &wallet_path,
                    memo,
                    amount,
                    to,
                    fee,
                )
                .await;
            }
            WalletOptions::ResendPending {} => {
                crate::cli::wallet::resend_pending(
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    &wallet_path,
                )
                .await;
            }
            WalletOptions::Info { validator } => {
                crate::cli::wallet::info(
                    conf.expect(BAZUKA_NOT_INITILIZED),
                    wallet.expect(BAZUKA_NOT_INITILIZED),
                    validator,
                )
                .await;
            }
        },
    }
}
