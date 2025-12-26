FROM rust:1.92.0-slim AS base

ARG BINARY_NAME
WORKDIR /app
COPY . .
RUN cargo build --release --bin ${BINARY_NAME}

FROM debian:13.2-slim

ARG BINARY_NAME
COPY --from=base /app/target/release/${BINARY_NAME} /usr/local/bin/app

CMD ["app"]
