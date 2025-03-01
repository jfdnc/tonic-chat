# Build stage
FROM rust:1.74 as builder
WORKDIR /usr/src/tonic-chat
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/tonic-chat/target/release/tonic-chat /usr/local/bin/
EXPOSE 50051
CMD ["tonic-chat"]
