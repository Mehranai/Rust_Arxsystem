use ethers::providers::{Http, Middleware, Provider};
use std::sync::Arc;

use crate::config::SyncMode;
use crate::helper::tron::TronClient;

pub async fn resolve_start_block_evm(
    sync_mode: &SyncMode,
    provider: Arc<Provider<Http>>,
    config_start_block: u64,
    last_synced: Option<u64>,
) -> anyhow::Result<u64> {
    match sync_mode {
        SyncMode::Backfill => Ok(config_start_block),

        SyncMode::Live => {
            let latest = provider.get_block_number().await?.as_u64();
            Ok(latest.saturating_sub(10)) // safety window
        }

        SyncMode::Auto => {
            if let Some(last) = last_synced {
                Ok(last + 1)
            } else {
                Ok(config_start_block)
            }
        }
    }
}

pub fn resolve_start_block_btc(
    sync_mode: &SyncMode,
    config_start_block: u64,
    last_synced: Option<u64>,
) -> u64 {
    match sync_mode {
        SyncMode::Backfill => config_start_block,
        SyncMode::Live => config_start_block, // BTC live mode یعنی از start_block شروع کن
        SyncMode::Auto => {
            if let Some(last) = last_synced {
                last + 1
            } else {
                config_start_block
            }
        }
    }
}

pub async fn resolve_start_block_tron(
    sync_mode: &SyncMode,
    tron_client: Arc<TronClient>,
    config_start_block: u64,
    last_synced: Option<u64>,
) -> anyhow::Result<u64> {
    match sync_mode {
        SyncMode::Backfill => Ok(config_start_block),

        SyncMode::Live => {
            let latest = tron_client.get_block_number().await?;
            Ok(latest.saturating_sub(20))
        }

        SyncMode::Auto => {
            if let Some(last) = last_synced {
                Ok(last + 1)
            } else {
                Ok(config_start_block)
            }
        }
    }
}
