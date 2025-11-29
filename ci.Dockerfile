FROM rust:1.91.1-trixie

ARG USER=user
ENV USER=${USER}

RUN apt-get update && \
    apt-get install -y \
    # CI dependencies
    git \
    # Devcontainer packages
    sudo locales && \
    rm -rf /var/lib/apt/lists/* 
    
RUN useradd --create-home --shell /bin/bash ${USER} && \
    echo "${USER} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/nopasswd

RUN echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen && locale-gen

RUN rustup toolchain install nightly && \
    rustup component add rustfmt --toolchain nightly

USER ${USER}

RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/v1.10.20/install-from-binstall-release.sh | bash && \
    cargo binstall --no-confirm \
    cargo-llvm-cov@0.6.15 \
    cargo-deny@0.18.6 \
    just@1.43.1
