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
report alongside the table printed to stdout. Add `--action-json PATH` to save
an independent `customSmallerIsBetter` export for optional external consumers;
the two output paths must resolve to different files. The current CI dashboard
does not use this export. Fixture setup, post-timing correctness checks, and the
Lean input clone performed immediately before aggregation are outside the timed
region. Lighthouse operations are calibrated in batches of at least 10 ms and
reported per operation.

The local equivalent of the slow PR job is:

```bash
cargo build --release -p lean-multisig-comparison --bin slow_comparison
/usr/bin/time -v -o resource-usage.txt target/release/slow_comparison \
  --samples 3 \
  --same-sizes 1,8,16 \
  --distinct-sizes 1,8,16 \
  --warmup-proofs \
  --json full.json
```

`resource-usage.txt` contains the process-wide peak resident set size for the
complete slow suite, not a per-workload attribution. CI normalizes this value to
bytes together with `full.json` for the dashboard. GNU `/usr/bin/time` reports
RSS in KiB; normalization rejects missing, duplicate, malformed, zero, or
unrepresentable measurements.

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

## Pull request benchmark CI

Relevant pull requests run the fast suite and three warmed-up slow-suite samples
at sizes `1,8,16` in parallel. Both place their current measurements in the job
summary, and the normalized slow artifact includes overall peak RSS. Both jobs
retain their raw output, normalized data, and environment metadata as workflow
artifacts for 14 days. Fast artifacts also contain Criterion estimates; slow
artifacts contain the full numeric report and GNU time resource report.

These shared-runner measurements are informational. They do not use an
automated performance threshold and do not gate merging. The pull request
workflow has read-only repository permissions and never updates the public
dashboard.

Benchmark execution, proof/signature validation, report conversion, and
artifact failures still fail their jobs. Each environment sidecar records the
commit, runner OS and architecture, runner image name and version, CPU, kernel,
Rust and Cargo versions, the manifest-derived Lighthouse revision, suite,
sample count, size axes, and proof-warmup mode.

## Current-results dashboard

The trusted benchmark workflow has three modes:

- Relevant pushes to `main` run both suites in parallel. The slow suite uses
  three samples, sizes `1,8,16`, and proof warm-up. Fast artifacts are retained
  for 30 days and slow artifacts for 90 days.
- `workflow_dispatch` accepts `fast`, `slow`, or `all`, plus the slow sample
  count (minimum 3) and independent same/distinct size lists. Manual slow runs
  always use proof warm-up. One-sample smoke runs remain local-only and cannot
  be published by the workflow.

For example, this explicitly opts into the large same-claim sweep while keeping
distinct claims within their hard limit of 16:

```bash
gh workflow run benchmarks-history.yml \
  -f suite=slow \
  -f samples=3 \
  -f same_sizes=32,64,128,256,512 \
  -f distinct_sizes=1,8,16
```

That run can take substantial time and memory. Sizes above 16 are never part of
a push or pull-request default.

Each successful measurement produces a validated `dashboard.json`. A separate
publication job downloads only successful artifacts, checks out `gh-pages`, and
replaces `data/fast.json` and/or `data/slow.json`. A fast-only run preserves the
last slow result and vice versa. It also installs the static dashboard as the
branch-root `index.html`; it does not execute measured repository code.

The public page is <https://ethereum.github.io/lean-multisig-bindings/>. It shows
only the newest result for each suite: LeanVM and Lighthouse timings, throughput,
and ratios grouped by operation family; LeanVM proof and BLS signature sizes;
overall slow-suite peak RSS; and the measurement environment. A short glossary
defines the aggregation and verification workloads. There are no
commit-over-commit plots. The first publication removes the old nested
`dev/bench` pages.

### One-time Pages setup

Create an orphan `gh-pages` branch. To avoid disturbing a working checkout, do
this in a temporary clone (set `REPOSITORY_URL` first):

```bash
benchmark_pages_dir=$(mktemp -d)
git clone "$REPOSITORY_URL" "$benchmark_pages_dir/repo"
cd "$benchmark_pages_dir/repo"
git switch --orphan gh-pages
git rm -rf .
touch .nojekyll
git add .nojekyll
git commit -m 'chore: initialize benchmark dashboard'
git push origin gh-pages
```

In repository settings, configure GitHub Pages to deploy from the `gh-pages`
branch and its root directory. Ensure GitHub Actions may use the workflow's
declared write permission. Trusted `main` pushes and manual runs may then update
`gh-pages`; PR workflows retain `contents: read` and receive no secrets.
