use clickhouse::Client;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub clickhouse: Arc<Client>,
}

impl AppState {
    pub fn new(client: Client) -> Self { Self { clickhouse: Arc::new(client) } }
}
