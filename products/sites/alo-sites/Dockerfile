# Build context is the repository root (the Cargo workspace);
# compose invokes this as: context "../.." dockerfile "products/sites/alo-sites/Dockerfile".
FROM rust:1.90-slim AS build
WORKDIR /src
# SQLX_OFFLINE + the checked-in .sqlx query cache let sqlx's compile-time
# query checks pass without a database at build time.
ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY platform ./platform
COPY products ./products
COPY suite ./suite
COPY migrate ./migrate
# Only this binary. alo-sites runs no migrations — the authenticated alo-jmap
# service owns the schema — so nothing else needs to be in the image.
#
# Jobs are capped because the production box has two cores, four gigabytes and
# live mail on it: an unconstrained rustc can take the memory the SMTP and JMAP
# services are using. Override on a bigger machine with
# `--build-arg CARGO_JOBS=8`.
ARG CARGO_JOBS=1
RUN cargo build --release --jobs ${CARGO_JOBS} --package alo-sites --bins

FROM debian:bookworm-slim AS runtime
# Blob reads need a CA bundle for nothing today, but the image serves images
# from disk and resolves nothing outward; keep it minimal.
RUN groupadd --system --gid 10001 alo && useradd --system --no-create-home --uid 10001 --gid 10001 alo \
    && mkdir -p /var/lib/alo/blobs \
    && chown -R alo /var/lib/alo
COPY --from=build /src/target/release/alo-sites /usr/local/bin/alo-sites
USER alo
ENV ALO_BLOB_DIR=/var/lib/alo/blobs \
    ALO_SITES_ADDR=0.0.0.0:8082
# The public site server listens on plain HTTP internally; Caddy terminates
# TLS on 443 — including the on-demand certificates for customer subdomains
# and custom domains, whose issuance it checks against /internal/tls/ask here.
EXPOSE 8082
ENTRYPOINT ["/usr/local/bin/alo-sites"]
