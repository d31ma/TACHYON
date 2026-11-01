# The Linux gate image: GTK 4 and WebKitGTK 6 from Debian trixie, Rust, Bun and Node.js.
#
# Two stages. `gate` has what the native, application-state and icon gates need (the CI linux-native job builds
# only it, with `--target gate`). The final stage adds what the other Linux gates need:
#   - the desktop pieces of compositor-test.sh (headless mutter), desktop-links-test.sh (xdg-utils and the
#     OpenURI portal), desktop-services-test.sh (GeoClue, dunst, and systemd's busctl, which dunstctl calls) and
#     deb-install-test.sh (desktop-file-utils),
#     installed in `gate` already, because they are small;
#   - the toolchains of page-services-native-test.mjs, whose Tally page has a WebAssembly service in Go, C#, F#,
#     Swift, Kotlin, Java, Dart, Ruby, Lua, C++ and Python. They are the versions and checksums of rust-ci.yml's
#     browser-languages job and scripts/ci/install-toolchains.mjs: Go 1.27, .NET 10.0.302 with wasm-tools,
#     Swift 6.3.3 with its Swift SDK for WebAssembly, Kotlin 2.1.10 with kotlin-stdlib-wasm-js, JDK 21 with
#     TeaVM 0.15.0, Dart 3.13.1, Emscripten 6.0.10 (runtimes/ruby-web/sources.txt) for C++, Lua and the Tac Ruby
#     runtime, Ruby for that runtime's build, and the Pyodide 314.0.7 core files in Tachyon's cache.
# Release extras build the final stage (`docker build -f scripts/linux/Dockerfile -t tachyon-linux-gate:1.97.1
# scripts/linux`), and wine.Dockerfile starts from it.
FROM debian:trixie-slim AS gate

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
      desktop-file-utils \
      dunst \
      geoclue-2-demo \
      geoclue-2.0 \
      gir1.2-atspi-2.0 \
      libgtk-4-dev \
      libjson-glib-dev \
      libwebkitgtk-6.0-dev \
      mutter \
      pkg-config \
      procps \
      python3-gi \
      squashfs-tools \
      systemd \
      unzip \
      x11-utils \
      xauth \
      xdg-desktop-portal \
      xdg-desktop-portal-gtk \
      xdg-utils \
      xdotool \
      xfwm4 \
      xvfb \
      xz-utils \
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

# Node.js from nodejs.org, the version the CI workflows set up: Yon runs JavaScript handlers with Node.js only from
# 24.5 (TY2101), and Debian trixie's nodejs is 20.
ARG NODE_VERSION=24.22.2
ARG NODE_SHA256_ARM64=eb187837a2230d4047161ac0be008584e16b46f726e9c9490ee35dc6b17d9ba8
ARG NODE_SHA256_X64=48cc0e21e048790e54c0cf5749c2725005313d652651c40b7a21feb9697aaf75
RUN case "$(dpkg --print-architecture)" in arm64) arch=arm64 sha=${NODE_SHA256_ARM64} ;; *) arch=x64 sha=${NODE_SHA256_X64} ;; esac \
 && curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/node.tar.xz \
      "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${arch}.tar.xz" \
 && echo "${sha}  /tmp/node.tar.xz" | sha256sum -c - \
 && mkdir -p /opt/node \
 && tar -xJf /tmp/node.tar.xz -C /opt/node --strip-components=1 \
 && rm -rf /tmp/node.tar.xz /opt/node/share/doc /opt/node/share/man \
 && ln -s /opt/node/bin/node /opt/node/bin/npm /opt/node/bin/npx /usr/local/bin/ \
 && node --version

WORKDIR /workspace

FROM gate

# JDK 21 for Kotlin and Java (TeaVM), Ruby for the Tac Ruby runtime's mruby build, ICU for .NET, bzip2 for Pyodide,
# and the runtime libraries of swift.org's Ubuntu 24.04 toolchain.
RUN apt-get update \
 && apt-get install --no-install-recommends --yes \
      bzip2 \
      libcurl4t64 \
      libedit2 \
      libicu76 \
      libncurses6 \
      libsqlite3-0 \
      libxml2 \
      libz3-4 \
      openjdk-21-jdk-headless \
      ruby \
      tzdata \
 && rm -rf /var/lib/apt/lists/*

ARG GO_VERSION=1.27.3
ARG GO_SHA256_ARM64=a06e99309dbcbdca1a8ccd2282c3ee146e55930886e26b4f1193269587165f49
ARG GO_SHA256_AMD64=65efbc80fdc1fa2cd2631cad5b481777a68e8725f34e89f50fc6ac714e379b40
RUN case "$(dpkg --print-architecture)" in arm64) arch=arm64 sha=${GO_SHA256_ARM64} ;; *) arch=amd64 sha=${GO_SHA256_AMD64} ;; esac \
 && curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/go.tar.gz "https://go.dev/dl/go${GO_VERSION}.linux-${arch}.tar.gz" \
 && echo "${sha}  /tmp/go.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/go.tar.gz -C /usr/local \
 && rm -rf /tmp/go.tar.gz /usr/local/go/test /usr/local/go/api /usr/local/go/doc \
 && /usr/local/go/bin/go version

ENV DOTNET_ROOT=/opt/dotnet DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
ARG DOTNET_VERSION=10.0.302
RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/dotnet-install.sh https://dot.net/v1/dotnet-install.sh \
 && bash /tmp/dotnet-install.sh --version "${DOTNET_VERSION}" --install-dir "${DOTNET_ROOT}" \
 && "${DOTNET_ROOT}/dotnet" workload install wasm-tools wasm-tools-net9 \
 && "${DOTNET_ROOT}/dotnet" nuget locals all --clear \
 && rm -rf /tmp/dotnet-install.sh /root/.nuget /root/.local/share/NuGet \
 && "${DOTNET_ROOT}/dotnet" workload list

# lldb, sourcekit-lsp and DocC are dropped, since Tachyon only compiles with swiftc. The Swift SDK for WebAssembly is
# downloaded with curl and installed from the file: `swift sdk install <url>` aborts in Foundation's downloader on
# trixie ("_MultiHandle deallocated with non-zero retain count").
ARG SWIFT_VERSION=6.3.3
ARG SWIFT_SHA256_ARM64=47126395429653fa768d370655876ec1b68f6a95c7884f5e4f179700141c9b7f
ARG SWIFT_SHA256_AMD64=da8272a5fddccd65b1529ed0e52e04526e2eadd4237d58d6220efeb973c6cd19
ARG SWIFT_WASM_SDK_SHA256=cabfa08b73bb8ac783927ecd15fa386e99d0c139c5f232445067bcf58379cae7
RUN case "$(dpkg --print-architecture)" in arm64) folder=ubuntu2404-aarch64 suffix=-aarch64 sha=${SWIFT_SHA256_ARM64} ;; *) folder=ubuntu2404 suffix= sha=${SWIFT_SHA256_AMD64} ;; esac \
 && release="https://download.swift.org/swift-${SWIFT_VERSION}-release" \
 && curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/swift.tar.gz \
      "${release}/${folder}/swift-${SWIFT_VERSION}-RELEASE/swift-${SWIFT_VERSION}-RELEASE-ubuntu24.04${suffix}.tar.gz" \
 && echo "${sha}  /tmp/swift.tar.gz" | sha256sum -c - \
 && mkdir -p /opt/swift \
 && tar -xzf /tmp/swift.tar.gz -C /opt/swift --strip-components=1 \
 && rm /tmp/swift.tar.gz \
 && cd /opt/swift/usr \
 && rm -rf bin/lldb* lib/liblldb* lib/python3* bin/sourcekit-lsp lib/libsourcekitdInProc.so bin/docc share/docc share/doc share/man \
 && curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/swift-wasm-sdk.artifactbundle.tar.gz \
      "${release}/wasm-sdk/swift-${SWIFT_VERSION}-RELEASE/swift-${SWIFT_VERSION}-RELEASE_wasm.artifactbundle.tar.gz" \
 && echo "${SWIFT_WASM_SDK_SHA256}  /tmp/swift-wasm-sdk.artifactbundle.tar.gz" | sha256sum -c - \
 && /opt/swift/usr/bin/swift sdk install --swift-sdks-path /opt/swift-sdks /tmp/swift-wasm-sdk.artifactbundle.tar.gz \
 && rm /tmp/swift-wasm-sdk.artifactbundle.tar.gz \
 && test -d "/opt/swift-sdks/swift-${SWIFT_VERSION}-RELEASE_wasm.artifactbundle/swift-${SWIFT_VERSION}-RELEASE_wasm/wasm32-unknown-wasip1/WASI.sdk" \
 && ln -s "/opt/swift-sdks/swift-${SWIFT_VERSION}-RELEASE_wasm.artifactbundle/swift-${SWIFT_VERSION}-RELEASE_wasm/wasm32-unknown-wasip1" /opt/swift-wasm-sdk \
 && rm -rf /root/.swiftpm /root/.cache \
 && /opt/swift/usr/bin/swiftc --version
ENV TAC_SWIFT_WASM_COMPILER=/opt/swift/usr/bin/swiftc TAC_SWIFT_WASM_SDK=/opt/swift-wasm-sdk

ARG KOTLIN_VERSION=2.1.10
ARG KOTLIN_SHA256=c6e9e2636889828e19c8811d5ab890862538c89dc2a3101956dfee3c2a8ba6b1
ARG KOTLIN_WASM_STDLIB_SHA256=db9ab6772a2d3af9121d570a058cc64cf36893e1c089726b799b4d1252184ca2
RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/kotlin.zip \
      "https://github.com/JetBrains/kotlin/releases/download/v${KOTLIN_VERSION}/kotlin-compiler-${KOTLIN_VERSION}.zip" \
 && echo "${KOTLIN_SHA256}  /tmp/kotlin.zip" | sha256sum -c - \
 && unzip -q /tmp/kotlin.zip -d /opt \
 && rm /tmp/kotlin.zip \
 && mkdir -p /opt/kotlin-wasm \
 && curl --proto '=https' --tlsv1.2 -fsSL -o "/opt/kotlin-wasm/kotlin-stdlib-wasm-js-${KOTLIN_VERSION}.klib" \
      "https://repo1.maven.org/maven2/org/jetbrains/kotlin/kotlin-stdlib-wasm-js/${KOTLIN_VERSION}/kotlin-stdlib-wasm-js-${KOTLIN_VERSION}.klib" \
 && echo "${KOTLIN_WASM_STDLIB_SHA256}  /opt/kotlin-wasm/kotlin-stdlib-wasm-js-${KOTLIN_VERSION}.klib" | sha256sum -c - \
 && ln -s "/opt/kotlin-wasm/kotlin-stdlib-wasm-js-${KOTLIN_VERSION}.klib" /opt/kotlin-wasm/kotlin-stdlib-wasm-js.klib \
 && /opt/kotlinc/bin/kotlinc -version
ENV TAC_KOTLIN_WASM_STDLIB=/opt/kotlin-wasm/kotlin-stdlib-wasm-js.klib

ARG TEAVM_VERSION=0.15.0
RUN mkdir -p /opt/teavm \
 && cd /opt/teavm \
 && maven=https://repo1.maven.org/maven2/org/teavm \
 && for jar in \
      "cli cli-${TEAVM_VERSION}-all.jar c6a1ecb81dd90b66985ac7b0f09db55bec2076fa024428c7fe72b800b6b48cae" \
      "teavm-classlib teavm-classlib-${TEAVM_VERSION}.jar 7d805be4670a892a34c89d9d750a8afd65e2f8201b7b3a10343634ea19d1c410" \
      "teavm-jso-impl teavm-jso-impl-${TEAVM_VERSION}.jar 9a8d15205807b70d7efdfbd4c1356304954e20146e464aec3830dbc6616ef57c" \
      "teavm-metaprogramming-impl teavm-metaprogramming-impl-${TEAVM_VERSION}.jar 616d30aa28f7b0180a61c2289b11c77bf84c715bd79f255e1e42400edb5b6d70" \
      "teavm-platform teavm-platform-${TEAVM_VERSION}.jar 999d369ab15aa83c17987d048ba59560759169891de7e13a5a1c2bd93dd060a8"; do \
      set -- $jar \
   && curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "${maven}/$1/${TEAVM_VERSION}/$2" \
   && echo "$3  $2" | sha256sum -c - || exit 1; \
    done
ENV TAC_TEAVM_HOME=/opt/teavm

ARG DART_VERSION=3.13.1
ARG DART_SHA256_ARM64=5141d5ac62dabfcf0d3dd8fbf5c4d143d0092ef42fad0f25f6cf6587e5cfc3bf
ARG DART_SHA256_X64=9251c4606ebb31480c45f430be69fce2697e3b888a0a828b9e16e7d23403d72a
RUN case "$(dpkg --print-architecture)" in arm64) arch=arm64 sha=${DART_SHA256_ARM64} ;; *) arch=x64 sha=${DART_SHA256_X64} ;; esac \
 && curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/dart.zip \
      "https://storage.googleapis.com/dart-archive/channels/stable/release/${DART_VERSION}/sdk/dartsdk-linux-${arch}-release.zip" \
 && echo "${sha}  /tmp/dart.zip" | sha256sum -c - \
 && unzip -q /tmp/dart.zip -d /opt \
 && rm /tmp/dart.zip \
 && /opt/dart-sdk/bin/dart --version

# The emsdk commit and Emscripten version that runtimes/ruby-web/sources.txt pins.
ARG EMSDK_COMMIT=e566f7bdcc7735f44037911c24b87a58a3c93145
ARG EMSDK_SHA256=8634b12869818ff088757141804dd4b36713add0a5669a14de1a7f72609c26df
ARG EMSCRIPTEN_VERSION=6.0.10
RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/emsdk.tar.gz \
      "https://github.com/emscripten-core/emsdk/archive/${EMSDK_COMMIT}.tar.gz" \
 && echo "${EMSDK_SHA256}  /tmp/emsdk.tar.gz" | sha256sum -c - \
 && mkdir -p /opt/emsdk \
 && tar -xzf /tmp/emsdk.tar.gz -C /opt/emsdk --strip-components=1 \
 && rm /tmp/emsdk.tar.gz \
 && cd /opt/emsdk \
 && python3 emsdk.py install "${EMSCRIPTEN_VERSION}" \
 && python3 emsdk.py activate "${EMSCRIPTEN_VERSION}" \
 && rm -rf /opt/emsdk/downloads /opt/emsdk/zips \
 && /opt/emsdk/upstream/emscripten/emcc --version

# Tachyon reads the Pyodide core files from its cache (python-web/<version>), as install-toolchains.mjs puts them.
ARG PYODIDE_VERSION=314.0.7
ARG PYODIDE_SHA256=2abdcc2e35208af406e07724cffa85bc582ced97e9028383ecf5462541393f95
RUN curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/pyodide.tar.bz2 \
      "https://github.com/pyodide/pyodide/releases/download/${PYODIDE_VERSION}/pyodide-core-${PYODIDE_VERSION}.tar.bz2" \
 && echo "${PYODIDE_SHA256}  /tmp/pyodide.tar.bz2" | sha256sum -c - \
 && mkdir -p /tmp/pyodide "/root/.cache/tachyon/python-web/${PYODIDE_VERSION}" \
 && tar -xjf /tmp/pyodide.tar.bz2 -C /tmp/pyodide \
 && for name in pyodide.mjs pyodide.asm.mjs pyodide.asm.wasm python_stdlib.zip pyodide-lock.json; do \
      cp "/tmp/pyodide/pyodide/${name}" "/root/.cache/tachyon/python-web/${PYODIDE_VERSION}/${name}" || exit 1; \
    done \
 && rm -rf /tmp/pyodide /tmp/pyodide.tar.bz2

# Swift stays off PATH (Tachyon finds it through TAC_SWIFT_WASM_COMPILER), so its clang and lld never shadow the
# system's; Emscripten's folder has only its em* launchers.
ENV PATH=/usr/local/cargo/bin:/opt/node/bin:/usr/local/go/bin:/opt/dotnet:/opt/kotlinc/bin:/opt/dart-sdk/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/opt/emsdk/upstream/emscripten

WORKDIR /workspace
