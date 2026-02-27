use serde::Serialize;
use clickhouse::Row;

#[derive(Debug, Serialize, Row)]
pub struct TronClassifiedEventRow {
    pub tx_hash: String,
    pub block_number: u64,
    pub event_type: String,
    pub primary_address: String,
    pub secondary_address: String,
    pub token_address: String,
    pub amount: String,
}