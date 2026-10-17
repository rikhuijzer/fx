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

    if [[ $OS == "Linux" && $ARCH == "x86_64" ]]; then
        TARGET="x86_64-unknown-linux-musl"
    elif [[ $OS == "Linux" && ( $ARCH == "aarch64" || $ARCH == "arm64" ) ]]; then
        TARGET="aarch64-unknown-linux-musl"
    else
        echo "Container releases require Linux on amd64 or arm64" >&2
        exit 1
    fi

    # Both CI runners build natively. Use musl for Rust linking and native dependencies.
    TARGET_ENV="${TARGET//-/_}"
    export "CC_${TARGET_ENV}=musl-gcc"
    export "CARGO_TARGET_${TARGET_ENV^^}_LINKER=musl-gcc"
    cargo build -p bloglite --release --locked --target="$TARGET"
    cp --verbose "target/$TARGET/release/bloglite" "public/"

    # The static runtime has no dynamic loader or shared libraries.
    readelf --program-headers public/bloglite > public/program-headers.txt
    readelf --dynamic public/bloglite > public/dynamic.txt
    if grep -q 'INTERP' public/program-headers.txt || grep -q '(NEEDED)' public/dynamic.txt; then
        echo "Release binary must be statically linked" >&2
        exit 1
    fi
    rm public/program-headers.txt public/dynamic.txt

    cat > public/Dockerfile << EOF
    FROM gcr.io/distroless/static-debian13
    ENV BLOGLITE_PRODUCTION="true"
    COPY bloglite /
    CMD ["/bloglite", "serve"]
    EOF

    cat public/Dockerfile

    # To avoid accidentally editing the files in public manually.
    chmod 444 public/Dockerfile

serve:
    #!{{shebang}}

    cargo watch -x "run -- serve --password=test-password"
