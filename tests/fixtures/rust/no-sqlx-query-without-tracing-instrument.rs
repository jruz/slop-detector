async fn missing_instrument(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

async fn missing_local_pool_instrument() -> Result<(), sqlx::Error> {
    let pool = get_pool().await?;
    sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    Ok(())
}
