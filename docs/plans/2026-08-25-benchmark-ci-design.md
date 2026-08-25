# Benchmark CI and Pull Request Reporting Design

## Goal and policy

Run the Lighthouse comparison benchmarks in GitHub Actions, make results visible on pull requests, and retain enough structured output to establish historical trends without treating shared-runner timing noise as a correctness failure.

Benchmark execution, signature/proof validation, report conversion, or artifact generation failures will fail CI. Performance changes are advisory: the reporting action will flag large changes in job summaries but `fail-on-alert` remains disabled. This reflects the benchmark action's documented 10–20% variance on hosted runners. No PR comments, external benchmark service, self-hosted runner, or performance merge gate are introduced.

All new actions are pinned to immutable commit SHAs. PR jobs use `pull_request` with `contents: read`; they do not use secrets, write permissions, `pull_request_target`, or `workflow_run`. The expensive proof suite runs on a PR only when a maintainer applies `benchmark-slow`.

## Report flow

Criterion stdout remains the fast-suite input to `github-action-benchmark`'s Cargo parser. The slow runner gains an independent `--action-json PATH` output using the action's `customSmallerIsBetter` schema. Each comparison contributes Lean median time, Lighthouse median time, their ratio, and both artifact sizes. Each supplemental Lighthouse measurement contributes its median time. Names contain workload, implementation/metric, and input size; `extra` records sample count, proof-warmup mode, and pinned Lighthouse revision. The existing full report JSON remains unchanged.

PR jobs upload raw Criterion output, Criterion estimate files, slow full/action JSON, and an environment sidecar. They then render a job summary. If benchmark history is not enabled, the reporting action skips the missing `gh-pages` fetch and displays current results without a baseline.

## Workflows and history

`.github/workflows/benchmarks-pr.yml` runs the fast suite for relevant changes. Its slow job is label-gated, uses three samples, default size axes `1,8,16`, and `--warmup-proofs`. Concurrent runs for the same PR cancel older work. Fast and slow jobs have 30-minute and 180-minute timeouts, with artifacts retained for 14 days.

`.github/workflows/benchmarks-history.yml` runs fast benchmarks on relevant pushes to `main`, slow steady-state benchmarks weekly, and either suite through `workflow_dispatch`. Measurement jobs are read-only and upload results. A separate publication job has `contents: write` and `deployments: write`, downloads numeric artifacts, and never executes repository code. It serially publishes fast and slow histories to separate `gh-pages` paths with at most 100 chart points.

History publication is gated by a repository variable, `BENCHMARK_HISTORY_ENABLED=true`, so the workflows work before Pages setup. Enabling it requires creating an orphan `gh-pages` branch and configuring GitHub Pages to serve it. PR jobs may then read the stored baseline, but they never update it. Main artifacts are retained for 30 days and scheduled slow artifacts for 90 days.

The environment sidecar records the commit, runner OS/image, CPU, kernel, `rustc -Vv`, Cargo version, Lighthouse revision, sample count, size axes, and proof-warmup mode. The comparison README documents local equivalents and the one-time history setup.

## Verification

Rust tests cover action-entry serialization, exact names/units/values/context, empty-report rejection, CLI parsing, duplicate paths, and preservation of the existing full JSON. Workflow validation checks YAML/action syntax, immutable action pins, read-only PR permissions, label gating, advisory thresholds, trusted publication separation, artifact retention, timeouts, and absence of privileged PR triggers. Local verification includes package tests, formatting, Clippy, release builds, a one-sample action-JSON smoke run, and workflow linting.
