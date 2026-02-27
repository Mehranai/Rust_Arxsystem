use anyhow::{Result, bail};
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub clickhouse_url: String,
    pub clickhouse_user: String,
    pub clickhouse_password: String,

    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,

    pub max_trace_depth: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();


        let cfg = Self {
            clickhouse_url: get_env("CLICKHOUSE_URL")?,
            clickhouse_user: get_env("CLICKHOUSE_USER")?,
            clickhouse_password: get_env("CLICKHOUSE_PASSWORD")?,

            neo4j_uri: get_env("NEO4J_URI")?,
            neo4j_user: get_env("NEO4J_USER")?,
            neo4j_password: get_env("NEO4J_PASSWORD")?,

            max_trace_depth: get_env("MAX_TRACE_DEPTH")?
                .parse::<usize>()
                .unwrap_or(3),
        };

        log::info!("CLICKHOUSE_URL={}", clickhouse_url);
        log::info!("NEO4J_URI={}", neo4j_uri);
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if self.max_trace_depth == 0 || self.max_trace_depth > 10 {
            bail!("MAX_TRACE_DEPTH must be between 1 and 10");
        }
        Ok(())
    }
}

fn get_env(key: &str) -> Result<String> {
    env::var(key)
        .map_err(|_| anyhow::anyhow!("Missing env var: {}", key))
}
