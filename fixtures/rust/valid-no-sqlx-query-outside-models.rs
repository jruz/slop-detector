async fn load(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let _migrator = sqlx::migrate!("./migrations");
    let _connection = sqlx::PgPool::connect("postgres://example").await?;
    pool.acquire().await?;
    Ok(())
}
