#!/bin/sh
# Verify staging always uses cargo-zigbuild and preserves the target-specific layout.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

mkdir -p "$tmp_dir/bin"
cat > "$tmp_dir/bin/cargo" <<'EOF'
#!/bin/sh
set -eu

printf '%s\n' "$@" > "$TEST_CARGO_ARGS"

target=
while [ "$#" -gt 0 ]; do
    if [ "$1" = "--target" ]; then
        target=$2
        break
    fi
    shift
done

mkdir -p "$CARGO_TARGET_DIR/$target/release"
: > "$CARGO_TARGET_DIR/$target/release/liblean_multisig_native.a"
EOF
chmod +x "$tmp_dir/bin/cargo"

stage() {
    target=$1
    archive_dir="$tmp_dir/$target"
    args_file="$tmp_dir/$target.args"

    CARGO_TARGET_DIR="$tmp_dir/target" \
        TEST_CARGO_ARGS="$args_file" \
        PATH="$tmp_dir/bin:$PATH" \
        sh "$script_dir/stage-static.sh" "$target" "$archive_dir"

    test "$(sed -n '1p' "$args_file")" = "zigbuild"
    test "$(sed -n '2p' "$args_file")" = "--manifest-path"
    test "$(sed -n '4p' "$args_file")" = "--release"
    test "$(sed -n '5p' "$args_file")" = "--target"
    test "$(sed -n '6p' "$args_file")" = "$target"
    test -f "$archive_dir/liblean_multisig_native.a"
}

stage x86_64-unknown-linux-gnu
stage aarch64-unknown-linux-gnu
stage universal2-apple-darwin
