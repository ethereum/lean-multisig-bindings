# Expanded Same-Claim Benchmark Sizes Design

## Goal

Measure same-claim aggregation and verification at 32, 64, 128, 256, and 512 signers without implying that Lean supports more than 16 distinct claims and without making ordinary local or CI runs unexpectedly expensive.

## Interface and limits

The slow comparison runner will expose two independent size axes. `--same-sizes` accepts positive, unique counts through 512, while `--distinct-sizes` accepts positive, unique counts through `lean_multisig::MAX_CLAIMS` (currently 16). Both default to the existing practical set `1,8,16`, so expanded Lean proof generation is opt-in. The existing `--sizes` option remains as a backward-compatible shorthand for setting both axes and consequently retains the 16-item ceiling. It cannot be combined with either specific option, avoiding ambiguous precedence.

The general comparison-report model will accept input sizes through 512. Distinct-claim safety remains enforced at the runner parser and fixture constructor, where the workload semantics are known. The JSON and human-readable table schemas do not change.

## Execution and fast benchmarks

The slow runner will process same-claim and distinct-claim lists in separate loops. Same-claim rows are emitted for every requested same size; distinct aggregation, distinct verification, and Lighthouse signature-set rows are emitted only for requested distinct sizes. Existing sampling, warm-up, calibration, ordering, and artifact-size behavior remains unchanged.

Criterion's Lighthouse same-claim aggregation and verification groups will cover `1,8,16,32,64,128,256,512`. Its distinct-claim aggregation, distinct verification, and signature-set groups remain at `1,8,16`.

## Verification

Parser tests will establish defaults, independent overrides, backward compatibility, conflict handling, the 16-claim distinct limit, and acceptance/rejection around the 512-signer same-claim limit. A fixture test will construct 512 same-claim signers and validate shape only, deliberately avoiding expensive Lean aggregation. Targeted unit tests, formatting, Clippy, release compilation, benchmark-name listing, and a small split-axis smoke run will complete verification. The full 32-to-512 Lean proof sweep remains an explicit user-run benchmark.
