# lean-multisig-bindings

Language bindings for [leanMultisig](https://github.com/leanEthereum/leanMultisig), providing XMSS signatures and zkVM-backed signature aggregation.

## Bindings

- [`bindings/python`](bindings/python/README.md) — the `py-lean-multisig` package for Python.
- [`bindings/java`](bindings/java/README.md) — Java 25 bindings for JVM applications such as Teku.

Each binding uses a language-native wrapper around the Rust implementation. The Java binding has a
private C-shaped Rust ABI solely for Java's finalized Foreign Function & Memory API; it is not a
separate, supported C binding. Future bindings can use whichever wrapper best fits their language
ecosystem.

## License

MIT OR Apache-2.0
