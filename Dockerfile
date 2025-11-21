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
RUN rustup toolchain install nightly-2025-09-01 && \
    rustup component add rustfmt --toolchain nightly-2025-09-01

# Install cargo tools for CI
RUN cargo install cargo-llvm-cov

USER ${USER}
