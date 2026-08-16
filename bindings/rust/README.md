# lean-multisig

Safe Rust bindings for [leanMultisig](https://github.com/leanEthereum/leanMultisig): XMSS
signatures and zkVM-backed signature aggregation.

This crate deliberately re-exports the shared `lean_multisig_api` facade unchanged. It keeps
the recursion topology, proving parameters, and proof representation private while making claims
and expected signer sets explicit.

## Install

Until `lean_multisig_api` is published to crates.io, depend on this crate directly from this
repository:

```toml
[dependencies]
lean-multisig = { git = "https://github.com/ethereum/lean-multisig-bindings.git" }
```

The crate is intentionally not independently published yet: it re-exports the pinned shared API
crate from leanVM. Releasing it to crates.io requires releasing that API first.

## Sign and verify

```rust
use lean_multisig::{Claim, SecretKey, verify};

let claim = Claim::new([0x42; 32], 5);
let key = SecretKey::from_seed([0x01; 32], 0..=1023)?;
let signature = key.sign(&claim)?;

verify(&signature, &[key.public_key()], &claim)?;
# Ok::<(), lean_multisig::Error>(())
```

XMSS keys are stateful: never sign different messages at the same slot. Persist your own durable
high-water slot alongside `SecretKey::to_bytes()` and advance it before publishing a signature.

## Aggregation

Call `setup()` before aggregating, decoding an aggregate, or verifying an aggregate. It is safe to
call repeatedly.

```rust
use lean_multisig::{Claim, SecretKey, aggregate, setup, verify};

setup();
let claim = Claim::new([0x42; 32], 5);
let keys = [
    SecretKey::from_seed([1; 32], 0..=1023)?,
    SecretKey::from_seed([2; 32], 0..=1023)?,
];
let aggregate = aggregate(
    keys.iter()
        .map(|key| key.sign(&claim))
        .collect::<Result<_, _>>()?,
    &claim,
)?;
let expected = keys.map(|key| key.public_key());
verify(&aggregate, &expected, &claim)?;
# Ok::<(), lean_multisig::Error>(())
```

## License

MIT OR Apache-2.0
