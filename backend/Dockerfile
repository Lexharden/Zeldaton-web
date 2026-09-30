# syntax=docker/dockerfile:1
FROM rust:1.96-slim AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend
RUN cargo build --release -p zeldathon-server --bin zeldathon-server

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 zeldathon && mkdir /data && chown zeldathon /data
COPY --from=build /src/target/release/zeldathon-server /usr/local/bin/zeldathon-server
USER zeldathon
ENV BIND=0.0.0.0:8080 DATABASE_URL=sqlite:///data/zeldathon.db?mode=rwc
VOLUME /data
EXPOSE 8080
CMD ["zeldathon-server"]
