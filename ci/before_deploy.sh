# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd)

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    # TODO Update this to build the artifacts that matter to you
    cross rustc -p jorup --bin jorup --target $TARGET --release -- -C lto

    cp target/$TARGET/release/jorup $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET
}

main
