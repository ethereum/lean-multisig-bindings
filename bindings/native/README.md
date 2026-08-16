# Internal native bridge

`lean-multisig-native` is a private Rust `cdylib` bridge for managed-language bindings in this
repository. It owns the opaque Rust handles and buffers behind the `lms_*` ABI declared in
[`include/lean_multisig_native.h`](include/lean_multisig_native.h).

It is currently consumed by the Java 25 FFM binding. A future Go binding may consume the same ABI
through cgo, rather than reimplementing the Rust integration. This crate and header are internal:
they are not a supported or versioned general-purpose C API.

The bridge's `LMCG-v1` claim-context encoding is likewise private. Language wrappers should use it
only to pass `ClaimSigners` context to the bridge; it is not a public serialization format.
