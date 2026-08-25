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
the independent `customSmallerIsBetter` input used by benchmark CI; the two
output paths must resolve to different files. Fixture setup, post-timing
correctness checks, and the Lean input clone performed immediately before
aggregation are outside the timed region. Lighthouse operations are calibrated
in batches of at least 10 ms and reported per operation.

The local equivalent of the opt-in slow PR job is:

```bash
cargo run --release -p lean-multisig-comparison --bin slow_comparison -- \
  --samples 3 \
  --same-sizes 1,8,16 \
  --distinct-sizes 1,8,16 \
  --warmup-proofs \
  --json full.json \
  --action-json action.json
```

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

Relevant pull requests run the fast suite and place its current Bencher-format
measurements in the job summary. Applying the `benchmark-slow` label also runs
three warmed-up samples at sizes `1,8,16` and places the comparison table in
the job summary. Applying unrelated labels does not rerun either suite. Both
jobs retain their raw output, numeric reports, Criterion estimates where
applicable, and environment metadata as workflow artifacts for 14 days.

These shared-runner measurements are informational. They do not use an
automated performance threshold and do not gate merging. Historical comparison
is added to the summary only after maintainers explicitly enable benchmark
history; the pull request workflow never writes benchmark history.

Benchmark execution, proof/signature validation, report conversion, and
artifact failures still fail their jobs. Each environment sidecar records the
commit, runner OS and architecture, runner image name and version, CPU, kernel,
Rust and Cargo versions, the manifest-derived Lighthouse revision, suite,
sample count, size axes, and proof-warmup mode.

## Trusted benchmark history

The history workflow has three modes:

- Relevant pushes to `main` run and retain the fast suite for 30 days.
- A weekly Monday 03:17 UTC schedule runs the slow suite with three samples,
  sizes `1,8,16`, and proof warm-up; its artifacts are retained for 90 days.
- `workflow_dispatch` accepts `fast`, `slow`, or `all`, plus the slow sample
  count (minimum 3) and independent same/distinct size lists. Manual slow runs
  always use proof warm-up. One-sample smoke runs remain local-only and cannot
  be published by the history workflow.

For example, this explicitly opts into the large same-claim sweep while keeping
distinct claims within their hard limit of 16:

```bash
gh workflow run benchmarks-history.yml \
  -f suite=slow \
  -f samples=3 \
  -f same_sizes=32,64,128,256,512 \
  -f distinct_sizes=1,8,16
```

That run can take substantial time and memory. It is never part of a push,
pull-request, or scheduled default.

Measurement jobs are read-only. A separate publication job downloads only
successful numeric artifacts and publishes fast and slow histories sequentially
to `dev/bench/fast` and `dev/bench/slow`. It performs a shallow checkout of the
trusted workflow revision, without persisting checkout credentials, solely to
initialize the Git state required by the publisher. It never executes repository
code. Publication is disabled unless the repository variable
`BENCHMARK_HISTORY_ENABLED` is exactly `true`.

### One-time history setup

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
git commit -m 'chore: initialize benchmark history'
git push origin gh-pages
```

In repository settings, configure GitHub Pages to deploy from the `gh-pages`
branch and its root directory. Ensure GitHub Actions may use the workflow's
declared write permissions. Only after that setup, enable publication:

```bash
gh variable set BENCHMARK_HISTORY_ENABLED --body true
```

The trusted history workflow may then update `gh-pages`, while PR workflows
retain `contents: read`, receive no secrets, and never write history. Leave the
variable unset or set it to any value other than `true` to keep history
publication disabled without disabling measurement artifacts and current-result
summaries.
