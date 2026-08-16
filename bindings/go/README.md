# leanMultisig Go bindings

Go bindings for [leanMultisig](https://github.com/leanEthereum/leanMultisig), backed by a native
Rust static archive linked into the final program through cgo. The Go API uses normal Go values and
`Close` methods; it does not expose C pointers or the internal bridge ABI.

## Requirements

- Go 1.24 or newer with cgo enabled
- A C linker for the target platform
- An installed, target-matched `lean-multisig-native` static package discoverable by `pkg-config`

The installed native package contains `liblean_multisig_native.a`,
`lean_multisig_native.h`, and `lean-multisig-native.pc`. The forthcoming release workflow will
produce one package per supported OS/architecture; ordinary Go users will link that prebuilt archive
and will not compile Rust.

## Source build and test

From the repository root, stage a local static native package and point `pkg-config` to it:

```sh
stage_dir=$(mktemp -d)
sh bindings/native/scripts/stage-static.sh "$stage_dir"
cd bindings/go
PKG_CONFIG_PATH="$stage_dir/lib/pkgconfig" go test ./...
```

## Example

```go
package main

import (
    "log"

    leanmultisig "github.com/ethereum/lean-multisig-bindings/bindings/go"
)

func main() {
    var seed, message [32]byte
    key, err := leanmultisig.SecretKeyFromSeed(seed, 100, 115)
    if err != nil {
        log.Fatal(err)
    }
    defer key.Close()

    if err := leanmultisig.Setup(); err != nil {
        log.Fatal(err)
    }
    signature, err := key.Sign(leanmultisig.Claim{Message: message, Slot: 100})
    if err != nil {
        log.Fatal(err)
    }
    defer signature.Close()
}
```

Signature and multi-claim-proof byte encodings intentionally omit their claim/signer context.
Supply that context when calling `SignatureFromBytes`, `MultiClaimProofFromBytes`, `Verify`, or
`VerifyClaims`.
