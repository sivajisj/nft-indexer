# Stage 1: Build
FROM rust:1.97-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the ENTIRE project at once, no dummy-binary trick, simpler and correct
COPY . .

# Use the committed .sqlx query cache instead of a live DB connection at build time
ENV SQLX_OFFLINE=true

RUN cargo build --release

# Verify the real binary actually exists and check its size, fails the build loudly if wrong
RUN ls -la /app/target/release/nft-indexer && \
    test $(stat -c%s /app/target/release/nft-indexer) -gt 1000000 || \
    (echo "ERROR: binary suspiciously small, likely build issue" && exit 1)

# Stage 2: Runtime
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/nft-indexer /app/nft-indexer
COPY migrations ./migrations

EXPOSE 4000

CMD ["./nft-indexer"]