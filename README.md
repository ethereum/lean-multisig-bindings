# py-lean-multisig

Python bindings for leanMultisig: XMSS signatures and zkVM-backed signature aggregation.

Requires Python >= 3.11. Wheels are built for Linux (x86_64, aarch64; glibc + musl) and macOS arm64.

## Install

```
pip install py-lean-multisig
```

## XMSS keygen / sign / verify

```python
import py_lean_multisig as lm

# Key generation (seed is 32 bytes; slot range is inclusive)
sk, pk = lm.keygen(b"\x00" * 32, 0, 1023)

# Sign and verify (message is 32 bytes)
msg = b"\x42" * 32
sig = lm.sign(sk, msg, 5)  # deterministic: randomness derives from the key seed
lm.verify(pk, msg, sig, 5)

# Serialize/deserialize
pk2 = lm.PublicKey.from_bytes(pk.to_bytes())
sig2 = lm.Signature.from_bytes(sig.to_bytes())
assert pk == pk2 and sig == sig2
```

## Aggregation

Aggregate N signatures over the same `(message, slot)` into a single proof, then verify.

```python
import py_lean_multisig as lm

msg, slot = b"\x42" * 32, 5
pairs = [lm.keygen(bytes([i+1])*32, 0, 1023) for i in range(4)]
pks  = [pk for _, pk in pairs]
sigs = [lm.sign(sk, msg, slot) for sk, _ in pairs]

prover = lm.Prover(log_inv_rate=4)
verifier = lm.Verifier()

sorted_pks, agg = prover.aggregate(pks, sigs, msg, slot)
verifier.verify(sorted_pks, msg, agg, slot)

# Round-trip through bytes
agg2 = lm.SingleMessageSignature.from_bytes(agg.to_bytes())
```

## Hierarchical aggregation

Aggregated proofs are themselves zkVM-verifiable, so a `Prover` can fold
existing aggregates into a new one via the `children=`. 

This is useful for tree-shaped aggregation (each node aggregates its sub-tree's proofs)
and for distributing proving work across machines (each shard produces
a child proof and a coordinator folds them).

```python
import py_lean_multisig as lm

msg, slot = b"\x42" * 32, 5

def _signers(seed_offset, n):
    pairs = [lm.keygen(bytes([seed_offset + i]) * 32, 0, 1023) for i in range(n)]
    pks  = [pk for _, pk in pairs]
    sigs = [lm.sign(sk, msg, slot) for sk, _ in pairs]
    return pks, sigs

prover   = lm.Prover(log_inv_rate=4)
verifier = lm.Verifier()

# Two disjoint sets of signers, aggregated independently.
pks_a, sigs_a = _signers(seed_offset=1,  n=2)
pks_b, sigs_b = _signers(seed_offset=50, n=2)
sorted_pks_a, agg_a = prover.aggregate(pks_a, sigs_a, msg, slot)
sorted_pks_b, agg_b = prover.aggregate(pks_b, sigs_b, msg, slot)

# Top level: no fresh raw signatures, just fold the two child proofs.
# `children=` takes a list of SingleMessageSignature directly — each
# child carries its own bound pubkeys via `child.pubkeys`.
sorted_pks_top, agg_top = prover.aggregate(
    [], [], msg, slot,
    children=[agg_a, agg_b],
)

verifier.verify(sorted_pks_top, msg, agg_top, slot)
```

You can also mix raw signatures with children at the same level — fold
two existing child aggregates plus a fresh batch of raw signatures
into one combined proof in a single `aggregate()` call:

```python
# Re-use sorted_pks_a / agg_a / sorted_pks_b / agg_b from above, plus
# a fresh batch of signers not already in either child:
pks_c, sigs_c = _signers(seed_offset=150, n=2)

sorted_pks_top, agg_top = prover.aggregate(
    pks_c, sigs_c, msg, slot,                       # fresh raw signatures
    children=[agg_a, agg_b],                        # plus the two children
)

# sorted_pks_top is the union: 2 from child A + 2 from child B + 2 fresh
verifier.verify(sorted_pks_top, msg, agg_top, slot)
```

## Multi-message bundles

Aggregating signatures over **different** `(message, slot)` pairs requires a
two-step path: build a `SingleMessageSignature` per `(message, slot)`, then
bundle them into a `MultiMessageSignature` (up to 16 components). Each
component keeps its own `(message, slot, pubkeys)` triple.

```python
import py_lean_multisig as lm

prover, verifier = lm.Prover(log_inv_rate=4), lm.Verifier()

# Build one SingleMessage per (message, slot); each (pks_*, sigs_*)
# batch is produced as in the aggregation examples above.
_, sm_a = prover.aggregate(pks_a, sigs_a, msg_a, slot_a)
_, sm_b = prover.aggregate(pks_b, sigs_b, msg_b, slot_b)
_, sm_c = prover.aggregate(pks_c, sigs_c, msg_c, slot_c)

# Bundle into one MultiMessage proof
multi = prover.merge([sm_a, sm_b, sm_c])
assert isinstance(multi, lm.MultiMessageSignature)
assert len(multi) == 3

# Verify: pass the (pks, msg, slot) you expect each component to be
# bound to, in component order. These must come from your own context
# (the block body, a validator registry) — reading them back off
# `multi.components` would only prove the signature is internally
# consistent, not that it says what you think it says.
verifier.verify_multi(
    [(pks_a, msg_a, slot_a), (pks_b, msg_b, slot_b), (pks_c, msg_c, slot_c)],
    multi,
)

# Recover one component as a standalone SingleMessage (1 zkVM op).
sm_b_recovered = prover.split(multi, index=1)
verifier.verify(pks_b, msg_b, sm_b_recovered, slot_b)
```

## Pubkey-free serialization

`to_bytes()` bundles the signer set into the blob. When the receiver
already knows the signers (e.g. from a validator registry), the compact
pubkey-free form drops them from the wire and the receiver supplies them
at decode time:

```python
compact = agg.to_bytes_without_pubkeys()
agg2 = lm.SingleMessageSignature.from_bytes_without_pubkeys(compact, sorted_pks)

# MultiMessage: one signer set per component, in component order.
compact_multi = multi.to_bytes_without_pubkeys()
multi2 = lm.MultiMessageSignature.from_bytes_without_pubkeys(
    compact_multi,
    [c.pubkeys for c in multi.components],
)
```

A signer set different from the one aggregated fails verification — the
binding lives in the proof, not the serialized keys.

All byte encodings are upstream's: SSZ for `PublicKey`/`Signature` and
postcard for the aggregate proofs, so blobs are interchangeable with other
leanMultisig consumers.

## Development

Uses [`uv`](https://github.com/astral-sh/uv) for venv + dependency
management

```bash
# One-time setup: create the venv, install maturin + dev deps, build
# and editable-install the extension.
uv venv
uv pip install maturin pytest mypy
uv run maturin develop --release --extras dev

# Run the test suite.
uv run pytest tests/ -v

# Verify the .pyi stubs match the runtime extension.
uv run python -m mypy.stubtest py_lean_multisig --allowlist stubtest_allowlist.txt
```

After Rust source changes, re-run `uv run maturin develop --release` to
rebuild the extension.

## License

MIT OR Apache-2.0
