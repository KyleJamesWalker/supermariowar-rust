# smw_relay, the netplay lobby and WebSocket relay for the browser build (docs/RELAY.md).
FROM rust:1.94-slim-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/smw-globals crates/smw-globals
COPY relay/Cargo.toml relay/
COPY relay/src relay/src
COPY src/common/file_io.rs src/common/file_io.rs
COPY src/common_netplay src/common_netplay
COPY src/globals src/globals
COPY src/server src/server
# Cargo needs a target for every workspace member; the game itself is not built here.
RUN touch src/main.rs \
    && cargo build --release --locked --package smw_relay \
    && mkdir -p /out/srv \
    && cp target/release/smw_relay /out/ \
    && ln -s /dev/null /out/srv/serverlog.txt

FROM gcr.io/distroless/cc-debian12:nonroot
LABEL org.opencontainers.image.source="https://github.com/KyleJamesWalker/supermariowar-rust" \
      org.opencontainers.image.description="Super Mario War netplay lobby and WebSocket relay for the browser build"
COPY --from=build /out/smw_relay /usr/local/bin/smw_relay
# The lobby also appends to ./serverlog.txt; this keeps the log on stdout only.
COPY --from=build /out/srv /srv
WORKDIR /srv
ENV SMW_RELAY_PORT=8080
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 CMD ["/usr/local/bin/smw_relay", "--healthcheck"]
ENTRYPOINT ["/usr/local/bin/smw_relay"]
