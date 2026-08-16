#!/bin/sh
# Build and stage one target's static bridge archive for cgo consumers and release packaging.
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 STAGING_DIRECTORY" >&2
    exit 64
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
archive_dir=$1
target_dir=${CARGO_TARGET_DIR:-"$repo_dir/target"}

cargo build --manifest-path "$repo_dir/bindings/native/Cargo.toml" --release

mkdir -p "$archive_dir"
install -m 0644 "$target_dir/release/liblean_multisig_native.a" "$archive_dir/liblean_multisig_native.a"
