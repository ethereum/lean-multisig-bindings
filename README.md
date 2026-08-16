# lean-multisig-bindings

Language bindings for [leanMultisig](https://github.com/leanEthereum/leanMultisig), providing XMSS signatures and zkVM-backed signature aggregation.

## Bindings

- [`bindings/python`](bindings/python/README.md) — the `py-lean-multisig` package for Python.

Each binding uses a language-native wrapper around the Rust implementation. A language-neutral C ABI is not required: future bindings can live alongside Python and use the wrapper that best fits their language ecosystem.

## License

MIT OR Apache-2.0
