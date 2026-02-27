use crate::services::loader::LoaderBtc;
use crate::models::transaction::Sensivity;
use crate::services::progress::{save_tx, save_wallet};
use crate::models::blockstreams::*;

use crate::db::sync_state::{get_last_synced_block, update_last_synced_block};

use clickhouse::Client;
use std::sync::Arc;
use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};

// helper functions ---------------
fn calc_sensivity_btc(value: f64) -> Sensivity {
    if value > 100.0 {
        Sensivity::Red
    } else if value > 10.0 {
        Sensivity::Yellow
    } else {
        Sensivity::Green
    }
}

fn btc_from_sats(sats: u64) -> f64 {
    sats as f64 / 100_000_000.0
}
// --------------------------------

pub async fn fetch_btc(
    loader: Arc<LoaderBtc>,
    start_block: u64,
    total_txs: u64,
    base_url: &str,
) -> Result<()> {
    let clickhouse = loader.clickhouse.clone();

    // latest block واقعی شبکه
    let latest_height = get_latest_btc_height(base_url).await?;
    println!("BTC latest height: {}", latest_height);

    // ادامه از sync_state
    let last_synced = get_last_synced_block(&clickhouse, "btc").await?;
    let mut current_height = last_synced.unwrap_or(start_block);

    let mut tx_count: u64 = 0;

    while current_height <= latest_height {
        if tx_count >= total_txs {
            break;
        }

        println!("BTC processing block: {}", current_height);

        let block_hash = get_block_hash_by_height(base_url, current_height).await?;
        let txs = get_block_txs(base_url, &block_hash).await?;

        let mut tasks = FuturesUnordered::new();

        for tx in txs {
            if tx_count >= total_txs {
                break;
            }

            let clickhouse = Arc::clone(&clickhouse);

            tasks.push(tokio::spawn(async move {
                process_tx(clickhouse, tx, current_height).await?;
                Ok::<(), anyhow::Error>(())
            }));

            tx_count += 1;
            println!("Added BTC tx #{}", tx_count);
        }

        while let Some(res) = tasks.next().await {
            res??;
        }

        // بعد از کامل شدن بلاک، sync_state رو update کن
        update_last_synced_block(&clickhouse, "btc", current_height).await?;
        println!("BTC synced block: {}", current_height);

        current_height += 1;
    }

    Ok(())
}

async fn process_tx(
    clickhouse: Arc<Client>,
    tx: BlockTx,
    block_number: u64,
) -> Result<()> {
    // اولین آدرس ورودی
    let from_addr = tx
        .vin
        .iter()
        .filter_map(|v| v.prevout.as_ref()?.scriptpubkey_address.clone())
        .next()
        .unwrap_or_default();

    // اولین آدرس خروجی
    let to_addr = tx
        .vout
        .iter()
        .filter_map(|v| v.scriptpubkey_address.clone())
        .next()
        .unwrap_or_default();

    // مقدار کل خروجی‌ها
    let total_value_sats: u64 = tx.vout.iter().map(|v| v.value).sum();
    let total_value = btc_from_sats(total_value_sats);

    save_tx(
        clickhouse.clone(),
        tx.txid.clone(),
        block_number,
        from_addr.clone(),
        to_addr.clone(),
        total_value.to_string(),
        calc_sensivity_btc(total_value) as u8,
    )
    .await?;

    // در BTC nonce نداریم → صفر
    save_wallet(
        clickhouse.clone(),
        &from_addr,
        total_value.to_string(),
        0,
        "wallet".to_string(),
    )
    .await?;

    save_wallet(
        clickhouse.clone(),
        &to_addr,
        total_value.to_string(),
        0,
        "wallet".to_string(),
    )
    .await?;

    Ok(())
}

// API Helper ------------------------------

async fn get_latest_btc_height(base_url: &str) -> Result<u64> {
    let url = format!("{}/blocks/tip/height", base_url);
    let text = reqwest::get(&url).await?.text().await?;
    Ok(text.trim().parse::<u64>()?)
}

async fn get_block_hash_by_height(base_url: &str, height: u64) -> Result<String> {
    let url = format!("{}/block-height/{}", base_url, height);
    Ok(reqwest::get(&url).await?.text().await?.trim().to_string())
}

async fn get_block_txs(base_url: &str, block_hash: &str) -> Result<Vec<BlockTx>> {
    let mut all_txs = Vec::new();
    let mut last_seen_txid: Option<String> = None;

    loop {
        let url = match &last_seen_txid {
            Some(txid) => format!("{}/block/{}/txs/{}", base_url, block_hash, txid),
            None => format!("{}/block/{}/txs", base_url, block_hash),
        };

        let resp = reqwest::get(&url).await?;
        let body_text = resp.text().await?;

        let txs: Vec<BlockTx> = serde_json::from_str(&body_text)?;

        if txs.is_empty() {
            break;
        }

        last_seen_txid = Some(txs.last().unwrap().txid.clone());
        all_txs.extend(txs);
    }

    Ok(all_txs)
}
