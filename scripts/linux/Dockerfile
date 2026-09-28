FROM debian:trixie-slim

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update \
 && apt-get install --no-install-recommends --yes \
      at-spi2-core \
      build-essential \
      ca-certificates \
      curl \
      dbus-x11 \
      gir1.2-atspi-2.0 \
      libgtk-4-dev \
      libjson-glib-dev \
      libssl-dev \
      libwebkitgtk-6.0-dev \
      nodejs \
      pkg-config \
      python3-gi \
      xauth \
      xvfb \
 && rm -rf /var/lib/apt/lists/*

ARG RUST_TOOLCHAIN=1.97.1
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
 && rustc --version

WORKDIR /workspace
