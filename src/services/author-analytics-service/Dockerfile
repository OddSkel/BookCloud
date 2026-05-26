# syntax=docker/dockerfile:1.7

FROM rust:1.94-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock build.rs ./
COPY proto/ proto/

RUN mkdir src && printf 'fn main() {}\n' > src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release

RUN rm -rf src

COPY src/ src/

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    touch src/main.rs && \
    cargo build --release && \
    cp /app/target/release/author-analytics-service /tmp/author-analytics-service

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false appuser

COPY --from=builder /tmp/author-analytics-service /usr/local/bin/author-analytics-service

RUN chown appuser:appuser /usr/local/bin/author-analytics-service

USER appuser

EXPOSE 50056
EXPOSE 8086

CMD ["author-analytics-service"]