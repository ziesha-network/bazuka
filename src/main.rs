#[cfg(not(tarpaulin_include))]
pub mod cli;

use crate::cli::initialize_cli;
use log::LevelFilter;
use std::io::Write;

#[cfg(feature = "client")]
use bazuka::client::NodeError;

#[cfg(not(tarpaulin_include))]
#[cfg(feature = "client")]
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    env_logger::builder()
        .filter(None, LevelFilter::Info)
        .format(|buf, record| {
            let ts = buf.timestamp();
            bazuka::report_log(&format!("{}: {}: {}", ts, record.level(), record.args()));
            writeln!(
                buf,
                "{}: {}: {}",
                ts,
                buf.default_styled_level(record.level()),
                record.args()
            )
        })
        .init();
    initialize_cli().await;
    Ok(())
}

#[cfg(not(feature = "client"))]
fn main() {}
