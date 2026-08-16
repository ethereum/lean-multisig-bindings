# lean-multisig-bindings

Language bindings for [leanMultisig](https://github.com/leanEthereum/leanMultisig), providing XMSS signatures and zkVM-backed signature aggregation.

## Bindings

- [`bindings/python`](bindings/python/README.md) — the `py-lean-multisig` package for Python.
- [`bindings/java`](bindings/java/README.md) — Java 25 bindings for JVM applications such as Teku.

Each binding uses a language-native wrapper around the Rust implementation. The internal
[`bindings/native`](bindings/native/README.md) crate supplies a private C-shaped Rust ABI for
managed wrappers: Java consumes it through the finalized Foreign Function & Memory API, and a
future Go binding can consume it through cgo. It is not a separate, supported C binding; future
bindings can use whichever wrapper best fits their language ecosystem.

## License

MIT OR Apache-2.0
