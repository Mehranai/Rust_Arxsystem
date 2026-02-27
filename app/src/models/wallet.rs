use clickhouse::Row;
use serde::Serialize;

#[derive(Serialize, Row)]
pub struct WalletRow {
    pub address: String,
    pub balance: String,
    pub nonce: u64,
    #[serde(rename = "type")]
    pub wallet_type: String,
    pub person_id: String
}
