# leanMultisig Zig bindings

Safe Zig bindings for leanMultisig XMSS signatures and recursive aggregation. The package owns
opaque native handles and exposes normal Zig structs; callers must call `deinit` on native objects
and on `ClaimGroups` returned by `verifiedClaims`.

The binding links a target-matched static `liblean_multisig_native.a` produced by the shared Rust
bridge. It supports Linux x86_64/aarch64 and universal macOS archives. The archive is an
implementation detail of this binding, not a public C API.

## Requirements

- Zig 0.16 or newer
- A target-matched `liblean_multisig_native.a`

## Build from this repository

Install Zig and `cargo-zigbuild`, then stage the native archive for your platform. On Linux x86_64:

```sh
rustup target add x86_64-unknown-linux-gnu
cargo install cargo-zigbuild --locked
sh bindings/native/scripts/stage-static.sh x86_64-unknown-linux-gnu bindings/zig/.native
cd bindings/zig
zig build test -Dnative-dir=.native
```

For Linux aarch64, stage `aarch64-unknown-linux-gnu`; for macOS, stage
`universal2-apple-darwin` on macOS after adding both Apple Rust targets. The release process can
provide those same static archives instead of building them locally.

## Example

```zig
const std = @import("std");
const lms = @import("lean_multisig");

pub fn main() !void {
    const allocator = std.heap.page_allocator;
    try lms.setup();

    var key = try lms.SecretKey.fromSeed([_]u8{1} ** 32, 100, 115);
    defer key.deinit();
    const claim = lms.Claim{ .message = [_]u8{42} ** 32, .slot = 100 };
    var signature = try key.sign(claim);
    defer signature.deinit();

    const signer = try key.publicKey(allocator);
    try std.debug.assert(try signature.verify(&[_]lms.PublicKey{signer}, claim));
}
```

Signature and proof encodings do not contain their claim/signer context. Supply the original
context to `Signature.fromBytes`, `MultiClaimProof.fromBytes`, `verify`, and `verifyClaims`.
