use serde::{Serialize, Deserialize};
use clickhouse::Row;

#[derive(Serialize, Row)]
pub struct TransactionRow {
    pub hash: String,
    pub block_number: u64,
    pub from_addr: String,
    pub to_addr: String,
    pub value: String,
    pub sensivity: u8,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
pub enum Sensivity {
    Red = 1,
    Yellow = 2,
    Green = 3
}
