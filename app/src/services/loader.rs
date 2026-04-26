use clickhouse::Client;
use ethers::prelude::*;
use std::sync::Arc;
use tokio::sync::Semaphore;
use anyhow::Result;

use crate::helper::tron::TronClient;

pub struct LoaderEth {
    pub clickhouse: Arc<Client>,
    pub eth_provider: Arc<Provider<Http>>,
    pub rpc_limiter: Arc<Semaphore>,
}

impl LoaderEth{
    pub async fn new(config: &crate::config::AppConfig) -> anyhow::Result<Self> {
        let clickhouse = Arc::new(
            Client::default()
                //.with_url("tcp://clickhouse:9000")
                .with_url(&config.clickhouse_url)
                .with_user(&config.clickhouse_user)
                .with_password(&config.clickhouse_pass)
                .with_database(&config.clickhouse_db_eth)
        );

        let eth_rpc_url = config
            .eth_rpc_url
            .as_ref()
            .expect("ETH_RPC_HTTP must be set for eth mode");

        let rpc_limiter = Arc::new(Semaphore::new(config.rpc_max_concurrency));

        let eth_provider = Arc::new(
            Provider::<Http>::try_from(eth_rpc_url.as_str())?
        );

        Ok(Self {
            clickhouse,
            eth_provider,
            rpc_limiter,
        })
    }
}

pub struct LoaderBtc {
    pub clickhouse: Arc<Client>,
}

impl LoaderBtc {
    pub async fn new(config: &crate::config::AppConfig) -> anyhow::Result<Self> {
        let clickhouse = Arc::new(
                Client::default()
                    .with_url(&config.clickhouse_url)
                    .with_user(&config.clickhouse_user)
                    .with_password(&config.clickhouse_pass)
                    .with_database(&config.clickhouse_db_btc),
            );

        Ok(Self {
            clickhouse
        })
    }
}


pub struct LoaderBsc {
    pub clickhouse: Arc<Client>,
    pub bsc_provider: Arc<Provider<Http>>,
    pub rpc_limiter: Arc<Semaphore>,
}

impl LoaderBsc {
    pub async fn new(config: &crate::config::AppConfig) -> anyhow::Result<Self> {
        let clickhouse = Arc::new(
            Client::default()
                .with_url(&config.clickhouse_url)
                .with_user(&config.clickhouse_user)
                .with_password(&config.clickhouse_pass)
                .with_database(&config.clickhouse_db_bsc),
        );

        let bsc_rpc_url = config
            .bsc_rpc_url
            .as_ref()
            .expect("BSC_RPC_HTTP must be set for bsc mode");

        let bsc_provider = Arc::new(
            Provider::<Http>::try_from(bsc_rpc_url.as_str())?
        );

        let rpc_limiter = Arc::new(Semaphore::new(config.rpc_max_concurrency));

        Ok(Self {
            clickhouse,
            bsc_provider,
            rpc_limiter,
        })
    }
}

pub struct LoaderTron {
    pub clickhouse: Arc<Client>,
    pub tron_client: Arc<TronClient>,
    pub rpc_limiter: Arc<Semaphore>,
}

impl LoaderTron {
    pub async fn new(config: &crate::config::AppConfig) -> Result<Self> {
        // ClickHouse (tron_db)
        let clickhouse = Arc::new(
            Client::default()
                .with_url(&config.clickhouse_url)
                .with_user(&config.clickhouse_user)
                .with_password(&config.clickhouse_pass)
                .with_database(&config.clickhouse_db_tron),
        );

        // Tron RPC
        let tron_rpc_url = config
            .tron_rpc_url
            .as_ref()
            .expect("TRON RPC must be set");

        let tron_client = Arc::new(
            TronClient::new(
                tron_rpc_url,
                config.tron_api_key.clone(),
            )?
        );

        // Rate limiter
        let rpc_limiter = Arc::new(
            Semaphore::new(config.rpc_max_concurrency)
        );

        Ok(Self {
            clickhouse,
            tron_client,
            rpc_limiter,
        })
    }
}