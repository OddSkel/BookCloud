FROM rust:1.94-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY src/services/book-search/ .
COPY src/services/author-catalog/proto/ ../author-catalog/proto/

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash appuser

COPY --from=builder /app/target/release/book-search /usr/local/bin/book-search

RUN chown appuser:appuser /usr/local/bin/book-search

USER appuser

EXPOSE 50054

CMD ["book-search"]
