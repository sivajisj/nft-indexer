use crate::errors::AppError;
use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use siwe::Message;
use sqlx::PgPool;
use std::str::FromStr;

const NONCE_TTL_MINUTES: i64 = 10;

#[derive(Deserialize)]
pub struct NonceRequest {
    pub wallet_address: String,
}

#[derive(Serialize)]
pub struct NonceResponse {
    pub nonce: String,
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub message: String,
    pub signature: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub wallet_address: String,
}
pub async fn get_nonce(
    State(pool): State<PgPool>,
    Json(payload): Json<NonceRequest>,
) -> Result<Json<NonceResponse>, AppError> {
    let wallet_address = payload.wallet_address.to_lowercase(); // normalize, addresses are case-insensitive

    let nonce: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32) // wider entropy than before, production strength
        .map(char::from)
        .collect();

    sqlx::query(
        "INSERT INTO users (wallet_address, nonce, nonce_issued_at) VALUES ($1, $2, NOW())
         ON CONFLICT (wallet_address) DO UPDATE SET nonce = $2, nonce_issued_at = NOW()",
    )
    .bind(&wallet_address)
    .bind(&nonce)
    .execute(&pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(NonceResponse { nonce }))
}

pub async fn verify_signature(
    State(pool): State<PgPool>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    let message = Message::from_str(&payload.message)
        .map_err(|e| AppError::BadRequest(format!("Malformed SIWE message: {}", e)))?;

    let signature_bytes = hex::decode(payload.signature.trim_start_matches("0x"))
        .map_err(|_| AppError::BadRequest("Invalid signature encoding".to_string()))?;

    message
        .verify(&signature_bytes, &Default::default())
        .await
        .map_err(|_| AppError::InvalidSignature)?;

    let wallet_address = format!("0x{}", hex::encode(message.address)).to_lowercase();
    let (stored_nonce, nonce_issued_at): (String, Option<DateTime<Utc>>) =
        sqlx::query_as("SELECT nonce, nonce_issued_at FROM users WHERE wallet_address = $1")
            .bind(&wallet_address)
            .fetch_optional(&pool)
            .await
            .map_err(AppError::Database)?
            .ok_or(AppError::UserNotFound)?;

    if stored_nonce != message.nonce {
        return Err(AppError::NonceMismatch);
    }

    let issued_at = nonce_issued_at.ok_or(AppError::NonceExpired)?;
    if Utc::now() - issued_at > chrono::Duration::minutes(NONCE_TTL_MINUTES) {
        return Err(AppError::NonceExpired);
    }

    // Invalidate the nonce immediately after successful use, single-use enforcement
    let fresh_nonce: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    sqlx::query(
        "UPDATE users SET nonce = $1, nonce_issued_at = NULL, last_login_at = NOW() WHERE wallet_address = $2"
    )
    .bind(&fresh_nonce)
    .bind(&wallet_address)
    .execute(&pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(VerifyResponse {
        success: true,
        wallet_address,
    }))
}
