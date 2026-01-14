# --- Stage 1: Build ---
FROM rust:1.84-slim-bookworm as builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release -p frictional_bank --bin frictional-bank

# --- Stage 2: Runtime ---
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/frictional-bank ./frictional-bank

# Create data directory for demo config
RUN mkdir -p demo/data/output

ENV RUST_LOG=info

RUN useradd -m appuser
USER appuser

# Cloud Run sets PORT env var
CMD ["./frictional-bank"]
