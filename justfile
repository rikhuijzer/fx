shebang := '''
    /usr/bin/env bash
    set -euxo pipefail
'''

alias r := release
alias s := serve

default:
    just --list

# Build release and copy files to public/
#
# The copy might speed up Docker builds due to fewer files in the context.
release:
    #!{{shebang}}

    cargo build --release

    rm -rf public
    mkdir -p public
    cp --verbose target/release/fx public/

    cat > public/Dockerfile << EOF
    FROM debian:bookworm-slim
    WORKDIR /app
    COPY fx /usr/local/bin
    ENTRYPOINT ["/usr/local/bin/fx", "serve"]
    EOF

    cat public/Dockerfile

    # To avoid accidentally editing the files in public manually.
    chmod 444 public/Dockerfile

serve:
    #!{{shebang}}

    cargo watch -x "run -- serve --admin-password=localpw"
