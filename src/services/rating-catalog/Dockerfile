# syntax=docker/dockerfile:1.7

FROM rust:1.94-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml build.rs ./
COPY proto/ proto/

RUN mkdir src && printf 'fn main() {}\n' > src/main.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release

RUN rm -rf src

COPY src/ src/

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    touch src/main.rs && \
    cargo build --release && \
    cp /app/target/release/rating-catalog /tmp/rating-catalog

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false appuser

COPY --from=builder /tmp/rating-catalog /usr/local/bin/rating-catalog

RUN chown appuser:appuser /usr/local/bin/rating-catalog

USER appuser

EXPOSE 50053

CMD ["rating-catalog"]
