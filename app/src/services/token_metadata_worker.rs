use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use clickhouse::Client;
use ethers::contract::abigen;
use ethers::providers::Middleware;
use ethers::types::Address;
use tokio::sync::Semaphore;

use crate::models::token_metadata::TokenMetadataRow;
use crate::services::progress::save_token_metadata;

// ERC20 ABI
abigen!(
    ERC20Contract,
    r#"[
        function name() view returns (string)
        function symbol() view returns (string)
        function decimals() view returns (uint8)
        function totalSupply() view returns (uint256)
    ]"#
);

pub async fn process_new_tokens<M: Middleware + 'static>(
    clickhouse: Arc<Client>,
    provider: Arc<M>,
    limiter: Arc<Semaphore>,
    discovered_tokens: Vec<Address>,
) -> Result<()> {
    if discovered_tokens.is_empty() {
        return Ok(());
    }

    // Deduplicate tokens
    let mut unique_tokens: HashSet<Address> = HashSet::new();
    for t in discovered_tokens {
        unique_tokens.insert(t);
    }

    for token_address in unique_tokens {
        let token_str = format!("{:?}", token_address);

        // check if token already exists
        let exists: u64 = clickhouse
            .query(
                "SELECT count()
                 FROM token_metadata
                 WHERE token_address = ?
                 LIMIT 1",
            )
            .bind(&token_str)
            .fetch_one::<u64>()
            .await?;

        if exists > 0 {
            continue;
        }

        let _permit = limiter.acquire().await?;

        let contract = ERC20Contract::new(token_address, provider.clone());

        // fetch metadata (safe calls)
        let name = contract.name().call().await.unwrap_or_else(|_| "UNKNOWN".into());
        let symbol = contract.symbol().call().await.unwrap_or_else(|_| "UNKNOWN".into());
        let decimals = contract.decimals().call().await.unwrap_or(0u8);

        let total_supply = contract
            .total_supply()
            .call()
            .await
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "0".to_string());

        let row = TokenMetadataRow {
            token_address: token_str,
            name,
            symbol,
            decimals,
            total_supply,
            is_verified: 1, // فعلا همیشه 1 میذاریم (بعدا میشه verify logic اضافه کرد)
        };

        save_token_metadata(clickhouse.clone(), row).await?;
    }

    Ok(())
}
