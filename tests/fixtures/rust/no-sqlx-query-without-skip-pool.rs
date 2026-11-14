use tracing::instrument;

#[tracing::instrument]
async fn missing_skip(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

#[instrument(fields(user_id))]
async fn missing_skip_with_import(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(())
}

async fn missing_instrument(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_as::<_, (i64,)>("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}
