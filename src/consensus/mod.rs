use crate::consensus::slots::Slot;
use crate::core::Header;

pub mod digest;
mod slots;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Custom(String),
    #[error("the quantity of pre-digests in header can't be bigger than one")]
    MultiplePreDigests,
    #[error("the quantity of pre-digests in header can't be zero")]
    NoPreDigest,
}

#[async_trait::async_trait]
pub trait CreateSlotAuxProvider: Sync + Send {
    type SlotAuxData: SlotAuxData;

    async fn create_aux_provider<H>(&self, parent_hash: H) -> Result<Self::SlotAuxData>
    where
        H: AsRef<[u8]>;
}

/// Provide Some Aux Data for Slot,
/// such as: timestamp, slot, parent_block_proof and more
pub trait SlotAuxData: Sync + Send {
    fn timestamp(&self) -> u64;
    fn slot(&self) -> Slot;
    fn parent_block_proof(&self) -> Vec<u8>;
}

/// ChainSelector would select `best` chain upon which kind of strategy is used.
#[async_trait::async_trait]
pub trait ChainSelector: Sync + Send + Clone {
    async fn best_chain(&self) -> Result<Header>;
}
