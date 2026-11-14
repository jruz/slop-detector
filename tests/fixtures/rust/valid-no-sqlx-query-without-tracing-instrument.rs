#[tracing::instrument]
async fn traced_pool(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

#[tracing::instrument]
async fn traced_second_pool(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_as::<_, (i64,)>("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}

#[tracing::instrument]
async fn traced_local_pool() -> Result<(), sqlx::Error> {
    let pool = get_pool().await?;
    sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await?;
    Ok(())
}
