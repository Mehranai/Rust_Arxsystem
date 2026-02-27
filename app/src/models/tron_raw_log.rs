use serde::Serialize;
use clickhouse::Row;

#[derive(Debug, Serialize, Row)]
pub struct TronRawLogRow {
    pub tx_hash: String,
    pub block_number: u64,
    pub log_index: u32,
    pub contract_address: String,
    pub topic0: String,
    pub topic1: String,
    pub topic2: String,
    pub topic3: String,
    pub data: String,
}