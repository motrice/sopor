# syntax=docker/dockerfile:1.7
FROM rust:1.95-bookworm AS builder
WORKDIR /app

# Cache deps first.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main(){}" > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/deps/sopor*

COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
RUN useradd -r -u 10001 -m -d /home/sopor sopor
COPY --from=builder /app/target/release/sopor /usr/local/bin/sopor
USER sopor
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/sopor"]
