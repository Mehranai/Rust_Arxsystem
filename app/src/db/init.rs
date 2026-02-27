use clickhouse::Client;

pub async fn run_sql(
    client: &Client,
    sql: &str,
) -> anyhow::Result<()> {
    
    for stmt in sql.split(';') {
        let stmt = stmt.trim();
        if !stmt.is_empty() {
            // println!("Inja reside");
            client.query(stmt).execute().await?;
        }
    }
    Ok(())
}