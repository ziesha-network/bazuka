#[cfg(not(tarpaulin_include))]
pub mod cli;

use crate::cli::initialize_cli;

#[cfg(feature = "client")]
use bazuka::client::NodeError;

#[cfg(not(tarpaulin_include))]
#[cfg(feature = "client")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    env_logger::init();
    initialize_cli().await;
    Ok(())
}

#[cfg(not(feature = "client"))]
fn main() {}
