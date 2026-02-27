// src/models/mod.rs
pub mod wallet;
pub mod transaction;
pub mod owner;
pub mod blockstreams;
pub mod token_transfer;
pub mod token_metadata;
pub mod sync_state;
pub mod tron_raw_log;
pub mod tron_classify_event;

// Structs for ClickHouse
pub use wallet::WalletRow;
pub use transaction::TransactionRow;
pub use owner::OwnerRow;
pub use token_transfer::TokenTransferRow;
pub use token_metadata::TokenMetadataRow;
pub use sync_state::SyncStateRow;
pub use tron_raw_log::TronRawLogRow;
pub use tron_classify_event::TronClassifiedEventRow;




