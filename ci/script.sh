# This script takes care of testing your crate

set -ex

main() {
    cat > Cross.toml <<EOF
[target.x86_64-unknown-linux-gnu]
image = "cryze/x86_64-unknown-linux-gnu-romhack-compiler"
[target.i686-unknown-linux-gnu]
image = "cryze/i686-unknown-linux-gnu-romhack-compiler"
EOF

    cross build -p romhack-patcher --target $TARGET --release

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi
