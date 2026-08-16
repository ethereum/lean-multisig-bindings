# py-lean-multisig

Python bindings for the safer `lean_multisig_api` facade from leanVM. It keeps
the recursion topology, proving parameters, and representation choices inside
the library. Applications carry the claims and expected signer sets that
authorize each signature.

Requires Python >= 3.11. Wheels are built for Linux (x86_64, aarch64; glibc +
musl) and macOS arm64.

## Install

```bash
pip install py-lean-multisig
```

## Claims and signatures

`Claim` binds a 32-byte message to a slot. A `Signature` always knows its
claim; its serialized form deliberately omits both the claim and signer set,
which must come from the surrounding protocol when it is restored.

```python
import py_lean_multisig as lm

claim = lm.Claim(b"\x42" * 32, 5)
key = lm.SecretKey.from_seed(b"\x01" * 32, 0, 1023)
signature = key.sign(claim)

# Verify against the exact expected set—checking the proof alone is not enough.
lm.verify(signature, [key.public_key], claim)

# Supply external context when decoding the wire value.
restored = lm.Signature.from_bytes(signature.to_bytes(), claim, [key.public_key])
lm.verify(restored, [key.public_key], claim)
```

XMSS keys are stateful: never sign different messages at the same slot. Persist
your own durable high-water slot alongside `SecretKey.to_bytes()` and advance it
before publishing a signature.

## Single-claim aggregation

Call `setup()` once before aggregating, decoding an aggregate, or verifying an
aggregate. It is safe to call repeatedly.

```python
import py_lean_multisig as lm

lm.setup()
claim = lm.Claim(b"\x42" * 32, 5)
keys = [lm.SecretKey.from_seed(bytes([i]) * 32, 0, 1023) for i in (1, 2)]

aggregate = lm.aggregate([key.sign(claim) for key in keys], claim)
expected = [key.public_key for key in keys]
lm.verify(aggregate, expected, claim)
```

Raw and already aggregated `Signature` values can be mixed in `aggregate`; no
parallel public-key list or child-proof topology is exposed.

## Multi-claim proofs

`merge_claims` groups signatures by claim and produces one proof. Each expected
claim has its own explicitly authorized signer set.

```python
groups = [lm.ClaimSigners(claim, expected)]
proof = lm.merge_claims([aggregate])
lm.verify_claims(proof, groups)

restored = lm.MultiClaimProof.from_bytes(proof.to_bytes(), groups)
lm.verify_claims(restored, groups)
```

## Breaking change

This replaces the legacy `keygen`, `Prover`, `Verifier`, `PublicKey`, and
`AggregatedSignature` API. The replacement makes the claim and expected signer
set explicit, carries signer context with signatures in memory, and chooses
aggregation tuning internally.

## Development

```bash
uv run --extra dev maturin develop --release
uv run --extra dev pytest tests/ -v
uv run --extra dev python -m mypy.stubtest py_lean_multisig --allowlist stubtest_allowlist.txt
```

After Rust source changes, re-run `uv run --extra dev maturin develop --release`.

## License

MIT OR Apache-2.0
