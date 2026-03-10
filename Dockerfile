FROM ubuntu:24.04

ARG TARGETARCH

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY build/${TARGETARCH}/proxy-pulse /app/proxy-pulse
RUN chmod +x /app/proxy-pulse

EXPOSE 8080

ENV HOST=0.0.0.0
ENV PORT=8080

ENTRYPOINT ["/app/proxy-pulse"]
