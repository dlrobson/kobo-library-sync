FROM rust:1.93.0-slim-trixie

ARG USER=user
ENV USER=${USER}

RUN apt-get update && \
    apt-get install -y \
    # CI dependencies
    curl git jq \
    # Development packages
    locales procps sudo && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Install Docker
RUN install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc \
    && chmod a+r /etc/apt/keyrings/docker.asc \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian \
    $(. /etc/os-release && echo "$VERSION_CODENAME") stable" > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends docker-ce-cli docker-buildx-plugin \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

RUN echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen && locale-gen && \
    useradd --create-home --shell /bin/bash ${USER} && \
    echo "${USER} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/nopasswd

# Install components first to only download these for the default toolchain
RUN rustup component add clippy rust-analyzer rustfmt && \
    rustup toolchain install nightly --profile minimal --component rustfmt,llvm-tools-preview

USER ${USER}

COPY --chown=${USER}:${USER} binstall-versions.json /tmp/binstall-versions.json
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/v1.17.4/install-from-binstall-release.sh | bash && \
    cargo binstall --no-confirm $(jq -r '.[] | "\(.crate)@\(.version)"' /tmp/binstall-versions.json) && \
    rm /tmp/binstall-versions.json
