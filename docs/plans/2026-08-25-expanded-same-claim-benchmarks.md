# Expanded Same-Claim Benchmark Sizes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add opt-in same-claim comparisons through 512 signers while preserving the 16-distinct-claim limit.

**Architecture:** Split slow-runner configuration into same-claim and distinct-claim size lists, retaining `--sizes` as a mutually exclusive compatibility shorthand. Use separate Criterion size constants so only same-claim BLS groups expand. Keep report serialization unchanged.

**Tech Stack:** Rust 2021, anyhow, Criterion 0.8, Cargo tests and benchmarks.

---

### Task 1: Split runner size configuration

**Files:**
- Modify: `benchmarks/comparison/tests/report.rs`
- Modify: `benchmarks/comparison/src/lib.rs`

**Step 1: Write failing configuration tests**

Change default assertions to `same_sizes == [1, 8, 16]` and `distinct_sizes == [1, 8, 16]`. Add tests proving:

```rust
let config = RunConfig::parse_from([
    "slow-comparison",
    "--same-sizes", "32,64,128,256,512",
    "--distinct-sizes", "1,8,16",
]).unwrap();
assert_eq!(config.same_sizes, vec![32, 64, 128, 256, 512]);
assert_eq!(config.distinct_sizes, vec![1, 8, 16]);
```

Also assert that same size 513 fails, distinct size 17 fails, `--sizes 1,8` sets both lists, and `--sizes` conflicts with either specific option.

**Step 2: Verify RED**

Run `cargo test -p lean-multisig-comparison --test report`.

Expected: compilation failures for the missing fields/options.

**Step 3: Implement minimal parsing and report limits**

Add `MAX_SAME_CLAIM_SIGNERS: usize = 512`. Replace `RunConfig::sizes` with `same_sizes` and `distinct_sizes`. Parse each option with a shared helper that takes the option name and maximum, and enforce mutual exclusivity. Widen `ComparisonReport`'s general input limit to `MAX_SAME_CLAIM_SIGNERS`; keep distinct-specific enforcement in `FixtureSet::distinct_claims` and the distinct CLI parser.

**Step 4: Verify GREEN**

Run `cargo test -p lean-multisig-comparison --test report`.

Expected: all report tests pass.

### Task 2: Split slow-runner workload loops

**Files:**
- Modify: `benchmarks/comparison/src/bin/slow_comparison.rs`

**Step 1: Use the compiler as the failing integration check**

After Task 1, run `cargo check -p lean-multisig-comparison --bin slow_comparison`.

Expected: failure because `config.sizes` no longer exists.

**Step 2: Implement independent loops**

Allocate capacities from both list lengths. Run same aggregation and verification only inside `for &size in &config.same_sizes`. Run distinct aggregation, distinct verification, and signature-set verification only inside `for &size in &config.distinct_sizes`. Do not alter measurement functions.

**Step 3: Verify GREEN**

Run `cargo check -p lean-multisig-comparison --bin slow_comparison` and the package test suite.

Expected: both pass.

### Task 3: Expand fast same-claim coverage and fixture scale

**Files:**
- Modify: `benchmarks/comparison/tests/fixtures.rs`
- Modify: `benchmarks/comparison/benches/comparison.rs`

**Step 1: Write a failing exported-limit fixture test**

Import `MAX_SAME_CLAIM_SIGNERS`, construct `FixtureSet::same_claim(MAX_SAME_CLAIM_SIGNERS)`, and assert all signer/signature collections have length 512. Do not aggregate or prove.

**Step 2: Verify RED**

Run `cargo test -p lean-multisig-comparison --test fixtures`.

Expected: compilation fails because the constant is not yet available to the test at its initial RED point.

**Step 3: Split Criterion constants**

Use:

```rust
const SAME_CLAIM_SIZES: [usize; 8] = [1, 8, 16, 32, 64, 128, 256, 512];
const DISTINCT_CLAIM_SIZES: [usize; 3] = [1, 8, 16];
```

Route same-claim groups to the first constant and all distinct/signature-set groups to the second.

**Step 4: Verify GREEN**

Run the fixture test and `cargo bench -p lean-multisig-comparison --bench comparison -- --list`.

Expected: tests pass and same-claim groups list all eight sizes while distinct groups list only three.

### Task 4: Document and verify local use

**Files:**
- Modify: `benchmarks/comparison/README.md`

**Step 1: Document the opt-in command**

Add the split options, their limits/defaults, compatibility behavior, and this example:

```bash
cargo run --release -p lean-multisig-comparison --bin slow_comparison -- \
  --samples 3 \
  --same-sizes 32,64,128,256,512 \
  --distinct-sizes 1,8,16
```

Warn that the large Lean proof sweep may take substantial time.

**Step 2: Run final verification**

Run:

```bash
cargo fmt --all --check
cargo test -p lean-multisig-comparison
cargo clippy -p lean-multisig-comparison --all-targets -- -D warnings
cargo bench -p lean-multisig-comparison --bench comparison --no-run
cargo run --release -p lean-multisig-comparison --bin slow_comparison -- \
  --samples 1 --same-sizes 1 --distinct-sizes 1
git diff --check
```

Expected: all commands pass; the smoke run prints two same rows, two distinct rows, and one Lighthouse-only signature-set row.

**Step 3: Commit**

Commit the implementation and docs with `feat: expand same-claim benchmark sizes`.
