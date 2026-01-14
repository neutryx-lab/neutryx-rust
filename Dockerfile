# --- Stage 1: Build ---
FROM rust:1.83-slim-bookworm as builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release -p demo_gui --features web --bin demo-web

# --- Stage 2: Runtime ---
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/demo-web ./demo-web

COPY --from=builder /app/demo/gui/static ./static

ENV FB_PORT=8080
ENV FB_HOST=0.0.0.0
ENV RUST_LOG=info

RUN useradd -m appuser
USER appuser

CMD ["./demo-web"]
