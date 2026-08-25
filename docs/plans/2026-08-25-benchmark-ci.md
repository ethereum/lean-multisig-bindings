# Benchmark CI and Pull Request Reporting Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Run fast and opt-in slow comparison benchmarks in GitHub Actions, show advisory PR summaries, and optionally publish trusted history.

**Architecture:** Add a tested GitHub-action JSON adapter to the slow runner, then add separate unprivileged PR measurement and trusted main/history workflows. Keep full benchmark JSON stable and gate Pages publication behind explicit repository setup.

**Tech Stack:** Rust 2021, Serde, GitHub Actions, Criterion, `benchmark-action/github-action-benchmark`, Actions artifacts, actionlint.

---

### Task 1: Add action-compatible slow benchmark JSON

**Files:**
- Modify: `benchmarks/comparison/tests/report.rs`
- Modify: `benchmarks/comparison/src/lib.rs`
- Modify: `benchmarks/comparison/src/bin/slow_comparison.rs`

**Step 1: Write failing report-conversion tests**

Define the wished-for `BenchmarkReport::to_action_benchmarks()` API. Test exact entries for a paired row and supplemental row: names, `ns`/`ratio`/`bytes` units, median/ratio/size values, and context containing samples, warm-up mode, and Lighthouse revision. Test that an empty report is rejected.

Add parser tests for `--action-json`, repeated flags, missing/empty values, and rejection when `--json` and `--action-json` resolve to the same path.

**Step 2: Verify RED**

Run `cargo test -p lean-multisig-comparison --test report`.

Expected: compilation fails because the action output API/config field does not exist.

**Step 3: Implement the action schema and CLI**

Add a Serde-serializable entry with `name`, `unit`, numeric `value`, and `extra`. Convert each comparison into five smaller-is-better entries and each supplemental row into one time entry. Use stable slash-separated names. Add `action_json_path` to `RunConfig`, parse it like `--json`, list it in unknown-option errors, and reject equal output paths.

After building the report, serialize/write the existing full JSON and action JSON independently with path-specific context.

**Step 4: Verify GREEN**

Run focused and package tests. Then perform a release size-1 smoke run writing both files and validate both with `serde_json`/`jq`.

Expected: all tests pass; action JSON is a nonempty array accepted by its schema; full JSON is unchanged.

**Step 5: Commit**

Commit as `feat: export CI benchmark results`.

### Task 2: Add the pull request benchmark workflow

**Files:**
- Create: `.github/workflows/benchmarks-pr.yml`
- Create: `scripts/check-benchmark-ci.sh`

**Step 1: Write a failing workflow policy check**

The check script must fail while the workflow is absent, then validate that all `uses:` references are full SHA pins, the workflow uses `pull_request` but not `pull_request_target`/`workflow_run`, top-level permissions are read-only, the slow job checks for `benchmark-slow`, alerts do not fail CI, and expected timeouts/retention are present.

Run `sh scripts/check-benchmark-ci.sh`.

Expected: failure because the PR workflow is absent.

**Step 2: Implement the PR workflow**

Use relevant path filters and per-PR cancellation. On `ubuntu-24.04` with Rust 1.94.0, run the fast Criterion suite for every relevant PR and capture combined stdout/stderr. Run the three-sample default-size slow runner with `--warmup-proofs`, full JSON, and action JSON only when the PR has `benchmark-slow`.

Write an environment sidecar, upload artifacts before reporting, and call the pinned benchmark action with `summary-always: true`, `save-data-file: false`, advisory 150% fast/200% slow thresholds, and `fail-on-alert: false`. Skip history fetch unless `BENCHMARK_HISTORY_ENABLED` is true.

**Step 3: Verify GREEN**

Run the policy script and actionlint. Expected: both pass.

**Step 4: Commit**

Commit as `ci: report benchmarks on pull requests`.

### Task 3: Add trusted main and scheduled history

**Files:**
- Create: `.github/workflows/benchmarks-history.yml`
- Modify: `scripts/check-benchmark-ci.sh`

**Step 1: Extend the policy check and verify RED**

Require push-to-main, weekly schedule, manual suite/sample/size inputs, read-only measurement permissions, separate write-enabled publication, shared non-cancelling concurrency, 30/90-day retention, distinct fast/slow history paths, and publication gating on `BENCHMARK_HISTORY_ENABLED`.

Run the policy check. Expected: failure because the history workflow is absent.

**Step 2: Implement the history workflow**

Run fast on relevant main pushes and fast/slow/all on manual requests. Run slow weekly with steady-state proofs. Upload results from read-only measurement jobs. In one serialized publication job, conditionally download successful artifacts and invoke the pinned reporting action sequentially with `auto-push: true` for `dev/bench/fast` and `dev/bench/slow`. The publication job must not run Cargo or repository scripts.

**Step 3: Verify GREEN**

Run the policy check and actionlint. Expected: both pass.

**Step 4: Commit**

Commit as `ci: retain benchmark history`.

### Task 4: Document local and CI operation

**Files:**
- Modify: `benchmarks/comparison/README.md`

**Step 1: Document workflows and setup**

Explain PR fast runs, the `benchmark-slow` label, advisory results, artifacts, weekly/manual modes, expanded same-claim manual inputs, and environment metadata. Document creation of an orphan `gh-pages` branch, Pages configuration, and `BENCHMARK_HISTORY_ENABLED=true`. State that PR workflows never write history.

**Step 2: Run final verification**

Run:

```bash
sh scripts/check-benchmark-ci.sh
actionlint .github/workflows/benchmarks-pr.yml .github/workflows/benchmarks-history.yml
cargo fmt --all --check
cargo test -p lean-multisig-comparison
cargo clippy -p lean-multisig-comparison --all-targets -- -D warnings
cargo bench -p lean-multisig-comparison --bench comparison --no-run
cargo build --release -p lean-multisig-comparison --bin slow_comparison
git diff --check
```

Expected: all checks pass with the intentional 512-fixture test ignored by default.

**Step 3: Request review and commit**

Request independent spec and quality reviews, address findings, rerun the gate, and commit documentation/review fixes as needed.
