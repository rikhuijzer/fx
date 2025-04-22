FROM debian:bookworm-slim
WORKDIR /app
COPY target/release/fx /usr/local/bin
ENTRYPOINT ["/usr/local/bin/fx", "serve"]
