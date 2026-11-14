async fn load(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    let _row = sqlx::query_as::<_, (i64,)>("SELECT 1")
        .fetch_one(pool)
        .await?;
    let _count = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(())
}
