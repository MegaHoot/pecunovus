FROM rust:1.81-slim as builder

WORKDIR /usr/src/pecunovus

# Install build deps
RUN apt-get update && apt-get install -y pkg-config libssl-dev clang cmake

# Copy source
COPY . .

# Build release binary
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/pecunovus/target/release/pecunovus /usr/local/bin/pecunovus

EXPOSE 7000 8080

ENTRYPOINT ["pecunovus"]
CMD ["run", "--bind", "0.0.0.0:7000", "--rpc", "0.0.0.0:8080"]
