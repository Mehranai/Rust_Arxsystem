use std::sync::Arc;

use anyhow::{Result, anyhow};
use futures::stream::{FuturesUnordered, StreamExt};

use crate::services::loader::LoaderTron;
use crate::services::progress::save_sync_state;
use crate::models::transaction::TransactionRow;
use crate::models::token_transfer::TokenTransferRow;

use crate::utils::tron_address::normalize_tron_address;

const TRC20_TRANSFER_TOPIC: &str =
    "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

// -------------------------------------
// Sensitivity (basic version)
// -------------------------------------
fn calc_sensivity_tron(amount: u128) -> u8 {
    if amount > 10_000_000_000 { // ~10k USDT (6 decimals)
        2
    } else if amount > 1_000_000_000 {
        1
    } else {
        0
    }
}

// -------------------------------------
// Address normalize (HEX -> base58 later)
// -------------------------------------
fn normalize_address(addr: &str) -> String {
    addr.to_string() // TODO: convert hex → base58
}

// -------------------------------------
// Extract TRC20 logs
// -------------------------------------
fn extract_trc20_transfers(
    receipt: &serde_json::Value,
    block_number: u64,
    tx_hash: &str,
) -> Vec<TokenTransferRow> {

    let mut result = vec![];

    let logs: &[serde_json::Value] = receipt["log"]
        .as_array()
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    for (i, log) in logs.iter().enumerate() {

        let topics = match log["topics"].as_array() {
            Some(t) if t.len() >= 3 => t,
            _ => continue,
        };

        let topic0 = topics[0].as_str().unwrap_or("");

        if !topic0.contains(TRC20_TRANSFER_TOPIC) {
            continue;
        }

        let from = normalize_tron_address(
            topics[1].as_str().unwrap_or("")
        ).unwrap_or_default();

        let to = normalize_tron_address(
            topics[2].as_str().unwrap_or("")
        ).unwrap_or_default();

        let data = log["data"].as_str().unwrap_or("0x0");

        let amount = u128::from_str_radix(
            data.trim_start_matches("0x"),
            16,
        ).unwrap_or(0);

        let token = normalize_tron_address(
            log["address"].as_str().unwrap_or("")
        ).unwrap_or_default();

        result.push(TokenTransferRow {
            tx_hash: tx_hash.to_string(),
            block_number,
            log_index: i as u32,
            token_address: token,
            from_addr: from,
            to_addr: to,
            amount: amount.to_string(),
        });
    }

    result
}

// -------------------------------------
// Process single tx
// -------------------------------------
async fn process_tx(
    loader: Arc<LoaderTron>,
    tx: &serde_json::Value,
    block_number: u64,
) -> Result<(TransactionRow, Vec<TokenTransferRow>)> {

    let raw = &tx["raw_data"]["contract"][0];

    let contract_type = raw["type"].as_str().unwrap_or("");

    let parameter = &raw["parameter"]["value"];

    let tx_hash = tx["txID"]
        .as_str()
        .ok_or_else(|| anyhow!("missing tx hash"))?
        .to_string();

    let mut from = "";
    let mut to = "";
    let mut amount: u128 = 0;

    match contract_type {
        "TransferContract" => {
            from = parameter["owner_address"].as_str().unwrap_or("");
            to = parameter["to_address"].as_str().unwrap_or("");
            amount = parameter["amount"].as_u64().unwrap_or(0) as u128;
        }

        "TriggerSmartContract" => {
            from = parameter["owner_address"].as_str().unwrap_or("");
            to = parameter["contract_address"].as_str().unwrap_or("");
        }

        _ => {}
    }

    // receipt (rate limited)
    let mut token_transfers = vec![];

    if contract_type == "TriggerSmartContract" {

        let receipt = {
            let _permit = loader.rpc_limiter.acquire().await?;
            loader.tron_client.get_tx_receipt(&tx_hash).await?
        };

        token_transfers = extract_trc20_transfers(
            &receipt,
            block_number,
            &tx_hash,
        );
    }

    let tx_row = TransactionRow {
        hash: tx_hash.clone(),
        block_number,
        from_addr: normalize_tron_address(from).unwrap_or_default(),
        to_addr: normalize_tron_address(to).unwrap_or_default(),
        value: amount.to_string(),
        sensivity: calc_sensivity_tron(amount),
    };

    Ok((tx_row, token_transfers))
}

// -------------------------------------
// MAIN FETCH
// -------------------------------------
pub async fn fetch_tron(
    loader: Arc<LoaderTron>,
    start_block: u64,
    total_txs: u64,
) -> Result<()> {

    let tron = loader.tron_client.clone();
    let clickhouse = loader.clickhouse.clone();

    let latest_block = tron.get_block_number().await?;
    println!("TRON Latest Block: {}", latest_block);

    let mut tx_count = 0;
    let mut current_block = start_block;
    let mut last_synced_block = start_block;

    // batching buffers
    let mut tx_batch: Vec<TransactionRow> = vec![];
    let mut transfer_batch: Vec<TokenTransferRow> = vec![];

    const BATCH_SIZE: usize = 5000;

    while current_block <= latest_block {

        if tx_count >= total_txs {
            break;
        }

        let block = {
            let _permit = loader.rpc_limiter.acquire().await?;
            tron.get_block(current_block).await?
        };

        let txs: &[serde_json::Value] = block["transactions"]
            .as_array()
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        if txs.is_empty() {
            last_synced_block = current_block;
            save_sync_state(clickhouse.clone(), "tron", last_synced_block).await?;
            current_block += 1;
            continue;
        }

        let mut tasks = FuturesUnordered::new();

        for tx in txs {

            if tx_count >= total_txs {
                break;
            }

            let loader = loader.clone();
            let tx = tx.clone();
            let block_number = current_block;

            tasks.push(tokio::spawn(async move {
                process_tx(loader, &tx, block_number).await
            }));

            tx_count += 1;
        }

        while let Some(res) = tasks.next().await {
            let (tx_row, transfers) = res??;

            tx_batch.push(tx_row);
            transfer_batch.extend(transfers);

            // flush batch
            if tx_batch.len() >= BATCH_SIZE {

                // insert tx batch
                let mut insert_tx = clickhouse
                    .insert::<TransactionRow>("transactions")
                    .await?;

                for row in tx_batch.drain(..) {
                    insert_tx.write(&row).await?;
                }

                insert_tx.end().await?;

                // insert transfers
                if !transfer_batch.is_empty() {
                    let mut insert_tr = clickhouse
                        .insert::<TokenTransferRow>("token_transfers")
                        .await?;

                    for row in transfer_batch.drain(..) {
                        insert_tr.write(&row).await?;
                    }

                    insert_tr.end().await?;
                }

                println!("[TRON] flushed batch");
            }
        }

        last_synced_block = current_block;

        save_sync_state(clickhouse.clone(), "tron", last_synced_block).await?;

        println!(
            "[TRON] synced block {} | total tx {}",
            last_synced_block, tx_count
        );

        current_block += 1;
    }

    // final flush
    if !tx_batch.is_empty() {
        let mut insert_tx = clickhouse
            .insert::<TransactionRow>("transactions")
            .await?;

        for row in tx_batch {
            insert_tx.write(&row).await?;
        }

        insert_tx.end().await?;
    }

    if !transfer_batch.is_empty() {
        let mut insert_tr = clickhouse
            .insert::<TokenTransferRow>("token_transfers")
            .await?;

        for row in transfer_batch {
            insert_tr.write(&row).await?;
        }

        insert_tr.end().await?;
    }

    save_sync_state(clickhouse.clone(), "tron", last_synced_block).await?;

    println!("[TRON] Finished");

    Ok(())
}