FROM rust:1.94-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace Cargo.toml
COPY Cargo.toml .

# Copy all workspace members
COPY src/services/book-search/ src/services/book-search/
COPY src/services/author-catalog/ src/services/author-catalog/
COPY src/services/rating-catalog/ src/services/rating-catalog/
COPY src/services/book-recommendation/ src/services/book-recommendation/

# Build from within book-search
WORKDIR /app/src/services/book-search
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash appuser

# Binary is now under the workspace target
COPY --from=builder /app/target/release/book-search /usr/local/bin/book-search

RUN chown appuser:appuser /usr/local/bin/book-search

USER appuser

EXPOSE 50057

CMD ["book-search"]