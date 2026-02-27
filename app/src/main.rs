use anyhow::Result;
use tokio;

use arz_axum_for_services::config::{AppConfig, AppMode};
use arz_axum_for_services::tasks::fetch_loop::{
    run_btc_loop,
    run_eth_loop,
    run_bsc_loop,
    run_tron_loop
};

#[tokio::main]
async fn main() -> Result<()> {

    //Testing Evn inputs
    //println!("{:?}", std::env::vars().collect::<Vec<_>>());
    let config = AppConfig::from_env();

    match config.mode {
        AppMode::Btc => {
            run_btc_loop(config).await?;
        }
        AppMode::Eth => {
            run_eth_loop(config).await?;
        }
        AppMode::Bsc => {
            run_bsc_loop(config).await?;
        }
        AppMode::Tron => {
            run_tron_loop(config).await?;
        }
    }

    Ok(())
}