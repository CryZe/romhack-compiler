# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    cargo clean
    cross rustc -p romhack-patcher --target $TARGET --release -- -C link-arg=-Wl,-rpath,'$ORIGIN'

    cp target/$TARGET/release/romhack-patcher $stage/
    case $TRAVIS_OS_NAME in
        linux)
            cp $(find target/release/build/ -type f -iname 'libui.so*') $stage/.
        osx)
            cp $(find target/release/build/ -type f -iname 'libui.dylib*') $stage/.
            ;;
    esac

    cd $stage
    tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *
    cd $src

    rm -rf $stage
}

main
