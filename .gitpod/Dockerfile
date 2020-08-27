FROM gitpod/workspace-full:latest

USER root
RUN apt-get install -yq \
        musl \
        musl-dev \
        musl-tools \
        libssl-dev \
    && apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/*

USER gitpod
RUN bash -lc "rustup target add x86_64-unknown-linux-musl"

