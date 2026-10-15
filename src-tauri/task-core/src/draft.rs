use crate::*;
pub async fn save(repo: &Repository, key: &str, json: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO ui_drafts VALUES(?,?) ON CONFLICT(key) DO UPDATE SET json=excluded.json",
    )
    .bind(key)
    .bind(json)
    .execute(&repo.pool)
    .await?;
    Ok(())
}
pub async fn load(repo: &Repository, key: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT json FROM ui_drafts WHERE key=?")
        .bind(key)
        .fetch_optional(&repo.pool)
        .await?)
}
