#!/bin/sh
# Build and stage one target's static bridge archive for cgo consumers and release packaging.
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 STAGING_DIRECTORY" >&2
    exit 64
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
stage_dir=$1
target_dir=${CARGO_TARGET_DIR:-"$repo_dir/target"}
native_version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$repo_dir/bindings/native/Cargo.toml" | head -n 1)

if [ -z "$native_version" ]; then
    echo "could not determine native bridge version" >&2
    exit 1
fi

cargo build --manifest-path "$repo_dir/bindings/native/Cargo.toml" --release

mkdir -p "$stage_dir/lib/pkgconfig" "$stage_dir/include"
install -m 0644 "$target_dir/release/liblean_multisig_native.a" "$stage_dir/lib/liblean_multisig_native.a"
install -m 0644 "$repo_dir/bindings/native/include/lean_multisig_native.h" "$stage_dir/include/lean_multisig_native.h"
sed \
    -e "s|@PREFIX@|$stage_dir|g" \
    -e "s|@VERSION@|$native_version|g" \
    "$repo_dir/bindings/native/lean-multisig-native.pc.in" \
    > "$stage_dir/lib/pkgconfig/lean-multisig-native.pc"
