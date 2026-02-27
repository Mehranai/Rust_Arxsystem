use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

#[derive(Clone)]
pub struct TronClient {
    pub client: Client,
    pub base_url: String,
    pub api_key: Option<String>,
}

impl TronClient {
    pub fn new(base_url: &str, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            api_key,
        }
    }

    async fn post(&self, endpoint: &str, body: Value) -> Result<Value> {
        let url = format!("{}/{}", self.base_url, endpoint);

        let mut req = self.client.post(url).json(&body);

        if let Some(key) = &self.api_key {
            req = req.header("TRON-PRO-API-KEY", key);
        }

        let resp = req.send().await?;
        // changed for test
        //Ok(resp.json().await?)
        
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            println!("HTTP ERROR {} : {}", status, text);
            return Ok(serde_json::json!({}));
        }

        let parsed: Value = serde_json::from_str(&text)
            .unwrap_or_else(|_| {
                println!("INVALID JSON: {}", text);
                serde_json::json!({})
            });

        Ok(parsed)
    }

    pub async fn get_block(&self, number: u64) -> Result<Value> {
        self.post("wallet/getblockbynum", serde_json::json!({ "num": number })).await
    }

    pub async fn get_now_block(&self) -> Result<Value> {
        self.post("wallet/getnowblock", serde_json::json!({})).await
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
            serde_json::json!({ "address": address, "visible": true }),
        )
        .await
    }

    // this one is new
    pub async fn get_block_number(&self) -> Result<u64> {
        let block = self.get_now_block().await?;

        Ok(
            block["block_header"]["raw_data"]["number"]
                .as_u64()
                .unwrap_or(0)
        )
    }
}

