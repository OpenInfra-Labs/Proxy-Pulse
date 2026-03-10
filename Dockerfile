FROM debian:bookworm-slim

ARG TARGETARCH

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY bin/proxy-pulse-${TARGETARCH} ./proxy-pulse
COPY config.example.yaml .
RUN chmod +x ./proxy-pulse

EXPOSE 8080

CMD ["./proxy-pulse"]
