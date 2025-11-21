FROM rust:1.91.1-trixie

ARG USER=user
ENV USER=${USER}

# Setup the user
RUN apt-get update && \
    apt-get install -y git sudo && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -m ${USER} && \
    echo "${USER} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/nopasswd;

# Install nightly toolchain for formatting and coverage
RUN rustup toolchain install nightly && \
    rustup component add rustfmt --toolchain nightly

# Install cargo-binstall for faster tool installation
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# Install cargo tools for CI
RUN cargo binstall --no-confirm cargo-llvm-cov@0.6.15

USER ${USER}
