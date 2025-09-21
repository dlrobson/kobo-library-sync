FROM rust:1.90.0-trixie

ARG USER=user
ENV USER=${USER}

# Setup the user
RUN apt-get update && \
    apt-get install -y git sudo && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -m ${USER} && \
    echo "${USER} ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers.d/nopasswd;

USER ${USER}
