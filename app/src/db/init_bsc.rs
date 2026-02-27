use clickhouse::Client;
use crate::db::init::run_sql;

pub async fn init_bsc_db(client: &Client) -> anyhow::Result<()> {
    let sql = include_str!("../../sql/init_database_bsc.sql");
    run_sql(client, sql).await
}
