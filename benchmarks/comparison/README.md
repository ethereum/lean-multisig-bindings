# leanMultisig and Lighthouse BLS benchmarks

This crate compares the Rust `lean-multisig` binding with Lighthouse's `bls`
wrapper pinned to revision
`e423a66763bb1bd780492d635123f208d80c3538` and its default Supranational
backend.

These are performance comparisons, not claims of equivalent security
semantics. leanMultisig aggregation generates zkVM proofs, whereas BLS
aggregation combines elliptic-curve points.

## Fast Lighthouse and single-operation benchmarks

Run the Criterion suite in release mode:

```bash
cargo bench -p lean-multisig-comparison --bench comparison
```

The Lighthouse same-claim aggregate and verification groups cover
`1,8,16,32,64,128,256,512` signers. Distinct-claim aggregation,
distinct-claim verification, and signature-set verification remain capped at
`1,8,16` claims.

## Slow paired comparison

The practical default runs three samples at sizes `1,8,16` on both axes:

```bash
cargo run --release -p lean-multisig-comparison --bin slow_comparison
```

Use independent options to opt into larger same-claim proofs:

```bash
cargo run --release -p lean-multisig-comparison --bin slow_comparison -- \
  --samples 3 \
  --same-sizes 32,64,128,256,512 \
  --distinct-sizes 1,8,16
```

The large Lean proof sweep can take substantial time and memory. It is never
part of the default run. `--same-sizes` accepts unique positive sizes through
512; `--distinct-sizes` accepts unique positive sizes through 16. Each option
defaults independently to `1,8,16`. The backward-compatible `--sizes` option
sets both axes, remains capped at 16, and cannot be combined with either
specific size option.

Add `--warmup-proofs` for steady-state proof measurements. Without it, no
explicit Lean proof warm-up runs; recorded samples may include process-local
first-use effects. Add `--json PATH` to save the unchanged machine-readable
report alongside the table printed to stdout. Fixture setup, post-timing
correctness checks, and the Lean input clone performed immediately before
aggregation are outside the timed region. Lighthouse operations are calibrated
in batches of at least 10 ms and reported per operation.

`distinct_claim_verify_conceptual` uses Lighthouse `aggregate_verify`, which
Lighthouse documents as an EF-test-only, non-production path. Use the
Lighthouse-only `lighthouse_signature_sets_verify` supplemental row for its
production-oriented batch-verification measurement.

The 512-signer fixture-shape regression validates every raw XMSS signature and
takes minutes, so ordinary `cargo test` skips it. Run it explicitly when
changing fixture construction:

```bash
cargo test -p lean-multisig-comparison --test fixtures \
  same_claim_fixtures_support_the_expanded_signer_limit -- --ignored --exact
```

When publishing results, include the CPU, operating system, Rust compiler, run
mode, sample count, and whether proof warm-up was enabled.
