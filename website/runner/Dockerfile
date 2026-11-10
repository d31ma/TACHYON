ARG TOOLCHAIN_IMAGE=tachyon-linux-ci:required
ARG LAMBDA_ADAPTER=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1
ARG UBUNTU=24.04

FROM ${LAMBDA_ADAPTER} AS adapter

FROM ${TOOLCHAIN_IMAGE} AS toolchains
USER root
RUN set -eu \
 && cd /opt/tachyon \
 && rm -rf swift-* cargo/bin/cargo-nextest cargo/bin/sccache \
 && rm -rf rustup/toolchains/*/lib/rustlib/wasm32-unknown-unknown rustup/toolchains/*/bin/clippy-driver rustup/toolchains/*/bin/cargo-clippy \
      rustup/toolchains/*/bin/rustfmt rustup/toolchains/*/bin/cargo-fmt \
 && rm -rf rustup/downloads/* rustup/tmp/* rustup/update-hashes \
 && DOTNET_ROOT=/opt/tachyon/dotnet DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1 HOME=/tmp dotnet/dotnet workload uninstall wasm-tools wasm-tools-net9 \
 && rm -rf /tmp/.dotnet /tmp/.nuget dotnet/library-packs \
 && sdk_major=$(ls dotnet/sdk | sort -V | tail -1 | cut -d. -f1) \
 && find dotnet/shared/Microsoft.NETCore.App dotnet/shared/Microsoft.AspNetCore.App -mindepth 1 -maxdepth 1 ! -name "$sdk_major.*" -exec rm -rf {} + \
 && sed -e 's# RUSTC_WRAPPER=sccache SCCACHE_CACHE_SIZE=10G##' -e 's#\$HOME/tachyon-ci/emscripten/emsdk/upstream/emscripten:##' \
      -e 's#CARGO_TARGET_DIR=\$HOME/tachyon-ci/target#CARGO_TARGET_DIR=/tmp/tachyon-runner-cargo-target#' ci-env.sh > runner-env.sh \
 && dpkg-query -W -f '${db:Status-Abbrev}${Package}\n' 'php*-cgi' 'lua5.*' 'openjdk-*-jdk-headless' | sed -n 's/^ii *//p' > /opt/tachyon/apt-packages \
 && test "$(wc -l < /opt/tachyon/apt-packages)" -eq 3 \
 && cp /etc/os-release /opt/tachyon/toolchain-os-release

FROM ubuntu:${UBUNTU}
ARG CI_UID=1001
ENV DEBIAN_FRONTEND=noninteractive LANG=C.UTF-8
COPY --from=toolchains /opt/tachyon/apt-packages /opt/tachyon/toolchain-os-release /tmp/toolchains/
RUN set -eu \
 && cmp /etc/os-release /tmp/toolchains/toolchain-os-release \
 && printf '%s\n' 'path-exclude=/usr/share/doc/*' 'path-include=/usr/share/doc/*/copyright' 'path-exclude=/usr/share/man/*' \
      'path-exclude=/usr/share/info/*' 'path-exclude=/usr/share/locale/*' 'path-include=/usr/share/locale/en*' > /etc/dpkg/dpkg.cfg.d/runner-excludes \
 && mkdir -p /usr/share/man/man1 \
 && apt-get update -q \
 && apt-get install -y -q --no-install-recommends ca-certificates tzdata clang g++ libc6-dev perl \
      libssl3t64 libyaml-0-2 libffi8 libgmp10 zlib1g libicu74 \
      $(cat /tmp/toolchains/apt-packages) \
 && ln -sf "$(ls /usr/bin/lua5.* | head -1)" /usr/local/bin/lua \
 && { id -u ubuntu > /dev/null 2>&1 && userdel -r ubuntu || true; } \
 && useradd -m -s /bin/bash -u $CI_UID ci \
 && ln -s /opt/tachyon/rustup /home/ci/.rustup && chown -h ci:ci /home/ci/.rustup \
 && rm -rf /var/lib/apt/lists/* /var/cache/apt/* /var/log/apt /var/log/dpkg.log /tmp/*
COPY --from=toolchains --chown=ci:ci /opt/tachyon /opt/tachyon
COPY --from=adapter /lambda-adapter /opt/extensions/lambda-adapter
COPY --chmod=755 ty /usr/local/bin/ty
COPY --chown=ci website /opt/tachyon/website
COPY --chown=ci runnable /opt/tachyon/snippets
RUN mkdir -p /opt/tachyon/cache && chown ci /opt/tachyon/cache
USER ci
ENV HOME=/tmp TMPDIR=/tmp TAC_CACHE_DIR=/opt/tachyon/cache \
    SNIPPET_HOST_PROJECTS=/opt/tachyon/snippets SNIPPET_HOST_WEBSITE=/opt/tachyon/website \
    SNIPPET_HOST_TY=/usr/local/bin/ty WEBSITE_PORT=8000 \
    AWS_LWA_PORT=8000 AWS_LWA_READINESS_CHECK_PATH=/
RUN bash -c 'set -eo pipefail; source /opt/tachyon/runner-env.sh; ls -d /opt/tachyon/snippets/*/project | xargs -P 4 -I {} sh -c "cd {} && ty bundle . > /dev/null"; rm -rf /opt/tachyon/cache/runtime/handlers-v1 /tmp/* /tmp/.cache; du -sh /opt/tachyon/snippets /opt/tachyon/cache'
CMD ["bash", "-c", "source /opt/tachyon/runner-env.sh && exec bun /opt/tachyon/website/runner/main.ts"]
