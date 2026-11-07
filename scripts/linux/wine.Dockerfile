# The image the #[ignore] Win32 tests that need Wine run in on arm64 Linux:
# Wine 11.0 built from source for aarch64, llvm-mingw for the ARM64 Windows
# programs and Rust's aarch64-pc-windows-gnullvm standard library, Debian's
# x86_64 MinGW GCC for the compile-only checks, and the WebView2 headers.
# Debian's and Ubuntu's arm64 Wine hang while they create a prefix (their
# unwinder stops on an AArch64 pointer-authentication note), so it is built
# here. It starts from the Linux gate image (scripts/linux/Dockerfile):
#
#   docker build -f scripts/linux/Dockerfile -t tachyon-linux-gate:1.97.1 scripts/linux
#   docker build -f scripts/linux/wine.Dockerfile -t tachyon-wine-gate:11.0 scripts/linux
#   docker run --rm --init -v "$PWD":/workspace -w /workspace \
#     -e CARGO_TARGET_DIR=/tmp/ty-target tachyon-wine-gate:11.0 ./scripts/linux/wine-tests.sh
ARG BASE=tachyon-linux-gate:1.97.1
FROM ${BASE}

RUN [ "$(dpkg --print-architecture)" = arm64 ] \
 || { echo "This image is for arm64 Linux; on x86_64 install Debian's wine64 and mingw-w64."; exit 1; }

RUN apt-get update \
 && apt-get install --no-install-recommends --yes \
      bison \
      flex \
      gcc-mingw-w64-x86-64 \
      libfontconfig-dev \
      libfreetype-dev \
      libx11-dev \
      libxcomposite-dev \
      libxcursor-dev \
      libxext-dev \
      libxfixes-dev \
      libxi-dev \
      libxinerama-dev \
      libxrandr-dev \
      libxrender-dev \
      xz-utils \
 && rm -rf /var/lib/apt/lists/*

ARG LLVM_MINGW=20261020
ARG LLVM_MINGW_SHA256=ca16bb0f4f485876496fa46ad8d353c525eba57d0cf9e9e49ec1738fe4810151
RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/llvm-mingw.tar.xz \
      "https://github.com/mstorsjo/llvm-mingw/releases/download/${LLVM_MINGW}/llvm-mingw-${LLVM_MINGW}-ucrt-ubuntu-22.04-aarch64.tar.xz" \
 && echo "${LLVM_MINGW_SHA256}  /tmp/llvm-mingw.tar.xz" | sha256sum -c - \
 && mkdir -p /opt/llvm-mingw \
 && tar -xJf /tmp/llvm-mingw.tar.xz -C /opt/llvm-mingw --strip-components=1 \
 && rm /tmp/llvm-mingw.tar.xz

# Debian's /usr/bin comes first, so x86_64-w64-mingw32-gcc stays GCC and only
# the aarch64 names come from llvm-mingw.
ENV PATH=/usr/local/cargo/bin:/opt/wine/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/opt/llvm-mingw/bin

ARG WINE_FOLDER=11.0
ARG WINE_VERSION=11.0
ARG WINE_SHA256=c07a6857933c1fc60dff5448d79f39c92481c1e9db5aa628db9d0358446e0701
RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/wine.tar.xz \
      "https://dl.winehq.org/wine/source/${WINE_FOLDER}/wine-${WINE_VERSION}.tar.xz" \
 && echo "${WINE_SHA256}  /tmp/wine.tar.xz" | sha256sum -c - \
 && mkdir -p /tmp/wine-source /tmp/wine-build \
 && tar -xJf /tmp/wine.tar.xz -C /tmp/wine-source --strip-components=1 \
 && cd /tmp/wine-build \
 && /tmp/wine-source/configure --prefix=/opt/wine --enable-archs=aarch64 --without-gstreamer \
 && make -j"$(nproc)" \
 && make install \
 && cd / \
 && rm -rf /tmp/wine.tar.xz /tmp/wine-source /tmp/wine-build \
 && wine --version

ARG WEBVIEW2_VERSION=1.0.3485.44
ARG WEBVIEW2_SHA256=bc09150b179246ac90189649b13be8e6b11b3ac200e817e18df106e1f3cf489e
RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/webview2.nupkg \
      "https://www.nuget.org/api/v2/package/Microsoft.Web.WebView2/${WEBVIEW2_VERSION}" \
 && echo "${WEBVIEW2_SHA256}  /tmp/webview2.nupkg" | sha256sum -c - \
 && mkdir -p /opt/webview2 \
 && unzip -q /tmp/webview2.nupkg -d /opt/webview2 \
 && rm /tmp/webview2.nupkg
ENV TAC_WEBVIEW2_SDK=/opt/webview2

RUN rustup target add aarch64-pc-windows-gnullvm x86_64-pc-windows-gnu
