FROM rust:1.91.1-slim-trixie

ARG USER=user
ENV USER=${USER}

RUN apt-get update && \
    apt-get install -y \
    # CI dependencies
    curl git \
    # Devcontainer packages
    sudo locales && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
    
RUN echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen && locale-gen && \
    useradd --create-home --shell /bin/bash ${USER} && \
    echo "${USER} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/nopasswd

# Install components first to only download these for the default toolchain
RUN rustup component add clippy rust-analyzer rustfmt && \
    rustup toolchain install nightly --profile minimal --component rustfmt,llvm-tools-preview

USER ${USER}

COPY binstall-versions.txt /tmp/binstall-versions.txt
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/v1.10.20/install-from-binstall-release.sh | bash && \
    cargo binstall --no-confirm $(cat /tmp/binstall-versions.txt)
