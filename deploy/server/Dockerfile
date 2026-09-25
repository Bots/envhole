FROM python:3.12-slim-bookworm@sha256:392307d22300de8b5986851a12d9176dfc0fc073e65bf6523ebd7dcbeb23564e

ARG SERVER_PACKAGE
RUN test -n "$SERVER_PACKAGE" \
    && python -m pip install --disable-pip-version-check --no-cache-dir "$SERVER_PACKAGE" \
    && groupadd --gid 65532 envhole \
    && useradd --uid 65532 --gid 65532 --no-create-home --shell /usr/sbin/nologin envhole \
    && install -d -o 65532 -g 65532 -m 0700 /data

USER 65532:65532
WORKDIR /data
VOLUME ["/data"]
