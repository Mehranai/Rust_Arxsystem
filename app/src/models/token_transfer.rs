use clickhouse::Row;
use serde::Serialize;


#[derive(Debug, Serialize, Row)]
pub struct TokenTransferRow {
    pub tx_hash: String,
    pub block_number: u64,
    pub log_index: u32,
    pub token_address: String,
    pub from_addr: String,
    pub to_addr: String,
    pub amount: String,
}