use clickhouse::Row;
use serde::Serialize;

#[derive(Row, Serialize)]
pub struct OwnerRow {
    pub address: String,
    pub person_name: String,
    pub person_id: String,
    pub personal_id: u16,
}
