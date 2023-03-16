use crate::db::KvStoreError;
use crate::zk::{StateManagerError, ZkError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockchainError {
    #[error("kvstore error happened: {0}")]
    KvStoreError(#[from] KvStoreError),
    #[error("different genesis block exists on the database")]
    DifferentGenesis,
    #[error("transaction signature is invalid")]
    SignatureError,
    #[error("balance insufficient")]
    BalanceInsufficient,
    #[error("contract balance insufficient")]
    ContractBalanceInsufficient,
    #[error("inconsistency error")]
    Inconsistency,
    #[error("block not found")]
    BlockNotFound,
    #[error("cannot extend from the genesis block")]
    ExtendFromGenesis,
    #[error("cannot extend from very future blocks")]
    ExtendFromFuture,
    #[error("block number invalid")]
    InvalidBlockNumber,
    #[error("parent hash invalid")]
    InvalidParentHash,
    #[error("merkle root invalid")]
    InvalidMerkleRoot,
    #[error("transaction nonce invalid")]
    InvalidTransactionNonce,
    #[error("block timestamp is in past")]
    InvalidTimestamp,
    #[error("miner reward not present")]
    MinerRewardNotFound,
    #[error("illegal access to treasury funds")]
    IllegalTreasuryAccess,
    #[error("miner reward transaction is invalid")]
    InvalidMinerReward,
    #[error("contract not found")]
    ContractNotFound,
    #[error("staker not found")]
    StakerNotFound,
    #[error("update function not found in the given contract")]
    ContractFunctionNotFound,
    #[error("Incorrect zero-knowledge proof")]
    IncorrectZkProof,
    #[error("Full-state not found in the update provided")]
    FullStateNotFound,
    #[error("Invalid full-state in the update provided")]
    FullStateNotValid,
    #[error("cannot draft a new block when full-states are outdated")]
    StatesOutdated,
    #[error("contract states at requested height are unavailable")]
    StatesUnavailable,
    #[error("block too big")]
    BlockTooBig,
    #[error("state-delta too big")]
    StateDeltaTooBig,
    #[error("compressed-state at specified height not found")]
    CompressedStateNotFound,
    #[error("no blocks to roll back")]
    NoBlocksToRollback,
    #[error("zk error happened: {0}")]
    ZkError(#[from] ZkError),
    #[error("state-manager error happened: {0}")]
    StateManagerError(#[from] StateManagerError),
    #[error("invalid deposit/withdraw signature")]
    InvalidContractPaymentSignature,
    #[error("insufficient mpn updates")]
    InsufficientMpnUpdates,
    #[error("invalid zero-transaction")]
    InvalidMpnTransaction,
    #[error("contract contains invalid state-model")]
    InvalidStateModel,
    #[error("height limit reached! if you are on a testnet, make sure you update your software")]
    TestnetHeightLimitReached,
    #[error("address not allowed to mine")]
    AddressNotAllowedToMine,
    #[error(
        "deposit/withdraw transaction was not intended to be passed to this contract/function"
    )]
    DepositWithdrawPassedToWrongFunction,
    #[error("block not on the testnet forced fork")]
    TestnetForcedFork,
    #[error("token already exists")]
    TokenAlreadyExists,
    #[error("token not found")]
    TokenNotFound,
    #[error("token not updatable")]
    TokenNotUpdatable,
    #[error("token is being updated by a wrong account")]
    TokenUpdatePermissionDenied,
    #[error("token supply not enough to be redeemed")]
    TokenSupplyInsufficient,
    #[error("token supply overflows when issued")]
    TokenSupplyOverflow,
    #[error("token has an invalid name/symbol")]
    TokenBadNameSymbol,
    #[error("only ziesha fees are accepted!")]
    OnlyZieshaFeesAccepted,
    #[error("transaction memo is too long")]
    MemoTooLong,
    #[error("Wrong validator has built the block!")]
    UnelectedValidator,
    #[error("delegate not found")]
    DelegateNotFound,
    #[error("cannot destroy a delegate that is still active")]
    DelegateStillActive,
    #[error("only a single update is allowed per contract in a block")]
    SingleUpdateAllowedPerContract,
    #[error("validator is not registered")]
    ValidatorNotRegistered,
}
