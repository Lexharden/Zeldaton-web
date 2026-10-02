# syntax=docker/dockerfile:1
# Build context is the repository root (Cargo workspace): docker build -f backend/Dockerfile .
FROM rust:1.96-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p zeldathon-server --bin zeldathon-server \
    && cp target/release/zeldathon-server /zeldathon-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 zeldathon \
    && mkdir -p /data/uploads && chown -R zeldathon /data
COPY --from=build /zeldathon-server /usr/local/bin/zeldathon-server
USER zeldathon
ENV BIND=0.0.0.0:8080 \
    DATABASE_URL=sqlite:///data/zeldathon.db?mode=rwc \
    UPLOADS_DIR=/data/uploads \
    RUST_LOG=info
VOLUME /data
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=15s --retries=3 \
    CMD bash -c 'exec 3<>/dev/tcp/127.0.0.1/8080 && printf "GET /health HTTP/1.0\r\n\r\n" >&3 && grep -q ok <&3' || exit 1
CMD ["zeldathon-server"]
