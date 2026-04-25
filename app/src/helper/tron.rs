use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct TronClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl TronClient {
    pub fn new(base_url: &str, api_key: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .context("Failed to build reqwest client")?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    async fn post(&self, endpoint: &str, body: Value) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, endpoint);

        let mut request = self.client.post(&url).json(&body);

        if let Some(key) = &self.api_key {
            request = request.header("TRON-PRO-API-KEY", key);
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("HTTP request failed: {}", endpoint))?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(anyhow!(
                "Tron API error | endpoint={} | status={} | body={}",
                endpoint,
                status,
                text
            ));
        }

        let parsed: Value = serde_json::from_str(&text)
            .with_context(|| format!("Invalid JSON from endpoint: {}", endpoint))?;

        Ok(parsed)
    }

    // --------------------------------------------------
    // PUBLIC API METHODS
    // --------------------------------------------------

    pub async fn get_block(&self, number: u64) -> Result<Value> {
        self.post(
            "wallet/getblockbynum",
            serde_json::json!({ "num": number }),
        )
        .await
    }

    pub async fn get_now_block(&self) -> Result<Value> {
        self.post("wallet/getnowblock", serde_json::json!({}))
            .await
    }

    pub async fn get_tx_receipt(&self, tx_hash: &str) -> Result<Value> {
        self.post(
            "wallet/gettransactioninfobyid",
            serde_json::json!({ "value": tx_hash }),
        )
        .await
    }

    pub async fn get_account(&self, address: &str) -> Result<Value> {
        self.post(
            "wallet/getaccount",
            serde_json::json!({
                "address": address,
                "visible": true
            }),
        )
        .await
    }

    pub async fn get_block_number(&self) -> Result<u64> {
        let block = self.get_now_block().await?;

        block["block_header"]["raw_data"]["number"]
            .as_u64()
            .ok_or_else(|| anyhow!("Failed to parse block number"))
    }
}