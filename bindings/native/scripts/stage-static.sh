#!/bin/sh
# Build and stage one cross-compiled static bridge archive for cgo consumers and release packaging.
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: $0 RUST_TARGET STAGING_DIRECTORY" >&2
    exit 64
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
rust_target=$1
archive_dir=$2
target_dir=${CARGO_TARGET_DIR:-"$repo_dir/target"}

cargo zigbuild --manifest-path "$repo_dir/bindings/native/Cargo.toml" --release --target "$rust_target"

mkdir -p "$archive_dir"
install -m 0644 "$target_dir/$rust_target/release/liblean_multisig_native.a" "$archive_dir/liblean_multisig_native.a"
