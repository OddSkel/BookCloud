FROM rust:1.94-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

<<<<<<< Updated upstream
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -ms /bin/bash appuser

COPY --from=builder /app/target/release/book-catalog /usr/local/bin/book-catalog

RUN chown appuser:appuser /usr/local/bin/book-catalog

USER appuser
=======
RUN sed -i 's/\r$//' generate_proto.sh \
 && chmod +x generate_proto.sh \
 && PROTO_ROOT=/app /bin/sh generate_proto.sh
>>>>>>> Stashed changes

EXPOSE 50051

CMD ["book-catalog"]
