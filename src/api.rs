use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use sqlx::PgPool;

use crate::db::get_owned_tokens;
use crate::errors::AppError;

#[derive(Serialize)]
pub struct OwnedTokenResponse {
    pub token_id: String,
}

pub async fn owned_tokens(
    State(pool): State<PgPool>,
    Path(address): Path<String>,
) -> Result<Json<Vec<OwnedTokenResponse>>, AppError> {
    let tokens = get_owned_tokens(&pool, &address)
        .await
        .map_err(AppError::Database)?;

    let response = tokens
        .into_iter()
        .map(|t| OwnedTokenResponse {
            token_id: t.token_id.to_string(),
        })
        .collect();

    Ok(Json(response))
}
