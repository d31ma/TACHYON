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
      unzip \
      x11-utils \
      xauth \
      xdotool \
      xfwm4 \
      xvfb \
 && rm -rf /var/lib/apt/lists/*

ARG RUST_TOOLCHAIN=1.97.1
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
 && rustup target add wasm32-unknown-unknown \
 && rustc --version

ARG BUN_VERSION=1.4.3
RUN case "$(dpkg --print-architecture)" in arm64) bun_arch=aarch64 ;; *) bun_arch=x64 ;; esac \
 && curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/bun.zip \
      "https://github.com/oven-sh/bun/releases/download/bun-v${BUN_VERSION}/bun-linux-${bun_arch}.zip" \
 && unzip -q /tmp/bun.zip -d /tmp \
 && install -m 0755 "/tmp/bun-linux-${bun_arch}/bun" /usr/local/bin/bun \
 && rm -rf /tmp/bun.zip "/tmp/bun-linux-${bun_arch}" \
 && bun --version

WORKDIR /workspace
