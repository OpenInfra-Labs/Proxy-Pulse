FROM rust:1.86-slim AS builder

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y pkg-config && rm -rf /var/lib/apt/lists/*
RUN cargo build --release --bin proxy-pulse

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/proxy-pulse .
COPY config.example.yaml .

EXPOSE 8080

CMD ["./proxy-pulse"]
