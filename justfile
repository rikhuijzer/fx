shebang := '''
    /usr/bin/env bash
    set -euxo pipefail
'''

alias r := release

default:
    just --list

# Build release and copy files to public/
#
# The copy might speed up Docker builds since due to fewer files in the context.
release:
    #!{{shebang}}

    cargo build --release

    mkdir -p public
    cp --verbose target/release/fedx public/
    cp --verbose fly.toml public/

    cat > public/Dockerfile << EOF
    FROM debian:bookworm-slim
    WORKDIR /app
    COPY fedx /usr/local/bin
    ENTRYPOINT ["/usr/local/bin/fedx", "serve"]
    EOF

    cat public/Dockerfile

    # To avoid accidentally editing the files in public manually.
    chmod 444 public/Dockerfile
    chmod 444 public/fedx
    chmod 444 public/fly.toml
