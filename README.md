# lean-multisig-bindings

Language bindings for [leanMultisig](https://github.com/leanEthereum/leanMultisig), providing XMSS signatures and zkVM-backed signature aggregation.

## Bindings

- [`bindings/python`](bindings/python/README.md) — the `py-lean-multisig` package for Python.
- [`bindings/rust`](bindings/rust/README.md) — the `lean-multisig` crate for Rust.
- [`bindings/java`](bindings/java/README.md) — Java 25 bindings for JVM applications such as Teku.
- [`bindings/go`](bindings/go/README.md) — Go bindings linked through cgo.
- [`bindings/node`](bindings/node/README.md) — Node.js bindings built with napi-rs.

Each binding uses a language-native wrapper around the Rust implementation. The internal
[`bindings/native`](bindings/native/README.md) crate supplies a private C-shaped Rust ABI for
managed wrappers: Java consumes it through the finalized Foreign Function & Memory API, and Go
consumes its static archive through cgo. It is not a separate, supported C binding; future bindings
can use whichever wrapper best fits their language ecosystem.

## License

MIT OR Apache-2.0
