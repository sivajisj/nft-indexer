use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::types::BigDecimal;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}

pub struct EventRecord {
    pub chain_id: i64,
    pub contract_address: String,
    pub token_id: BigDecimal,
    pub from_address: String,
    pub to_address: String,
    pub block_number: i64,
    pub tx_hash: String,
    pub log_index: i32,
}

pub struct OwnedToken {
    pub token_id: BigDecimal,
}
pub async fn insert_transfer_event(pool: &PgPool, record: EventRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO events (chain_id, contract_address, event_type, token_id, from_address, to_address, block_number, tx_hash, log_index, confirmed)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (tx_hash, log_index) DO NOTHING"
    )
    .bind(record.chain_id)
    .bind(record.contract_address)
    .bind("Transfer")
    .bind(record.token_id)
    .bind(record.from_address)
    .bind(record.to_address)
    .bind(record.block_number)
    .bind(record.tx_hash)
    .bind(record.log_index)
    .bind(false)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn promote_confirmed_events(pool: &PgPool, safe_block: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE events SET confirmed = true WHERE confirmed = false AND block_number <= $1",
    )
    .bind(safe_block)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn get_owned_tokens(
    pool: &PgPool,
    owner_address: &str,
) -> Result<Vec<OwnedToken>, sqlx::Error> {
    let rows = sqlx::query_as!(
        OwnedToken,
        r#"
        SELECT DISTINCT ON (token_id)
            token_id as "token_id!"
        FROM events
        WHERE event_type = 'Transfer' AND confirmed = true AND LOWER(to_address) = $1
        ORDER BY token_id, block_number DESC, log_index DESC
        "#,
        owner_address.to_lowercase()
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
