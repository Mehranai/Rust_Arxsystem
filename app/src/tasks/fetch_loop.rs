use std::sync::Arc;
use anyhow::Result;
use clickhouse::Client;

use crate::config::AppConfig;

use crate::services::{
    loader::{LoaderEth, LoaderBtc, LoaderBsc, LoaderTron},
    bitcoin,
    ethereum,
    bsc,
    tron,
    sync_logic::{resolve_start_block_btc, resolve_start_block_evm, resolve_start_block_tron},
};

use crate::db::init_eth::init_eth_db;
use crate::db::init_btc::init_btc_db;
use crate::db::init_bsc::init_bsc_db;
use crate::db::init_tron::init_tron_db;

use crate::db::sync_state::get_last_synced_block;

pub async fn run_btc_loop(config: AppConfig) -> Result<()> {
    println!("===============================");
    println!("[BTC] Starting BTC fetch loop...");
    println!("===============================");

    // Loader (client با دیتابیس btc_db)
    let loader = Arc::new(LoaderBtc::new(&config).await?);

    // client موقت بدون database برای init
    let admin_client = Client::default()
        .with_url(&config.clickhouse_url)
        .with_user(&config.clickhouse_user)
        .with_password(&config.clickhouse_pass);

    // init دیتابیس و جدول‌ها
    init_btc_db(&admin_client).await?;

    // گرفتن آخرین بلاک sync شده از دیتابیس
    let last_synced = get_last_synced_block(&loader.clickhouse, "btc").await?;

    // تعیین start_block با توجه به sync_mode
    let start_block =
        resolve_start_block_btc(&config.sync_mode, config.btc_start_block, last_synced);

    println!(
        "[BTC] sync_mode={:?} start_block={} last_synced={:?}",
        config.sync_mode, start_block, last_synced
    );

    // شروع fetch
    bitcoin::fetch_btc(
        loader.clone(),
        start_block,
        config.total_btc_txs,
        config.btc_api_url
            .as_ref()
            .expect("BTC_API_URL is not set!"),
    )
    .await?;

    println!("[BTC] Finished successfully.");
    Ok(())
}

pub async fn run_eth_loop(config: AppConfig) -> Result<()> {
    println!("===============================");
    println!("[ETH] Starting ETH fetch loop...");
    println!("===============================");

    // Loader (client با دیتابیس eth_db)
    let loader = Arc::new(LoaderEth::new(&config).await?);

    // client موقت بدون database برای init
    let admin_client = Client::default()
        .with_url(&config.clickhouse_url)
        .with_user(&config.clickhouse_user)
        .with_password(&config.clickhouse_pass);

    // init دیتابیس و جدول‌ها
    init_eth_db(&admin_client).await?;

    // گرفتن آخرین بلاک sync شده از دیتابیس
    let last_synced = get_last_synced_block(&loader.clickhouse, "eth").await?;

    // تعیین start_block با توجه به sync_mode
    let start_block = resolve_start_block_evm(
        &config.sync_mode,
        loader.eth_provider.clone(),
        config.eth_start_block,
        last_synced,
    )
    .await?;

    println!(
        "[ETH] sync_mode={:?} start_block={} last_synced={:?}",
        config.sync_mode, start_block, last_synced
    );

    // شروع fetch
    ethereum::fetch_eth(
        loader.clone(),
        start_block,
        config.total_eth_txs,
    )
    .await?;

    println!("[ETH] Finished successfully.");
    Ok(())
}

pub async fn run_bsc_loop(config: AppConfig) -> Result<()> {
    println!("===============================");
    println!("[BSC] Starting BSC fetch loop...");
    println!("===============================");

    // Loader (client با دیتابیس bsc_db)
    let loader = Arc::new(LoaderBsc::new(&config).await?);

    // client موقت بدون database برای init
    let admin_client = Client::default()
        .with_url(&config.clickhouse_url)
        .with_user(&config.clickhouse_user)
        .with_password(&config.clickhouse_pass);

    // init دیتابیس و جدول‌ها
    init_bsc_db(&admin_client).await?;

    // گرفتن آخرین بلاک sync شده از دیتابیس
    let last_synced = get_last_synced_block(&loader.clickhouse, "bsc").await?;

    // تعیین start_block با توجه به sync_mode
    let start_block = resolve_start_block_evm(
        &config.sync_mode,
        loader.bsc_provider.clone(),
        config.bsc_start_block,
        last_synced,
    )
    .await?;

    println!(
        "[BSC] sync_mode={:?} start_block={} last_synced={:?}",
        config.sync_mode, start_block, last_synced
    );

    // شروع fetch
    bsc::fetch_bsc(
        loader.clone(),
        start_block,
        config.total_bsc_txs,
    )
    .await?;

    println!("[BSC] Finished successfully.");
    Ok(())
}


pub async fn run_tron_loop(config: AppConfig) -> Result<()> {
    println!("===============================");
    println!("[TRON] Starting TRON fetch loop...");
    println!("===============================");

    let loader = Arc::new(LoaderTron::new(&config).await?);

    let admin_client = Client::default()
        .with_url(&config.clickhouse_url)
        .with_user(&config.clickhouse_user)
        .with_password(&config.clickhouse_pass);

    init_tron_db(&admin_client).await?;
    println!("Passed Clickhouse section");

    let last_synced =
        get_last_synced_block(&loader.clickhouse, "tron").await?;

    let start_block = resolve_start_block_tron(
        &config.sync_mode,
        loader.tron_client.clone(),
        config.tron_start_block,
        last_synced,
    )
    .await?;

    println!(
        "[TRON] sync_mode={:?} start_block={} last_synced={:?}",
        config.sync_mode, start_block, last_synced
    );

    tron::fetch_tron(
        loader.clone(),
        start_block,
        config.total_tron_txs,
    )
    .await?;

    println!("[TRON] Finished successfully.");
    Ok(())
}