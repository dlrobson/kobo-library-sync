FROM rust:1.84.0-slim-bookworm

ARG USER=user

# Setup the user
RUN apt-get update && \
    apt-get install -y git sudo && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -m ${USER} && \
    echo "${USER} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/nopasswd;

USER ${USER}
