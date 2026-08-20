FROM registry.access.redhat.com/ubi9/ubi:latest AS builder
RUN dnf install -y cargo rust openssl-devel pkgconf-pkg-config gcc && dnf clean all
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM registry.access.redhat.com/ubi9/ubi:latest
RUN dnf install -y iproute openssl-libs shadow-utils tar util-linux && dnf clean all
RUN useradd --uid 1001 --create-home --shell /bin/bash sandbox \
    && install -d -o sandbox -g sandbox /sandbox
COPY --from=builder --chown=sandbox:sandbox /build/target/release/proxy-m365-write /sandbox/proxy-m365-write
RUN chmod 755 /sandbox/proxy-m365-write
USER sandbox
EXPOSE 18081
ENTRYPOINT ["/sandbox/proxy-m365-write"]
