FROM rust:1.93.0-slim-trixie AS cargo-about-base

RUN apt-get update && \
    apt-get install -y \
    # Binstall dependencies
    curl jq && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
    
COPY . .
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/v1.17.4/install-from-binstall-release.sh | bash && \
    cargo binstall --no-confirm $(jq -r '.[] | "\(.crate)@\(.version)"' images/production/binstall-versions.json) && \
    cargo about generate --config images/production/about.toml --output-file /tmp/licenses.html images/production/about.hbs
    
FROM rust:1.93.0-slim-trixie AS build-base

ARG BINARY_NAME
WORKDIR /app
COPY . .

RUN cargo build --release --bin ${BINARY_NAME}

FROM debian:13.2-slim

ARG BINARY_NAME
COPY --from=build-base /app/target/release/${BINARY_NAME} /usr/local/bin/app
COPY --from=cargo-about-base /tmp/licenses.html /app/licenses.html

CMD ["app"]
