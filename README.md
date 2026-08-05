# NFT Minting Platform, Indexer

An async Rust backend that watches a live Ethereum contract, indexes its events into Postgres with reorg-safe confirmation depth, handles wallet-based authentication, and serves it all over a REST API. Dockerized, with CI running fmt, clippy, and tests on every push.

Part of a larger system, see [`nft-infra`](https://github.com/sivajisj/nft-infra) for the full architecture.

[![CI](https://github.com/sivajisj/nft-indexer/actions/workflows/ci.yml/badge.svg)](https://github.com/sivajisj/nft-indexer/actions)

---

## What it actually does

1. Connects to Sepolia via RPC, chunked scanning to stay under free-tier rate limits, with retry and exponential backoff on transient failures
2. Decodes `Transfer` events from the deployed [NFT contract](https://sepolia.etherscan.io/address/0x1d24fe1860f4e670afd65c1b93118a4b4f5c0f54) using `alloy`'s typed event decoding
3. Writes them into Postgres with a `UNIQUE(tx_hash, log_index)` constraint, so re-scanning the same range on restart can never create a duplicate
4. Marks events `confirmed` only once they clear a 12-block depth, protecting against chain reorgs rather than trusting the first RPC response
5. Runs an Axum API server *concurrently* with the indexing loop, on the same Tokio runtime, via `tokio::spawn`
6. Implements Sign-In With Ethereum (SIWE): nonce issuance, signature verification, single-use nonce invalidation, 10-minute expiry

## API endpoints

| Method | Path | What it does |
|---|---|---|
| `POST` | `/auth/nonce` | Issues a fresh, single-use nonce for a wallet address |
| `POST` | `/auth/verify` | Verifies a signed SIWE message against the issued nonce |
| `GET` | `/tokens/owned/{address}` | Returns the tokens currently owned by a wallet, derived from confirmed transfer history |

### Example

```bash
curl -X POST http://localhost:4000/auth/nonce \
  -H "Content-Type: application/json" \
  -d '{"wallet_address": "0x439Bc13b99428A538e48bFCD496486394c10C405"}'
# → {"nonce":"Fjax3tAOJXrmMAvU"}

curl http://localhost:4000/tokens/owned/0x439Bc13b99428A538e48bFCD496486394c10C405
# → [{"token_id":"0"},{"token_id":"1"},{"token_id":"2"}]
```

## Running it

### Option 1: Docker (recommended, matches production)

```bash
git clone https://github.com/sivajisj/nft-indexer.git
cd nft-indexer
cp .env.example .env   # fill in RPC_URL
docker compose up --build
```

Or via the top-level [`nft-infra`](https://github.com/sivajisj/nft-infra) compose file, which wires this up alongside Postgres with a proper healthcheck gate.

### Option 2: Local

Requires Postgres running locally and `sqlx-cli` installed.

```bash
cargo install sqlx-cli --no-default-features --features postgres
sqlx migrate run
cargo run
```

### Environment variables

```
DATABASE_URL=postgres://indexer_user:devpassword@localhost:5432/nft_indexer
RPC_URL=https://eth-sepolia.g.alchemy.com/v2/your-alchemy-key
```

## Architecture decisions

**Why Rust, not Node.js or Go.** `sqlx`'s compile-time-verified queries catch a mistyped column or wrong type at build time, not as a silent bad response at 3am. Tokio's async model means the indexing loop and the API server run concurrently on the same thread pool without either blocking the other, no separate process, no message queue between them, just `tokio::spawn`.

**Why chunked scanning with retry/backoff, not one big query.** Free-tier RPC providers cap `eth_getLogs` to small block ranges (10 blocks on Alchemy's free tier). Real production indexers hit this constraint too. The fix: walk the range in small windows, back off with increasing delay on rate-limit errors, and fail fast (not retry) on genuinely permanent errors like a malformed request, retrying those is pure waste.

**Why 12-block confirmation depth before trusting an event.** A block that's 1-2 deep can still be reorged out by the chain. Waiting 12 blocks (~24 seconds on a fast testnet) makes that vanishingly unlikely while keeping the wait tolerable. It's a UX/risk tradeoff, not a protocol constant, a high-value DeFi protocol would reasonably use 64-128+.

**Why `UNIQUE(tx_hash, log_index)`, not just `tx_hash`.** A single transaction can emit multiple events (a batch mint of 3 tokens emits 3 separate `Transfer` events, same `tx_hash`, different `log_index`). Deduplicating on `tx_hash` alone would silently drop real events.

**Why SIWE, not a traditional password system.** The wallet already proves identity cryptographically, signing a challenge nonce with a private key is a stronger, simpler proof than anything a password adds. Building a second identity system on top would just be more surface area to secure for no real benefit.

**A real debugging story worth knowing about.** The Docker build silently shipped an empty 437KB placeholder binary for several rebuild attempts, because a `cargo build` failure (missing `sqlx` offline query cache, then a glibc mismatch between build and runtime stages) meant Docker Compose kept quietly reusing the last *successful* image rather than surfacing the failure. The fix was `cargo sqlx prepare` (committing `.sqlx/` as an offline query cache) and matching the runtime base image's glibc to the builder's. The real lesson: a suspiciously small or stale-behaving image usually means "the build failed silently," not "debug the runtime."

## CI

Three jobs run on every push: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (warnings treated as errors), and `cargo build && cargo test`. Dependencies are cached between runs. `SQLX_OFFLINE=true` lets the build verify queries against the committed `.sqlx/` cache instead of needing a live database on the CI runner.

## Project structure

```
src/
  main.rs      → thin entry point, wires everything together
  config.rs    → environment loading
  db.rs        → connection pool, queries, no blockchain knowledge at all
  chain.rs     → RPC provider, event watching, retry logic, confirmation promotion
  events.rs    → Transfer event definition (alloy sol! macro)
  auth.rs      → SIWE nonce issuance and verification
  api.rs       → owned-tokens endpoint
  error.rs     → structured AppError → HTTP response mapping
migrations/    → versioned SQL schema
.sqlx/         → offline query cache (required for CI/Docker builds)
```

## About

Part of a full-stack portfolio project by [Sivaji Gadidala](https://sivajibuilds.netlify.app). See [`nft-infra`](https://github.com/sivajisj/nft-infra) for the complete architecture.

[Email](mailto:sivajigsivajig703@gmail.com) · [LinkedIn](https://linkedin.com/in/sivaji-gadidala-b712ba221) · [Portfolio](https://sivajibuilds.netlify.app)
