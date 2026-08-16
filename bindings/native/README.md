# Internal native bridge

`lean-multisig-native` is a private Rust bridge for managed-language bindings in this repository.
It produces a `cdylib` for Java FFM and a `staticlib` for Go cgo, and owns the opaque Rust handles
and buffers behind the `lms_*` ABI declared in
[`include/lean_multisig_native.h`](include/lean_multisig_native.h).

It is consumed by the Java 25 FFM and Go cgo bindings. This crate and header are internal: they are
not a supported or versioned general-purpose C API. `scripts/stage-static.sh` builds and stages one
platform's static archive for Go builds and release packaging.

The bridge's `LMCG-v1` claim-context encoding is likewise private. Language wrappers should use it
only to pass `ClaimSigners` context to the bridge; it is not a public serialization format.
