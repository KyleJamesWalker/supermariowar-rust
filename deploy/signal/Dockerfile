# matchbox_server, the WebRTC signaling server for netplay v2 (docs/design/NETPLAY_V2.md), unmodified from crates.io.
FROM mirror.gcr.io/library/rust:1.94-slim-bookworm AS build
RUN cargo install matchbox_server --version =0.14.0 --locked --root /out

FROM gcr.io/distroless/cc-debian12:nonroot
LABEL org.opencontainers.image.source="https://github.com/KyleJamesWalker/supermariowar-rust" \
      org.opencontainers.image.description="Super Mario War netplay v2 signaling (matchbox_server 0.14.0)"
COPY --from=build /out/bin/matchbox_server /usr/local/bin/matchbox_server
ENV HOST=0.0.0.0:3536
EXPOSE 3536
ENTRYPOINT ["/usr/local/bin/matchbox_server"]
