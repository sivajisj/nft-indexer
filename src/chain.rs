use alloy::primitives::Address;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Filter;
use alloy::sol_types::SolEvent;
use sqlx::PgPool;
use sqlx::types::BigDecimal;
use std::str::FromStr;
use tokio::time::{Duration, sleep};

use crate::db::{EventRecord, insert_transfer_event, promote_confirmed_events};
use crate::events::Transfer;

pub async fn build_provider(rpc_url: &str) -> Result<impl Provider, Box<dyn std::error::Error>> {
    Ok(ProviderBuilder::new().connect_http(rpc_url.parse()?))
}

async fn get_logs_with_retry(
    provider: &impl Provider,
    filter: &Filter,
) -> Result<Vec<alloy::rpc::types::Log>, Box<dyn std::error::Error>> {
    let mut attempts = 0;
    loop {
        match provider.get_logs(filter).await {
            Ok(logs) => return Ok(logs),
            Err(e) => {
                attempts += 1;
                if attempts >= 5 {
                    return Err(Box::new(e));
                }
                let backoff_ms = 500 * attempts;
                eprintln!(
                    "Rate limited or error, retrying in {}ms (attempt {})",
                    backoff_ms, attempts
                );
                sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }
}

pub async fn run_indexer(
    provider: impl Provider,
    pool: PgPool,
    contract_address: Address,
    chain_id: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let starting_block = provider.get_block_number().await?;
    let mut last_scanned_block = starting_block;
    let chunk_size = 10u64;

    println!("Indexer starting, watching from block {}", starting_block);

    loop {
        let block_number = provider.get_block_number().await?;

        while last_scanned_block <= block_number {
            let end_block = std::cmp::min(last_scanned_block + chunk_size - 1, block_number);

            let filter = Filter::new()
                .address(contract_address)
                .event_signature(Transfer::SIGNATURE_HASH)
                .from_block(last_scanned_block)
                .to_block(end_block);

            let logs = get_logs_with_retry(&provider, &filter).await?;

            for log in &logs {
                let decoded: Transfer = log.log_decode()?.inner.data;

                let record = EventRecord {
                    chain_id,
                    contract_address: contract_address.to_string(),
                    token_id: BigDecimal::from_str(&decoded.tokenId.to_string())?,
                    from_address: decoded.from.to_string(),
                    to_address: decoded.to.to_string(),
                    block_number: log.block_number.unwrap() as i64,
                    tx_hash: format!("{:?}", log.transaction_hash.unwrap()),
                    log_index: log.log_index.unwrap() as i32,
                };

                let tx_hash = record.tx_hash.clone();
                let log_index = record.log_index;
                let token_id = decoded.tokenId;

                insert_transfer_event(&pool, record).await?;

                println!(
                    "New event: tokenId={} tx={} log_index={}",
                    token_id, tx_hash, log_index
                );
            }

            last_scanned_block = end_block + 1;
        }

        let confirmation_depth = 12u64;
        let safe_block = block_number.saturating_sub(confirmation_depth);
        let confirmed_count = promote_confirmed_events(&pool, safe_block as i64).await?;

        if confirmed_count > 0 {
            println!(
                "Confirmed {} events (blocks up to {})",
                confirmed_count, safe_block
            );
        }

        sleep(Duration::from_secs(15)).await;
    }
}
