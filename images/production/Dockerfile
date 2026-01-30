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
WORKDIR /app
COPY . .
RUN cargo build --release && \
    cp $(find target/release -maxdepth 1 -type f -executable) /app/app

FROM debian:13.3-slim
COPY --from=build-base /app/app /app/app
COPY --from=cargo-about-base /tmp/licenses.html /app/licenses.html

CMD ["/app/app"]
