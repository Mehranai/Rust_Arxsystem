use std::sync::Arc;
use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};

use crate::services::loader::LoaderTron;
use crate::services::progress::{
    save_sync_state,
    save_tx,
    save_wallet,
    save_token_transfer,
    save_energy_usage,
    save_contract_call,
    save_raw_logs,
    save_classified_event,
    AddressEnergyRow,
    ContractCallRow,
};
use crate::models::token_transfer::TokenTransferRow;
use crate::models::tron_raw_log::TronRawLogRow;
use crate::models::tron_classify_event::TronClassifiedEventRow;
use crate::utils::tron_address::normalize_tron_address;
use crate::utils::tron_classification::*;

const ZERO_ADDRESS: &str = "T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb";

fn calc_sensivity_trx(sun: u64) -> u8 {
    let trx = sun as f64 / 1_000_000.0;
    if trx > 100_000.0 { 2 }
    else if trx > 10_000.0 { 1 }
    else { 0 }
}

// ===============================
// TRC20 Receipt Decoder
// ===============================

const TRC20_TRANSFER_SIG: &str =
    "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a9df523b3ef";

fn decode_trc20_from_receipt(
    receipt: &serde_json::Value,
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

// ---------------- RAW LOGS ----------------
async fn ingest_raw_logs(
    loader: &LoaderTron,
    tx_hash: &str,
    block_number: u64,
    receipt: &serde_json::Value,
) -> Result<()> {

    let logs = receipt["log"].as_array().cloned().unwrap_or_default();
    let mut rows = Vec::with_capacity(logs.len());

    for (idx, log) in logs.into_iter().enumerate() {
        let topics = log["topics"].as_array().cloned().unwrap_or_default();

        rows.push(TronRawLogRow {
            tx_hash: tx_hash.to_string(),
            block_number,
            log_index: idx as u32,
            contract_address: normalize_tron_address(
                log["address"].as_str().unwrap_or("")
            ).unwrap_or_default(),
            topic0: topics.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            topic1: topics.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            topic2: topics.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            topic3: topics.get(3).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            data: log["data"].as_str().unwrap_or("").to_string(),
        });
    }

    save_raw_logs(loader.clickhouse.clone(), rows).await?;
    Ok(())
}

// ---------------- PROCESS TX ----------------
async fn process_tx_tron(
    loader: Arc<LoaderTron>,
    tx: serde_json::Value,
    block_number: u64,
) -> Result<()> {

    let tx_hash = tx["txID"].as_str().unwrap_or("").to_string();
    if tx_hash.is_empty() { return Ok(()); }

    let contract = match tx["raw_data"]["contract"].get(0) {
        Some(c) => c, None => return Ok(()),
    };

    let contract_type = contract["type"].as_str().unwrap_or("").to_string();
    let val = &contract["parameter"]["value"];

    let from_addr = val["owner_address"].as_str().unwrap_or("").to_string();
    let to_addr   = val["to_address"].as_str().unwrap_or("").to_string();
    let amount    = val["amount"].as_u64().unwrap_or(0);

    // ---------------- SAVE TRX ----------------
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

    // ---------------- RECEIPT ----------------
    let receipt = loader.tron_client.get_tx_receipt(&tx_hash).await?;

    // 1️⃣ RAW LOGS
    ingest_raw_logs(&loader, &tx_hash, block_number, &receipt).await?;

    // 2️⃣ DECODE TRC20
    let trc20_transfers = decode_trc20_from_receipt(&receipt);

    let mut collected: Vec<SimpleTransfer> = Vec::new();

    for (_log_index, contract, from, to, amount_str) in trc20_transfers {
        let token_address = normalize_tron_address(&contract).unwrap_or(contract);
        let from_addr = normalize_tron_address(&from).unwrap_or(from);
        let to_addr = normalize_tron_address(&to).unwrap_or(to);
        let amount: u128 = amount_str.parse().unwrap_or(0);

        collected.push(SimpleTransfer {
            token: token_address.clone(),
            from: from_addr.clone(),
            to: to_addr.clone(),
            amount,
        });

        // Save raw transfer
        save_token_transfer(
            loader.clickhouse.clone(),
            TokenTransferRow {
                tx_hash: tx_hash.clone(),
                block_number,
                log_index: 0, // میتونی index دقیق هم بذاری
                token_address: token_address.clone(),
                from_addr: from_addr.clone(),
                to_addr: to_addr.clone(),
                amount: amount.to_string(),
            }
        ).await?;
    }

    // ---------------- ENERGY ----------------
    let energy_used = receipt["receipt"]["energy_usage_total"].as_u64().unwrap_or(0);
    let result = receipt["receipt"]["result"].as_str().unwrap_or("UNKNOWN").to_string();

    if !from_addr.is_empty() {
        save_energy_usage(
            loader.clickhouse.clone(),
            AddressEnergyRow {
                address: from_addr.clone(),
                block_number,
                energy_usage: energy_used,
                energy_fee: receipt["receipt"]["energy_fee"].as_u64().unwrap_or(0),
                net_usage: receipt["receipt"]["net_usage"].as_u64().unwrap_or(0),
                net_fee: receipt["receipt"]["net_fee"].as_u64().unwrap_or(0),
                tx_hash: tx_hash.clone(),
            }
        ).await?;
    }

    // ---------------- CONTRACT CALL ----------------
    if contract_type == "TriggerSmartContract" {
        let contract_address = val["contract_address"].as_str().unwrap_or("").to_string();
        let method_sig = val["data"].as_str().unwrap_or("").chars().take(8).collect::<String>();

        save_contract_call(
            loader.clickhouse.clone(),
            ContractCallRow {
                tx_hash: tx_hash.clone(),
                block_number,
                caller_address: from_addr.clone(),
                contract_address,
                contract_type: contract_type.clone(),
                method_signature: method_sig,
                call_value: val["call_value"].as_u64().unwrap_or(0).to_string(),
                energy_used,
                result,
            }
        ).await?;
    }

    // ---------------- ADVANCED DETECTION ----------------
    let swaps = detect_swaps_advanced(&collected);
    let bridges = detect_bridges(&collected);

    // Decide one final type
    let tx_type = if !bridges.is_empty() {
        "bridge"
    } else if !swaps.is_empty() {
        "swap"
    } else if !collected.is_empty() {
        "transfer"
    } else {
        "unknown"
    };

    save_classified_event(
        loader.clickhouse.clone(),
        TronClassifiedEventRow {
            tx_hash: tx_hash.clone(),
            block_number,
            event_type: tx_type.to_string(),
            primary_address: from_addr.clone(),
            secondary_address: to_addr.clone(),
            token_address: "".to_string(),
            amount: "".to_string(),
        }
    ).await?;

    // ---------------- SAVE WALLETS ----------------
    if from_addr != ZERO_ADDRESS && !from_addr.is_empty() {
        save_wallet(loader.clickhouse.clone(), &from_addr, "0".into(), 0, "wallet".into()).await?;
    }
    if to_addr != ZERO_ADDRESS && !to_addr.is_empty() {
        save_wallet(loader.clickhouse.clone(), &to_addr, "0".into(), 0, "wallet".into()).await?;
    }

    Ok(())
}

// ---------------- FETCH BLOCKS ----------------
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

        for mut tx in txs {
            if tx_count >= total_txs { break; }

            // Normalize addresses
            if let Some(contract) = tx["raw_data"]["contract"].get_mut(0) {
                let val = &mut contract["parameter"]["value"];

                if let Some(addr) = val["owner_address"].as_str() {
                    if let Some(norm) = normalize_tron_address(addr) {
                        val["owner_address"] = serde_json::Value::String(norm);
                    }
                }
                if let Some(addr) = val["to_address"].as_str() {
                    if let Some(norm) = normalize_tron_address(addr) {
                        val["to_address"] = serde_json::Value::String(norm);
                    }
                }
                if let Some(addr) = val["contract_address"].as_str() {
                    if let Some(norm) = normalize_tron_address(addr) {
                        val["contract_address"] = serde_json::Value::String(norm);
                    }
                }
            }

            let loader_clone = loader.clone();
            tasks.push(tokio::spawn(async move {
                process_tx_tron(loader_clone, tx, current_block).await
            }));

            tx_count += 1;
        }

        while let Some(res) = tasks.next().await {
            res??;
        }

        save_sync_state(loader.clickhouse.clone(), "tron", current_block).await?;
        current_block += 1;
    }

    Ok(())
}

// ---------------- CLASSIFY ----------------
fn classify_trc20_transfer(from: &str, to: &str) -> &'static str {
    if from == ZERO_ADDRESS { return "mint"; }
    if to == ZERO_ADDRESS { return "burn"; }
    "transfer"
}