#!/bin/bash

set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

channel() {
    if [ -n "${TRAVIS}" ]; then
        if [ "${TRAVIS_RUST_VERSION}" = "${CHANNEL}" ]; then
            pwd
            (set -x; cargo "$@")
        fi
    elif [ -n "${APPVEYOR}" ]; then
        if [ "${APPVEYOR_RUST_CHANNEL}" = "${CHANNEL}" ]; then
            pwd
            (set -x; cargo "$@")
        fi
    else
        pwd
        (set -x; cargo "+${CHANNEL}" "$@")
    fi
}

if [ -n "${NIGHTLY}" ]; then
    if [ -n "${TRAVIS}" ] && ! cargo install rustfmt-nightly --debug --force; then
        echo "COULD NOT COMPILE RUSTFMT, IGNORING FMT"
        exit
    fi

    cargo fmt -- --write-mode diff

    # cached installation will not work on a later nightly
    if [ -n "${TRAVIS}" ] && ! cargo install clippy --debug --force; then
        echo "COULD NOT COMPILE CLIPPY, IGNORING CLIPPY TESTS"
        exit
    fi

    cargo clippy
else
    for CHANNEL in nightly stable beta; do
        channel clean
        channel build
        channel test
    done
fi
