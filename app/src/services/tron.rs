use std::sync::Arc;
use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};
use serde_json::Value;

use crate::services::loader::LoaderTron;
use crate::services::progress::{
    save_sync_state,
    save_tx,
    save_token_transfer,
    save_contract_call,
};
use crate::models::token_transfer::TokenTransferRow;
use crate::models::tron_raw_log::TronRawLogRow;
use crate::models::tron_classify_event::TronClassifiedEventRow;
use crate::utils::tron_address::normalize_tron_address;

const ZERO_ADDRESS: &str = "T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb";
const TRC20_TRANSFER_SIG: &str =
    "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a9df523b3ef";

/// محاسبه حساسیت تراکنش TRX
fn calc_sensivity_trx(sun: u64) -> u8 {
    let trx = sun as f64 / 1_000_000.0;
    if trx > 100_000.0 { 2 }
    else if trx > 10_000.0 { 1 }
    else { 0 }
}

/// دیکد کردن TRC20 transfer از receipt
fn decode_trc20_from_receipt(
    receipt: &Value,
) -> Vec<(u32, String, String, String, String)> {

    let mut transfers = Vec::new();
    let logs = receipt["log"].as_array().cloned().unwrap_or_default();

    for (idx, log) in logs.into_iter().enumerate() {
        let topics = log["topics"].as_array().cloned().unwrap_or_default();
        if topics.len() < 3 { continue; }

        let topic0 = topics[0]
            .as_str()
            .unwrap_or("")
            .trim_start_matches("0x")
            .to_lowercase();

        if topic0 != TRC20_TRANSFER_SIG { continue; }

        let token_address = log["address"].as_str().unwrap_or("").to_string();

        let from = topics[1]
            .as_str()
            .unwrap_or("")
            .trim_start_matches("0x")
            .trim_start_matches("000000000000000000000000")
            .to_string();

        let to = topics[2]
            .as_str()
            .unwrap_or("")
            .trim_start_matches("0x")
            .trim_start_matches("000000000000000000000000")
            .to_string();

        let value_hex = log["data"]
            .as_str()
            .unwrap_or("0x0")
            .trim_start_matches("0x");

        let amount = u128::from_str_radix(value_hex, 16)
            .unwrap_or(0)
            .to_string();

        transfers.push((idx as u32, token_address, from, to, amount));
    }

    transfers
}

/// پردازش یک تراکنش TRON
pub async fn process_tx_tron(
    loader: Arc<LoaderTron>,
    tx: Value,
    block_number: u64,
    block_timestamp: u64,
) -> Result<()> {

    let tx_hash = tx["txID"].as_str().unwrap_or("").to_string();
    if tx_hash.is_empty() { return Ok(()); }

    let contract = match tx["raw_data"]["contract"].get(0) {
        Some(c) => c, None => return Ok(()),
    };

    let contract_type = contract["type"].as_str().unwrap_or("");
    let val = &contract["parameter"]["value"];

    let from_addr = normalize_tron_address(val["owner_address"].as_str().unwrap_or("")).unwrap_or_default();
    let to_addr   = normalize_tron_address(val["to_address"].as_str().unwrap_or("")).unwrap_or_default();
    let amount    = val["amount"].as_u64().unwrap_or(0);

    // دریافت receipt
    let receipt = loader.tron_client.get_tx_receipt(&tx_hash).await?;

    let energy_usage = receipt["receipt"]["energy_usage_total"].as_u64().unwrap_or(0);
    let net_usage = receipt["receipt"]["net_usage"].as_u64().unwrap_or(0);
    let fee = receipt["fee"].as_u64().unwrap_or(0);
    let success = if receipt["receipt"]["result"].as_str() == Some("SUCCESS") { 1 } else { 0 };

    // ذخیره TRX Transaction
    if !from_addr.is_empty() && !to_addr.is_empty() {
        save_tx(
            loader.clickhouse.clone(),
            tx_hash.clone(),
            block_number,
            from_addr.clone(),
            to_addr.clone(),
            amount.to_string(),
            calc_sensivity_trx(amount),
        ).await?;
    }

    // ذخیره TRC20 Transfers
    let trc20_transfers = decode_trc20_from_receipt(&receipt);
    for (_log_index, token, from, to, amt) in trc20_transfers {
        save_token_transfer(
            loader.clickhouse.clone(),
            TokenTransferRow {
                tx_hash: tx_hash.clone(),
                log_index: 0,
                block_number,
                block_timestamp,
                contract_address: token.clone(),
                from_address: from.clone(),
                to_address: to.clone(),
                amount: amt.clone(),
            }
        ).await?;
    }

    // ذخیره Contract Calls
    if contract_type == "TriggerSmartContract" {
        let contract_address = normalize_tron_address(val["contract_address"].as_str().unwrap_or("")).unwrap_or_default();
        let method_id = val["data"].as_str().unwrap_or("").chars().take(8).collect::<String>();

        save_contract_call(
            loader.clickhouse.clone(),
            ContractCallRow {
                tx_hash: tx_hash.clone(),
                block_number,
                block_timestamp,
                caller_address: from_addr.clone(),
                contract_address,
                method_id,
                call_value: val["call_value"].as_u64().unwrap_or(0),
                success,
            }
        ).await?;
    }

    Ok(())
}

/// Fetch و پردازش بلاک‌ها
pub async fn fetch_tron(
    loader: Arc<LoaderTron>,
    start_block: u64,
    total_txs: u64,
) -> Result<()> {

    let latest_block = loader.tron_client
        .get_now_block()
        .await?["block_header"]["raw_data"]["number"]
        .as_u64()
        .unwrap_or(start_block);

    let mut tx_count = 0;
    let mut current_block = start_block;

    while current_block <= latest_block && tx_count < total_txs {
        let block = loader.tron_client.get_block(current_block).await?;
        let txs = block["transactions"].as_array().cloned().unwrap_or_default();

        let mut tasks = FuturesUnordered::new();

        for tx in txs {
            if tx_count >= total_txs { break; }

            let loader_clone = loader.clone();
            let block_number = current_block;
            let block_timestamp = block["block_header"]["raw_data"]["timestamp"].as_u64().unwrap_or(0);

            tasks.push(tokio::spawn(async move {
                process_tx_tron(loader_clone, tx, block_number, block_timestamp).await
            }));

            tx_count += 1;
        }

        while let Some(res) = tasks.next().await { res??; }

        save_sync_state(loader.clickhouse.clone(), "tron", current_block).await?;
        current_block += 1;
    }

    Ok(())
}