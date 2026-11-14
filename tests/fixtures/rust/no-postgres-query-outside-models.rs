use deadpool_postgres::Pool;

async fn export(pool: &Pool) -> Result<(), tokio_postgres::Error> {
    let client = pool.get().await.unwrap();
    let _rows = client.simple_query("SELECT 1").await?;
    let _row = client.query_one("SELECT 1", &[]).await?;
    let _rows = client.query("SELECT 1", &[]).await?;
    let _row = client.query_opt("SELECT 1", &[]).await?;
    let _rows = client
        .query_raw(
            "SELECT 1",
            &[] as &[&(dyn tokio_postgres::types::ToSql + Sync)],
        )
        .await?;
    Ok(())
}
