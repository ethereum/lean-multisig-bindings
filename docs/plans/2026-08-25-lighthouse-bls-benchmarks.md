# Lighthouse BLS Comparison Benchmarks Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add reproducible Rust benchmarks comparing leanMultisig with Lighthouse's pinned production BLS wrapper for signing, verification, same-claim aggregation, and distinct-claim aggregation.

**Architecture:** Add an unpublished `benchmarks/comparison` workspace crate so comparison dependencies never enter the supported bindings. Put deterministic fixtures and report calculations in its library, use Criterion for fast operations, and use a configurable release-mode binary for expensive zkVM proof generation.

**Tech Stack:** Rust 2021, Cargo workspace, local `lean-multisig`, Lighthouse `bls` at commit `e423a66763bb1bd780492d635123f208d80c3538`, Criterion 0.8, Serde/serde_json, anyhow.

---

### Task 1: Scaffold the isolated comparison crate

**Files:**
- Modify: `Cargo.toml`
- Create: `benchmarks/comparison/Cargo.toml`
- Create: `benchmarks/comparison/src/lib.rs`

**Step 1: Verify the package does not exist**

Run:

```bash
cargo metadata --no-deps --format-version 1 | rg 'lean-multisig-comparison'
```

Expected: exit 1 with no match.

**Step 2: Add the workspace member and manifest**

Add `"benchmarks/comparison"` to `workspace.members` in the root manifest. Create this package manifest:

```toml
[package]
name = "lean-multisig-comparison"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
anyhow = "1"
lean-multisig = { path = "../../bindings/rust" }
lighthouse-bls = { package = "bls", git = "https://github.com/sigp/lighthouse.git", rev = "e423a66763bb1bd780492d635123f208d80c3538" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
criterion = "0.8"

[[bench]]
name = "comparison"
harness = false
```

Start `src/lib.rs` with crate-level documentation explaining that this is benchmark support, not a supported API.

**Step 3: Verify Cargo resolves the exact Lighthouse revision**

Run:

```bash
cargo metadata --format-version 1 > /tmp/lean-multisig-comparison-metadata.json
rg 'e423a66763bb1bd780492d635123f208d80c3538' Cargo.lock
```

Expected: metadata succeeds and `Cargo.lock` contains the pinned revision.

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock benchmarks/comparison/Cargo.toml benchmarks/comparison/src/lib.rs
git commit -m "build: scaffold BLS comparison benchmarks"
```

### Task 2: Build deterministic, pre-validated fixtures

**Files:**
- Modify: `benchmarks/comparison/src/lib.rs`
- Create: `benchmarks/comparison/tests/fixtures.rs`

**Step 1: Write failing fixture tests**

Create tests covering these observable contracts:

```rust
use lean_multisig_comparison::{FixtureSet, MAX_DISTINCT_CLAIMS};

#[test]
fn fixtures_have_the_requested_unique_signers() {
    let fixtures = FixtureSet::same_claim(3).unwrap();
    assert_eq!(fixtures.len(), 3);
    assert_eq!(fixtures.lean_public_keys().len(), 3);
    assert_eq!(fixtures.bls_public_keys().len(), 3);
    assert_ne!(fixtures.lean_public_keys()[0], fixtures.lean_public_keys()[1]);
    assert_ne!(
        fixtures.bls_public_keys()[0].serialize(),
        fixtures.bls_public_keys()[1].serialize()
    );
}

#[test]
fn distinct_fixtures_use_distinct_messages_and_slots() {
    let fixtures = FixtureSet::distinct_claims(3).unwrap();
    assert_eq!(fixtures.lean_claims()[0].slot(), 0);
    assert_eq!(fixtures.lean_claims()[1].slot(), 1);
    assert_ne!(fixtures.lean_claims()[0].message(), fixtures.lean_claims()[1].message());
    assert_ne!(fixtures.bls_messages()[0], fixtures.bls_messages()[1]);
}

#[test]
fn fixtures_reject_empty_and_excessive_counts() {
    assert!(FixtureSet::same_claim(0).is_err());
    assert!(FixtureSet::distinct_claims(0).is_err());
    assert!(FixtureSet::distinct_claims(MAX_DISTINCT_CLAIMS + 1).is_err());
}

#[test]
fn every_raw_fixture_is_valid_before_benchmarking() {
    FixtureSet::same_claim(3).unwrap().validate_raw().unwrap();
    FixtureSet::distinct_claims(3).unwrap().validate_raw().unwrap();
}
```

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p lean-multisig-comparison --test fixtures
```

Expected: compilation fails because `FixtureSet` and `MAX_DISTINCT_CLAIMS` do not exist.

**Step 3: Implement fixture generation**

Add:

```rust
pub const MAX_DISTINCT_CLAIMS: usize = lean_multisig::MAX_CLAIMS;

pub struct FixtureSet {
    lean_claims: Vec<lean_multisig::Claim>,
    lean_keys: Vec<lean_multisig::SecretKey>,
    lean_signatures: Vec<lean_multisig::Signature>,
    lean_public_keys: Vec<lean_multisig::PublicKey>,
    bls_messages: Vec<lighthouse_bls::Hash256>,
    bls_keys: Vec<lighthouse_bls::SecretKey>,
    bls_signatures: Vec<lighthouse_bls::Signature>,
    bls_public_keys: Vec<lighthouse_bls::PublicKey>,
}
```

Implement `same_claim(count)`, `distinct_claims(count)`, getters required by the benchmark targets, `len`, `is_empty`, and `validate_raw`.

Use Lighthouse's own deterministic test convention for BLS secrets: encode `index + 1` as big-endian in the final eight bytes and call `SecretKey::deserialize`. Generate deterministic XMSS seeds independently by encoding `index + 1` into a 32-byte seed. Use inclusive Lean slots `0..=MAX_DISTINCT_CLAIMS as u32`.

For same-claim validation, call Lean `verify` with exactly one signer and Lighthouse `Signature::verify`. For distinct claims, validate each corresponding tuple. Return `anyhow::Result` and attach signer-index context to every fallible operation.

**Step 4: Run the focused tests**

Run:

```bash
cargo test -p lean-multisig-comparison --test fixtures
```

Expected: 4 tests pass.

**Step 5: Commit**

```bash
git add benchmarks/comparison/src/lib.rs benchmarks/comparison/tests/fixtures.rs
git commit -m "test: add validated comparison fixtures"
```

### Task 3: Add report summaries and runner configuration

**Files:**
- Modify: `benchmarks/comparison/src/lib.rs`
- Create: `benchmarks/comparison/tests/report.rs`

**Step 1: Write failing summary and argument tests**

Cover:

```rust
use std::time::Duration;
use lean_multisig_comparison::{RunConfig, SampleSummary};

#[test]
fn summary_reports_samples_median_and_throughput() {
    let summary = SampleSummary::from_durations([
        Duration::from_millis(30),
        Duration::from_millis(10),
        Duration::from_millis(20),
    ]).unwrap();
    assert_eq!(summary.samples_ns, vec![30_000_000, 10_000_000, 20_000_000]);
    assert_eq!(summary.median_ns, 20_000_000);
    assert_eq!(summary.operations_per_second, 50.0);
}

#[test]
fn config_defaults_to_practical_sizes_and_three_samples() {
    let config = RunConfig::parse_from(["slow-comparison"]).unwrap();
    assert_eq!(config.samples, 3);
    assert_eq!(config.sizes, vec![1, 8, 16]);
    assert!(!config.warmup_proofs);
}

#[test]
fn config_accepts_samples_sizes_and_json_path() {
    let config = RunConfig::parse_from([
        "slow-comparison", "--samples", "2", "--sizes", "1,8", "--json", "/tmp/report.json", "--warmup-proofs"
    ]).unwrap();
    assert_eq!(config.samples, 2);
    assert_eq!(config.sizes, vec![1, 8]);
    assert_eq!(config.json_path.unwrap().to_str(), Some("/tmp/report.json"));
    assert!(config.warmup_proofs);
}

#[test]
fn config_rejects_zero_samples_and_out_of_range_sizes() {
    assert!(RunConfig::parse_from(["slow-comparison", "--samples", "0"]).is_err());
    assert!(RunConfig::parse_from(["slow-comparison", "--sizes", "0"]).is_err());
    assert!(RunConfig::parse_from(["slow-comparison", "--sizes", "17"]).is_err());
}
```

Add a JSON round-trip test for `BenchmarkReport` with one `ComparisonReport` and one BLS-only supplemental measurement.

**Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p lean-multisig-comparison --test report
```

Expected: compilation fails because the report/config types do not exist.

**Step 3: Implement the report model and parser**

Add Serde-enabled values:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SampleSummary {
    pub samples_ns: Vec<u64>,
    pub median_ns: u64,
    pub operations_per_second: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonReport {
    pub workload: String,
    pub input_size: usize,
    pub lean: SampleSummary,
    pub lighthouse: SampleSummary,
    pub lean_over_lighthouse: f64,
    pub lean_artifact_bytes: usize,
    pub lighthouse_artifact_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SupplementalReport {
    pub workload: String,
    pub input_size: usize,
    pub lighthouse: SampleSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkReport {
    pub lighthouse_revision: String,
    pub samples: usize,
    pub proof_warmup: bool,
    pub comparisons: Vec<ComparisonReport>,
    pub supplemental: Vec<SupplementalReport>,
}
```

Implement checked `Duration` to `u64` nanosecond conversion, sorted median calculation without changing recorded sample order, throughput, ratio calculation, JSON serialization, and `RunConfig::parse_from`. Parse only `--samples`, `--sizes`, `--json`, and the valueless `--warmup-proofs`; reject unknown, repeated, missing, zero, or oversized values with actionable errors. Default `warmup_proofs` to false and record the selected mode in JSON.

**Step 4: Run the focused tests**

Run:

```bash
cargo test -p lean-multisig-comparison --test report
```

Expected: all report tests pass.

**Step 5: Commit**

```bash
git add benchmarks/comparison/src/lib.rs benchmarks/comparison/tests/report.rs
git commit -m "feat: add benchmark report model"
```

### Task 4: Add Criterion fast-operation comparisons

**Files:**
- Modify: `benchmarks/comparison/benches/comparison.rs` (replace the scaffold placeholder)
- Modify: `benchmarks/comparison/src/lib.rs`

**Step 1: Expose only the fixture operations needed by benchmarks**

Add focused getters/helpers rather than making fields public. Include helpers to construct a BLS aggregate from fixture signatures and to construct borrowed `SignatureSet` values for production batch verification. Add unit assertions to `tests/fixtures.rs` showing the aggregates and signature sets verify.

**Step 2: Run the focused tests before implementation**

Run:

```bash
cargo test -p lean-multisig-comparison --test fixtures
```

Expected: compilation fails for the new helper calls.

**Step 3: Implement the helpers and fast benchmark target**

Create Criterion groups for:

- `key_creation/{lean,lighthouse}`
- `public_key/{lean,lighthouse}`
- `sign/{lean,lighthouse}`
- `raw_signature_serialize/{lean,lighthouse}`
- `raw_signature_deserialize/{lean,lighthouse}`
- `single_verify/{lean,lighthouse}`
- `lighthouse_same_claim_aggregate/{1,8,16}`
- `lighthouse_same_claim_verify/{1,8,16}`
- `lighthouse_distinct_claim_aggregate/{1,8,16}`
- `lighthouse_distinct_claim_verify/{1,8,16}`
- `lighthouse_signature_sets_verify/{1,8,16}`

Construct fixtures outside `bench_function`, pass all timed inputs and outputs through `criterion::black_box`, use `Throughput::Elements` for sized cases, and assert validation before registering each benchmark. Do not time fixture allocation, Lean `setup`, or post-operation validation.

**Step 4: Verify the benchmark compiles and run one fast filter**

Run:

```bash
cargo bench -p lean-multisig-comparison --bench comparison --no-run
cargo bench -p lean-multisig-comparison --bench comparison -- key_creation
```

Expected: the target compiles and both key-creation comparisons produce Criterion estimates.

**Step 5: Commit**

```bash
git add benchmarks/comparison/src/lib.rs benchmarks/comparison/tests/fixtures.rs benchmarks/comparison/benches/comparison.rs
git commit -m "feat: benchmark fast XMSS and BLS operations"
```

### Task 5: Implement the practical slow proof runner

**Files:**
- Modify: `benchmarks/comparison/src/lib.rs`
- Create: `benchmarks/comparison/src/bin/slow_comparison.rs`
- Modify: `benchmarks/comparison/tests/report.rs`

**Step 1: Write failing tests for paired measurement assembly**

Add tests around timing-independent assembly functions:

- `ComparisonReport::new` calculates `lean_over_lighthouse` from the two medians.
- A zero Lighthouse median is rejected rather than producing infinity.
- `BenchmarkReport::to_table()` includes workload, size, both medians, ratio, and artifact sizes.
- Supplemental rows are visibly labeled Lighthouse-only.

Run:

```bash
cargo test -p lean-multisig-comparison --test report
```

Expected: compilation fails for the new constructors/table formatter.

**Step 2: Implement the report constructors and formatter**

Use integer nanoseconds as the stored source of truth. Format human durations at an appropriate ns/us/ms/s scale only at presentation time. Keep JSON field names stable and descriptive.

**Step 3: Implement slow measurement functions**

The runner should call `lean_multisig::setup()` before fixture creation. For every configured size, measure these paired rows:

1. `same_claim_aggregate`: Lean `aggregate(signatures.clone(), claim)` versus a fresh Lighthouse `AggregateSignature::infinity()` plus `add_assign` for every signature.
2. `same_claim_verify`: Lean `verify` against the exact signer set versus Lighthouse `fast_aggregate_verify`.
3. `distinct_claim_aggregate`: Lean `merge_claims(signatures.clone())` versus Lighthouse aggregate construction.
4. `distinct_claim_verify_conceptual`: Lean `verify_claims` versus Lighthouse `aggregate_verify` with corresponding messages/public keys. This is explicitly conceptual/non-production because Lighthouse documents `aggregate_verify` as EF-test-only.

Also measure Lighthouse `verify_signature_sets` as the Lighthouse-only `lighthouse_signature_sets_verify` supplemental production-oriented batch-verification row. Build all fixtures and precomputed verification artifacts outside timed regions. Clone one consumed Lean signature vector immediately before each timer so memory remains O(input size). After every timed aggregate sample, verify its result before retaining the duration. Use `std::time::Instant` and collect exactly `RunConfig.samples` durations.

Default Lean proof measurements perform no explicit proof warm-up, so recorded samples may include process-local first-use effects. With `--warmup-proofs`, generate and verify exactly one untimed same-claim proof and one untimed distinct-claim proof per input size, then reuse them for verification fixtures. Calibrate every Lighthouse operation outside recorded samples to at least a modest batch duration (10–25 ms), record normalized per-operation durations, and require every verification iteration to succeed. Calibration serves as the cheap BLS warm-up. Alternate which implementation runs first for paired samples, starting with Lighthouse for sample zero, without sleeps or cooldowns.

Write JSON only when `--json PATH` is supplied. Print the table to stdout in all cases. Include the semantic warning, pinned Lighthouse revision, a prominent no-explicit-proof-warm-up or steady-state mode label, and a note directing readers from the conceptual EF-test row to `lighthouse_signature_sets_verify`. Share one revision constant between the runner and a test that guards agreement with the manifest pin.

**Step 4: Run non-proving tests and compile the release binary**

Run:

```bash
cargo test -p lean-multisig-comparison
cargo build --release -p lean-multisig-comparison --bin slow_comparison
```

Expected: all unit/integration tests pass and the runner compiles.

**Step 5: Run the smallest end-to-end smoke measurement**

Run:

```bash
cargo run --release -p lean-multisig-comparison --bin slow_comparison -- \
  --samples 1 --sizes 1 --json /tmp/lean-multisig-benchmark-smoke.json
cargo run --release -p lean-multisig-comparison --bin slow_comparison -- \
  --samples 1 --sizes 1 --warmup-proofs \
  --json /tmp/lean-multisig-benchmark-warm-smoke.json
```

Expected: both modes print all five rows, JSON records the correct proof-warmup mode, every proof verifies, and each process exits 0. The warm run executes exactly one additional same-claim proof and one additional distinct-claim proof. Do not treat the one-sample numbers as publishable results.

**Step 6: Commit**

```bash
git add benchmarks/comparison/src/lib.rs benchmarks/comparison/src/bin/slow_comparison.rs benchmarks/comparison/tests/report.rs
git commit -m "feat: add practical proof comparison runner"
```

### Task 6: Document commands, semantics, and reproducibility

**Files:**
- Create: `benchmarks/comparison/README.md`
- Modify: `.github/workflows/ci.yml`

**Step 1: Add a CI expectation that initially fails**

Extend the Rust job with:

```yaml
- name: Test comparison support
  run: cargo test -p lean-multisig-comparison

- name: Check comparison targets
  run: cargo check -p lean-multisig-comparison --all-targets
```

Run the exact commands locally before writing documentation. Any failure indicates the benchmark is not yet maintainable and must be fixed first.

**Step 2: Write the benchmark README**

Document:

```bash
cargo bench -p lean-multisig-comparison --bench comparison
cargo run --release -p lean-multisig-comparison --bin slow_comparison
cargo run --release -p lean-multisig-comparison --bin slow_comparison -- \
  --samples 1 --sizes 1,8 --json /tmp/lean-multisig-comparison.json
```

Explain default sizes and samples, expected runtime, all excluded setup/fixture costs, artifact sizes, exact Lighthouse revision/backend, `aggregate_verify`'s EF-test status, the production `verify_signature_sets` row, and the zkVM-versus-point-addition semantic caveat. Include a publication checklist for CPU model, core count, RAM, OS, Rust version, power mode, and whether the machine was otherwise idle.

Do not edit the root `README.md`; it has a pre-existing user modification in the primary worktree.

**Step 3: Verify docs commands and CI commands**

Run:

```bash
cargo test -p lean-multisig-comparison
cargo check -p lean-multisig-comparison --all-targets
```

Expected: both pass without running expensive proof benchmarks.

**Step 4: Commit**

```bash
git add benchmarks/comparison/README.md .github/workflows/ci.yml
git commit -m "docs: document Lighthouse BLS benchmarks"
```

### Task 7: Final verification and review

**Files:**
- Verify all files changed by Tasks 1-6

**Step 1: Format and lint**

Run:

```bash
cargo fmt --all --check
cargo clippy -p lean-multisig-comparison --all-targets -- -D warnings
```

Expected: both exit 0 with no warnings.

**Step 2: Run focused and workspace tests**

Run:

```bash
cargo test -p lean-multisig-comparison
cargo test --workspace
```

Expected: all tests and doc-tests pass; no expensive comparison runner executes.

**Step 3: Compile every benchmark target**

Run:

```bash
cargo bench -p lean-multisig-comparison --bench comparison --no-run
cargo build --release -p lean-multisig-comparison --bin slow_comparison
```

Expected: both release targets compile.

**Step 4: Inspect the final diff and worktree state**

Run:

```bash
git diff main...HEAD --check
git diff main...HEAD --stat
git status --short
```

Expected: no whitespace errors and a clean worktree.

**Step 5: Request code review**

Use `superpowers:requesting-code-review`, address every actionable finding using `superpowers:receiving-code-review`, and repeat the focused verification after any edits.

**Step 6: Commit any review fixes**

```bash
git add <reviewed-files>
git commit -m "fix: address benchmark review"
```

Skip this commit if review requires no changes.

**Step 7: Finish the development branch**

Use `superpowers:finishing-a-development-branch` and present the integration choices. Preserve the user's uncommitted root `README.md` change throughout integration.
