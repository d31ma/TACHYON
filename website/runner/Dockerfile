ARG TOOLCHAIN_IMAGE
ARG LAMBDA_ADAPTER=public.ecr.aws/awsguru/aws-lambda-adapter:0.9.1

FROM ${LAMBDA_ADAPTER} AS adapter

FROM ${TOOLCHAIN_IMAGE}
USER root
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
RUN bash -c 'set -eo pipefail; source /opt/tachyon/ci-env.sh; ls -d /opt/tachyon/snippets/*/project | xargs -P 4 -I {} sh -c "cd {} && ty bundle . > /dev/null"'
CMD ["bash", "-c", "source /opt/tachyon/ci-env.sh && exec bun /opt/tachyon/website/runner/main.ts"]
