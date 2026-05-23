# syntax=docker/dockerfile:1.7

FROM rust:1.94-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./

COPY src/api-gateway/Cargo.toml src/api-gateway/Cargo.toml
COPY src/services/author-analytics-service/Cargo.toml src/services/author-analytics-service/Cargo.toml
COPY src/services/author-catalog/Cargo.toml src/services/author-catalog/Cargo.toml
COPY src/services/book-recommendation/Cargo.toml src/services/book-recommendation/Cargo.toml
COPY src/services/book-search/Cargo.toml src/services/book-search/Cargo.toml
COPY src/services/rating-catalog/Cargo.toml src/services/rating-catalog/Cargo.toml

COPY src/services/book-search/build.rs src/services/book-search/build.rs
COPY src/services/book-search/proto/ src/services/book-search/proto/
COPY src/services/author-catalog/proto/ src/services/author-catalog/proto/

RUN mkdir -p \
        src/api-gateway/src \
        src/services/author-analytics-service/src \
        src/services/author-catalog/src \
        src/services/book-recommendation/src \
        src/services/book-search/src \
        src/services/rating-catalog/src \
    && printf 'fn main() {}\n' > src/api-gateway/src/main.rs \
    && printf 'fn main() {}\n' > src/services/author-analytics-service/src/main.rs \
    && printf 'fn main() {}\n' > src/services/author-catalog/src/main.rs \
    && printf 'fn main() {}\n' > src/services/book-recommendation/src/main.rs \
    && printf 'fn main() {}\n' > src/services/book-search/src/main.rs \
    && printf 'fn main() {}\n' > src/services/rating-catalog/src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release -p book-search
RUN rm -rf src/services/book-search/src

COPY src/services/book-search/src/ src/services/book-search/src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    touch src/services/book-search/src/main.rs && cargo build --release -p book-search

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

EXPOSE 50057

CMD ["book-search"]
