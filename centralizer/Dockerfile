# syntax=docker/dockerfile:1

FROM rust:1.80 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/lyricframe-centralizer /app/lyricframe-centralizer
COPY --from=builder /app/templates /app/templates

ENV PORT=3000
EXPOSE 3000

CMD ["./lyricframe-centralizer"]
