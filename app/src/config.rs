use std::env;

#[derive(Debug, Clone)]
pub enum AppMode {
    Eth,
    Btc,
    Bsc,
    Tron,
}

#[derive(Debug, Clone)]
pub enum SyncMode {
    Backfill,
    Live,
    Auto,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mode: AppMode,
    pub sync_mode: SyncMode,

    pub clickhouse_url: String,
    pub clickhouse_user: String,
    pub clickhouse_pass: String,

    pub clickhouse_db_eth: String,
    pub clickhouse_db_btc: String,
    pub clickhouse_db_bsc: String,
    pub clickhouse_db_tron: String,

    pub eth_rpc_url: Option<String>,
    pub bsc_rpc_url: Option<String>,
    pub btc_api_url: Option<String>,
    pub tron_rpc_url: Option<String>,
    pub tron_api_key: Option<String>,
    //pub btc_api_url: String,

    pub btc_start_block: u64,
    pub eth_start_block: u64,
    pub bsc_start_block: u64,
    pub tron_start_block: u64,

    pub total_btc_txs: u64,
    pub total_eth_txs: u64,
    pub total_bsc_txs: u64,
    pub total_tron_txs: u64,

    // rate limit
    pub rpc_timeout_seconds: u64,
    pub rpc_max_concurrency: usize,
}

// impl AppConfig {
//     pub fn from_env() -> Self {
//         let mode = match env::var("APP_MODE").as_deref() {
//             Ok("eth") => AppMode::Eth,
//             Ok("btc") => AppMode::Btc,
//             Ok("bsc") => AppMode::Bsc,
//             _ => panic!("APP_MODE must be one of these:\neth, bsc or btc"),
//         };

//         let sync_mode = match env::var("SYNC_MODE").as_deref() {
//             Ok("backfill") => SyncMode::Backfill,
//             Ok("live") => SyncMode::Live,
//             _ => SyncMode::Auto,
//         };

        

//         Self {
//             mode,
//             sync_mode,
//             clickhouse_url: env::var("CLICKHOUSE_URL").expect("Clickhouse URL Faild"),
//             clickhouse_user: env::var("CLICKHOUSE_USER").expect("Clickhouse Username Faild"),
//             clickhouse_pass: env::var("CLICKHOUSE_PASSWORD").expect("Clickhouse Password Faild"),

//             clickhouse_db_eth: env::var("CLICKHOUSE_DB_ETH").expect("Clickhouse DB ETH Faild").into(),
//             clickhouse_db_btc: env::var("CLICKHOUSE_DB_BTC").expect("Clickhouse DB BTC Faild").into(),
//             clickhouse_db_bsc: env::var("CLICKHOUSE_DB_BSC").expect("Clickhouse DB BSC Faild").into(),

//             eth_rpc_url: env::var("ETH_RPC_HTTP").ok(),
//             btc_api_url: env::var("BTC_API_URL").ok(),
//             bsc_rpc_url: env::var("BSC_RPC_HTTP").ok(),

//             btc_start_block: env::var("BTC_START_BLOCK").expect("Clickhouse BTC start block Faild").parse().expect("Cannot Parse String to int"),
//             eth_start_block: env::var("ETH_START_BLOCK").expect("Clickhouse ETH Start block Faild").parse().expect("Cannot Parse U64"),
//             bsc_start_block: env::var("BSC_START_BLOCK").expect("Clickhouse BSC Start block Faild").parse().expect("Cannot Parse U64"),
//             total_btc_txs: env::var("TOTAL_BTC_TXS").expect("Clickhouse totla btc Faild").parse().expect("cannot pase int"),
//             total_eth_txs: env::var("TOTAL_ETH_TXS").expect("Clickhouse total eth Faild").parse().expect("Cannot parse intss"),
//             total_bsc_txs: env::var("TOTAL_BSC_TXS").expect("Clickhouse total bsc Faild").parse().expect("Cannot parse intss"),

//             rpc_timeout_seconds: env::var("RPC_TIMEOUT_SECONDS")
//             .unwrap_or("120".into())
//             .parse()
//             .expect("Cannot parse RPC_TIMEOUT_SECONDS"),

//             rpc_max_concurrency: env::var("RPC_MAX_CONCURRENCY")
//                 .unwrap_or("10".into())
//                 .parse()
//                 .expect("Cannot parse RPC_MAX_CONCURRENCY"),
//         }
//     }
// }

// Test
impl AppConfig {
    pub fn from_env() -> Self {
        let mode = AppMode::Eth;
        let sync_mode: SyncMode = SyncMode::Backfill;
        Self {
            mode,
            sync_mode,
            clickhouse_url: "http://localhost:8123".into(),
            clickhouse_user: "admin".into(),
            clickhouse_pass: "mehran.admin".into(),

            clickhouse_db_eth:"eth_db".into(),
            clickhouse_db_btc:"btc_db".into(),
            clickhouse_db_bsc:"bsc_db".into(),
            clickhouse_db_tron:"tron_db".into(),
            
            //eth_rpc_url: Some("http://localhost:8545".into()),
            eth_rpc_url: Some("https://rpc.ankr.com/eth/a4ce905377a7aa94ded62bf6efb50b20acde76159d163f8de77a16ec6237137b".into()),
            btc_api_url: Some("https://blockstream.info/api".into()),
            bsc_rpc_url: Some("https://rpc.ankr.com/bsc/a4ce905377a7aa94ded62bf6efb50b20acde76159d163f8de77a16ec6237137b".into()),
            tron_rpc_url: Some("https://api.trongrid.io".into()),
            tron_api_key : Some("737a354f-36da-4255-be84-7d6efacff25a".into()),

            btc_start_block: 831000,
            eth_start_block: 90000,
            bsc_start_block: 15000000,
            tron_start_block: 55000000,

            total_btc_txs: 500,
            total_eth_txs: 500,
            total_bsc_txs: 500,
            total_tron_txs: 200,

            rpc_timeout_seconds: 120,
            rpc_max_concurrency: 10,
        }
    }
}