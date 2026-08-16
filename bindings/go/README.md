# leanMultisig Go bindings

Go bindings for [leanMultisig](https://github.com/leanEthereum/leanMultisig), backed by a native
Rust static archive linked into the final program through cgo. The Go API uses normal Go values and
`Close` methods; it does not expose C pointers or the internal bridge ABI.

## Requirements

- Go 1.24 or newer with cgo enabled
- A C linker for the target platform

Each Go release carries a target-matched `liblean_multisig_native.a` in its internal native-asset
directory. Go build tags select that archive and link it into the final Go binary. Ordinary users do
not install a native package or compile Rust.

## Source build and test

From the repository root, stage the local Linux amd64 archive in the layout used by the Go package:

```sh
sh bindings/native/scripts/stage-static.sh bindings/go/internal/native/linux_amd64
cd bindings/go
go test ./...
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
