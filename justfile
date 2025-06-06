shebang := '''
    /usr/bin/env bash
    set -euo pipefail
'''

alias r := release
alias s := serve

default:
    just --list

tag:
    #!{{shebang}}

    METADATA="$(cargo metadata --format-version=1 --no-deps)"
    VERSION="$(echo $METADATA | jq -r '.packages[0].version')"
    echo "VERSION: $VERSION"
    TAGNAME="v$VERSION"
    echo "TAGNAME: $TAGNAME"

    echo "Existing tags:"
    git tag
    echo ""
    read -p "Are you sure you want to tag $TAGNAME? Type YES to continue. " REPLY
    if [[ $REPLY == "YES" ]]; then
        echo ""
        git tag -a $TAGNAME -m "Release $VERSION"
        git push origin $TAGNAME
        exit 0
    else
        echo ""
        echo "Did not receive YES, aborting"
        exit 1
    fi

# Build release and copy files to public/
#
# The copy might speed up Docker builds due to fewer files in the context.
release:
    #!{{shebang}}

    rm -rf public
    mkdir -p public

    ARCH="$(uname -m)"
    echo "ARCH: $ARCH"
    OS="$(uname -s)"
    echo "OS: $OS"

    if [[ $ARCH == "x86_64" && $OS == "Linux" ]]; then
        TARGET="x86_64-unknown-linux-musl"
        cargo build -p fx --release --target="$TARGET"
        cp --verbose "target/$TARGET/release/fx" "public/"
    else
        cargo build -p fx --release
        cp --verbose "target/release/fx" "public/"
    fi

    cat > public/Dockerfile << EOF
    FROM gcr.io/distroless/cc-debian12
    ENV FX_PRODUCTION="true"
    COPY fx /
    CMD ["/fx", "serve"]
    EOF

    cat public/Dockerfile

    # To avoid accidentally editing the files in public manually.
    chmod 444 public/Dockerfile

serve:
    #!{{shebang}}

    cargo watch -x "run -- serve --password=test-password"
