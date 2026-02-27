use clickhouse::Row;
use serde::Serialize;

#[derive(Debug, Serialize, Row)]
pub struct SyncStateRow {
    pub chain: String,
    pub last_synced_block: u64,
}
