use clickhouse::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, clickhouse::Row)]
pub struct SyncStateRow {
    pub chain: String,
    pub last_synced_block: u64,
}

pub async fn get_last_synced_block(
    client: &Client,
    chain: &str,
) -> anyhow::Result<Option<u64>> {
    let row = client
        .query(
            "SELECT chain, last_synced_block
             FROM sync_state
             WHERE chain = ?
             LIMIT 1",
        )
        .bind(chain)
        .fetch_optional::<SyncStateRow>()
        .await?;

    Ok(row.map(|r| r.last_synced_block))
}

pub async fn update_last_synced_block(
    client: &Client,
    chain: &str,
    block: u64,
) -> anyhow::Result<()> {
    client
        .query(
            "INSERT INTO sync_state (chain, last_synced_block)
             VALUES (?, ?)",
        )
        .bind(chain)
        .bind(block)
        .execute()
        .await?;

    Ok(())
}
