use std::sync::Arc;

use anyhow::Result;
use ethers::prelude::*;
use futures::stream::{FuturesUnordered, StreamExt};

use crate::models::transaction::Sensivity;
use crate::services::loader::LoaderBsc;
use crate::services::progress::{
    save_sync_state,
    save_tx,
    save_wallet,
    save_token_transfer,
};
use crate::models::token_transfer::TokenTransferRow;
use crate::services::token_metadata_worker;

const ERC20_TRANSFER_TOPIC: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

fn calc_sensivity_bsc(value_wei: U256) -> Sensivity {
    // BSC coin (BNB) هم 18 decimals
    let bnb_value = value_wei.as_u128() as f64 / 1e18;

    if bnb_value > 1000.0 {
        Sensivity::Red
    } else if bnb_value > 100.0 {
        Sensivity::Yellow
    } else {
        Sensivity::Green
    }
}

// استخراج Transfer Logs
fn extract_token_transfers(receipt: &TransactionReceipt) -> Vec<(u32, Address, Address, Address, U256)> {
    let mut transfers = Vec::new();

    let transfer_topic: H256 = ERC20_TRANSFER_TOPIC.parse().unwrap();

    for log in &receipt.logs {
        if log.topics.len() == 3 && log.topics[0] == transfer_topic {
            let token_address = log.address;

            let from = Address::from_slice(&log.topics[1].as_bytes()[12..]);
            let to = Address::from_slice(&log.topics[2].as_bytes()[12..]);

            let amount = U256::from_big_endian(&log.data.0);

            transfers.push((
                log.log_index.unwrap_or(U256::zero()).as_u32(),
                token_address,
                from,
                to,
                amount,
            ));
        }
    }

    transfers
}

async fn save_wallet_bsc(
    provider: Arc<Provider<Http>>,
    clickhouse: Arc<clickhouse::Client>,
    limiter: Arc<tokio::sync::Semaphore>,
    addr: Address,
) -> Result<()> {
    if addr == Address::zero() {
        return Ok(());
    }

    let (balance, nonce, wallet_type) = {
        let _permit = limiter.acquire().await?;

        let balance = provider.get_balance(addr, None).await?;
        let nonce = provider.get_transaction_count(addr, None).await?;
        let code = provider.get_code(addr, None).await?;

        let wallet_type = if code.0.is_empty() {
            "wallet".to_string()
        } else {
            "smart_contract".to_string()
        };

        (balance, nonce, wallet_type)
    };

    save_wallet(
        clickhouse,
        &addr.to_string(),
        balance.to_string(),
        nonce.as_u64(),
        wallet_type,
    )
    .await?;

    Ok(())
}

async fn process_tx(
    provider: Arc<Provider<Http>>,
    clickhouse: Arc<clickhouse::Client>,
    limiter: Arc<tokio::sync::Semaphore>,
    tx: Transaction,
    block_number: u64,
) -> Result<Vec<Address>> {
    let hash = format!("{:#x}", tx.hash);
    let from = tx.from;
    let to = tx.to.unwrap_or_default();
    let value = tx.value;

    save_tx(
        clickhouse.clone(),
        hash.clone(),
        block_number,
        from.to_string(),
        to.to_string(),
        value.to_string(),
        calc_sensivity_bsc(value) as u8,
    )
    .await?;

    // Receipt (Rate limited)
    let receipt_opt = {
        let _permit = limiter.acquire().await?;
        provider.get_transaction_receipt(tx.hash).await?
    };

    let mut discovered_tokens: Vec<Address> = vec![];

    if let Some(receipt) = receipt_opt {
        let transfers = extract_token_transfers(&receipt);

        for (log_index, token, from_addr, to_addr, amount) in transfers {
            discovered_tokens.push(token);

            save_token_transfer(
                clickhouse.clone(),
                TokenTransferRow {
                    tx_hash: hash.clone(),
                    block_number,
                    log_index,
                    token_address: token.to_string(),
                    from_addr: from_addr.to_string(),
                    to_addr: to_addr.to_string(),
                    amount: amount.to_string(),
                },
            )
            .await?;
        }
    }

    // Save wallet info (Rate limited)
    save_wallet_bsc(
        provider.clone(),
        clickhouse.clone(),
        limiter.clone(),
        from,
    )
    .await?;

    if tx.to.is_some() {
        save_wallet_bsc(provider, clickhouse, limiter.clone(), to).await?;
    }

    Ok(discovered_tokens)
}

pub async fn fetch_bsc(
    loader: Arc<LoaderBsc>,
    start_block: u64,
    total_txs: u64,
) -> Result<()> {

    let provider = loader.bsc_provider.clone();
    let clickhouse = loader.clickhouse.clone();
    let limiter = loader.rpc_limiter.clone();

    let latest_block = provider.get_block_number().await?.as_u64();
    println!("BSC Latest Block: {}", latest_block);

    let mut tx_count: u64 = 0;
    let mut last_synced_block: u64 = start_block;

    let mut current_block = start_block;

    while current_block <= latest_block {
        if tx_count >= total_txs {
            break;
        }

        let block_opt = {
            let _permit = limiter.acquire().await?;
            provider.get_block_with_txs(current_block).await?
        };

        let Some(block) = block_opt else {
            current_block += 1;
            continue;
        };

        let mut tasks = FuturesUnordered::new();
        let mut discovered_tokens_all: Vec<Address> = vec![];

        let block_number = current_block;
        let mut fully_processed_block = true;

        for tx in block.transactions {
            if tx_count >= total_txs {
                fully_processed_block = false;
                break;
            }

            let provider = provider.clone();
            let clickhouse = clickhouse.clone();
            let limiter = limiter.clone();

            tasks.push(tokio::spawn(async move {
                process_tx(provider, clickhouse, limiter, tx, block_number).await
            }));

            tx_count += 1;
        }

        while let Some(res) = tasks.next().await {
            let tokens = res??;
            discovered_tokens_all.extend(tokens);
        }

        // Call token metadata worker
        if !discovered_tokens_all.is_empty() {
            token_metadata_worker::process_new_tokens(
                clickhouse.clone(),
                provider.clone(),
                limiter.clone(),
                discovered_tokens_all,
            )
            .await?;
        }

        // فقط اگر بلاک کامل پردازش شد sync_state آپدیت شود
        if fully_processed_block {
            last_synced_block = current_block;

            save_sync_state(
                clickhouse.clone(),
                "bsc",
                last_synced_block,
            )
            .await?;

            println!(
                "BSC synced block {} | total tx processed {}",
                last_synced_block, tx_count
            );
        } else {
            println!(
                "BSC stopped mid-block {} (tx limit reached) | total tx processed {}",
                current_block, tx_count
            );
            break;
        }

        current_block += 1;
    }

    save_sync_state(clickhouse.clone(), "bsc", last_synced_block).await?;

    Ok(())
}
