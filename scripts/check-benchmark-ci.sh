#!/bin/sh
set -eu

pr_workflow=.github/workflows/benchmarks-pr.yml
history_workflow=.github/workflows/benchmarks-history.yml
ci_workflow=.github/workflows/ci.yml

fail() {
  echo "benchmark CI policy: $*" >&2
  exit 1
}

require_file() {
  [ -f "$1" ] || fail "missing $1"
}

require_text() {
  file=$1
  pattern=$2
  description=$3
  grep -Eq -- "$pattern" "$file" || fail "$file: missing $description"
}

require_literal() {
  file=$1
  literal=$2
  description=$3
  grep -Fq -- "$literal" "$file" || fail "$file: missing $description"
}

reject_text() {
  file=$1
  pattern=$2
  description=$3
  if grep -Eq -- "$pattern" "$file"; then
    fail "$file: contains forbidden $description"
  fi
}

check_full_action_pins() {
  file=$1
  uses_lines=$(grep -E '^[[:space:]]*-?[[:space:]]*uses:' "$file" || true)
  [ -n "$uses_lines" ] || fail "$file: contains no actions"
  unpinned=$(printf '%s\n' "$uses_lines" | grep -Ev '@[0-9a-f]{40}([[:space:]]*(#.*)?)?$' || true)
  [ -z "$unpinned" ] || fail "$file: every uses reference must have a full immutable SHA pin: $unpinned"
}

require_file "$pr_workflow"
require_text "$pr_workflow" '^  pull_request:$' 'pull_request trigger'
reject_text "$pr_workflow" 'pull_request_target|workflow_run' 'privileged PR trigger'
reject_text "$pr_workflow" 'write-all|:[[:space:]]*write([[:space:]]|$)' 'write permission'
reject_text "$pr_workflow" '(^|[^[:alnum:]_])secrets([^[:alnum:]_]|$)' 'standalone secrets token'
reject_text "$pr_workflow" 'continue-on-error:' 'suppressed action failure'
require_text "$pr_workflow" '^permissions:$' 'top-level permissions block'
require_text "$pr_workflow" '^  contents: read$' 'read-only contents permission'
require_text "$pr_workflow" 'types: \[opened, synchronize, reopened, labeled\]' 'label-aware pull request event types'
require_text "$pr_workflow" "if: github\.event\.action != 'labeled' \|\| github\.event\.label\.name == 'benchmark-slow'" 'fast-job unrelated-label filter'
require_text "$pr_workflow" 'benchmark-slow' 'benchmark-slow label gate'
require_text "$pr_workflow" "if: contains\(github\.event\.pull_request\.labels\.\*\.name, 'benchmark-slow'\) && \(github\.event\.action != 'labeled' \|\| github\.event\.label\.name == 'benchmark-slow'\)" 'slow-job unrelated-label filter'
require_text "$pr_workflow" 'timeout-minutes: 30' '30-minute fast timeout'
require_text "$pr_workflow" 'timeout-minutes: 180' '180-minute slow timeout'
require_text "$pr_workflow" 'retention-days: 14' '14-day artifact retention'
require_text "$pr_workflow" 'fail-on-alert: false' 'disabled performance failure gate'
require_text "$pr_workflow" 'summary-always: true' 'always-on job summary'
require_text "$pr_workflow" 'save-data-file: false' 'read-only benchmark history setting'
require_text "$pr_workflow" '--output-format bencher' 'Criterion output compatible with the Cargo parser'
require_text "$pr_workflow" 'criterion-output\.txt.*GITHUB_STEP_SUMMARY|cat benchmark-artifacts/fast/criterion-output\.txt' 'direct fast job summary'
require_text "$pr_workflow" 'sed .*benchmark-artifacts/slow/slow-output\.txt' 'direct slow job summary'
require_text "$pr_workflow" "if: vars\.BENCHMARK_HISTORY_ENABLED == 'true'" 'history-gated benchmark action'
history_gates=$(grep -Ec "if: vars\.BENCHMARK_HISTORY_ENABLED == 'true'" "$pr_workflow")
[ "$history_gates" -eq 2 ] || fail "$pr_workflow: both benchmark action steps must be history-gated"
reject_text "$pr_workflow" 'skip-fetch-gh-pages:' 'history fetch bypass'
reject_text "$pr_workflow" 'alert-threshold:' 'untested automated PR threshold'
require_text "$pr_workflow" 'ubuntu-24\.04' 'pinned runner image'
require_text "$pr_workflow" 'toolchain: 1\.94\.0' 'pinned Rust toolchain'
require_literal "$pr_workflow" 'runner_image_os=${ImageOS:-unknown}' 'runner image OS metadata'
require_literal "$pr_workflow" 'runner_image_version=${ImageVersion:-unknown}' 'runner image version metadata'
image_os_entries=$(grep -Fc 'runner_image_os=${ImageOS:-unknown}' "$pr_workflow")
image_version_entries=$(grep -Fc 'runner_image_version=${ImageVersion:-unknown}' "$pr_workflow")
[ "$image_os_entries" -eq 2 ] || fail "$pr_workflow: fast and slow jobs must record runner image OS"
[ "$image_version_entries" -eq 2 ] || fail "$pr_workflow: fast and slow jobs must record runner image version"
reject_text "$pr_workflow" 'lighthouse_revision=[0-9a-f]{40}' 'hardcoded Lighthouse revision'
require_literal "$pr_workflow" 'benchmarks/comparison/Cargo.toml' 'Lighthouse revision manifest source'
require_literal "$pr_workflow" "grep -Eq '^[0-9a-f]{40}$'" 'exact Lighthouse revision validation'
require_literal "$pr_workflow" 'echo "lighthouse_revision=$lighthouse_revision"' 'derived Lighthouse revision emission'
lighthouse_manifest_reads=$(grep -Fc 'benchmarks/comparison/Cargo.toml' "$pr_workflow")
lighthouse_revision_emissions=$(grep -Fc 'echo "lighthouse_revision=$lighthouse_revision"' "$pr_workflow")
[ "$lighthouse_manifest_reads" -eq 2 ] || fail "$pr_workflow: fast and slow jobs must derive the Lighthouse revision from the manifest"
[ "$lighthouse_revision_emissions" -eq 2 ] || fail "$pr_workflow: fast and slow jobs must emit their derived Lighthouse revision"
require_text "$pr_workflow" 'de0fac2e4500dabe0009e67214ff5f5447ce83dd' 'pinned checkout action'
require_text "$pr_workflow" '35d8a35b823d6c20db516f5c35eb0a9640942c17' 'pinned Rust toolchain action'
require_text "$pr_workflow" '401aff9a7a08acb9d27b64936a90db81024cff97' 'pinned Rust cache action'
require_text "$pr_workflow" '043fb46d1a93c77aae656e7c1c64a875d1fc6a0a' 'pinned artifact upload action'
require_text "$pr_workflow" '52576c92bccf6ac60c8223ec7eb2565637cae9ba' 'pinned benchmark reporting action'
check_full_action_pins "$pr_workflow"

require_file "$history_workflow"
require_text "$history_workflow" '^  push:$' 'push trigger'
require_text "$history_workflow" '^      - main$' 'main branch restriction'
require_literal "$history_workflow" 'benchmarks/comparison/**' 'comparison benchmark path filter'
require_literal "$history_workflow" 'bindings/rust/**' 'Rust binding path filter'
require_literal "$history_workflow" 'Cargo.lock' 'workspace lockfile path filter'
require_text "$history_workflow" '^  schedule:$' 'weekly schedule trigger'
require_literal "$history_workflow" "cron: '17 3 * * 1'" 'weekly schedule cadence'
require_text "$history_workflow" '^  workflow_dispatch:$' 'manual trigger'
reject_text "$history_workflow" 'pull_request|pull_request_target|workflow_run' 'untrusted history trigger'
require_text "$history_workflow" '^permissions:$' 'top-level permissions block'
require_text "$history_workflow" '^  contents: read$' 'read-only measurement permission'
require_literal "$history_workflow" 'group: benchmark-history' 'shared history concurrency group'
require_literal "$history_workflow" 'cancel-in-progress: false' 'non-cancelling history concurrency'
require_literal "$history_workflow" "if: github.event_name == 'push' || (github.event_name == 'workflow_dispatch' && (inputs.suite == 'fast' || inputs.suite == 'all'))" 'fast trigger routing'
require_literal "$history_workflow" "if: github.event_name == 'schedule' || (github.event_name == 'workflow_dispatch' && (inputs.suite == 'slow' || inputs.suite == 'all'))" 'slow trigger routing'
require_text "$history_workflow" '^      suite:$' 'manual suite input'
require_text "$history_workflow" '^      samples:$' 'manual sample input'
require_text "$history_workflow" '^      same_sizes:$' 'manual same-claim size input'
require_text "$history_workflow" '^      distinct_sizes:$' 'manual distinct-claim size input'
require_text "$history_workflow" '^          - fast$' 'manual fast suite choice'
require_text "$history_workflow" '^          - slow$' 'manual slow suite choice'
require_text "$history_workflow" '^          - all$' 'manual all-suites choice'
require_literal "$history_workflow" 'BENCH_SAMPLES: ${{ github.event_name == '\''workflow_dispatch'\'' && inputs.samples || '\''3'\'' }}' 'manual sample input mapping'
require_literal "$history_workflow" 'BENCH_SAME_SIZES: ${{ github.event_name == '\''workflow_dispatch'\'' && inputs.same_sizes || '\''1,8,16'\'' }}' 'manual same-claim size input mapping'
require_literal "$history_workflow" 'BENCH_DISTINCT_SIZES: ${{ github.event_name == '\''workflow_dispatch'\'' && inputs.distinct_sizes || '\''1,8,16'\'' }}' 'manual distinct-claim size input mapping'
require_literal "$history_workflow" '[[ "$BENCH_SAMPLES" =~ ^[0-9]+$ ]] && (( 10#$BENCH_SAMPLES >= 3 ))' 'publishable sample validation'
require_literal "$history_workflow" '[[ "$BENCH_SAME_SIZES" =~ ^[0-9]+(,[0-9]+)*$ ]]' 'same-claim size syntax validation'
require_literal "$history_workflow" '[[ "$BENCH_DISTINCT_SIZES" =~ ^[0-9]+(,[0-9]+)*$ ]]' 'distinct-claim size syntax validation'
require_literal "$history_workflow" '--samples "$BENCH_SAMPLES"' 'validated sample runner argument'
require_literal "$history_workflow" '--same-sizes "$BENCH_SAME_SIZES"' 'validated same-claim runner argument'
require_literal "$history_workflow" '--distinct-sizes "$BENCH_DISTINCT_SIZES"' 'validated distinct-claim runner argument'
require_literal "$history_workflow" '--warmup-proofs' 'steady-state proof policy'
require_literal "$history_workflow" '--output-format bencher' 'history Criterion Cargo-parser output'
require_literal "$history_workflow" 'timeout-minutes: 30' '30-minute fast timeout'
require_literal "$history_workflow" 'timeout-minutes: 180' '180-minute slow timeout'
require_literal "$history_workflow" 'retention-days: 30' '30-day fast artifact retention'
require_literal "$history_workflow" 'retention-days: 90' '90-day slow artifact retention'
require_literal "$history_workflow" 'runner_image_os=${ImageOS:-unknown}' 'history runner image OS metadata'
require_literal "$history_workflow" 'runner_image_version=${ImageVersion:-unknown}' 'history runner image version metadata'
history_image_os_entries=$(grep -Fc 'runner_image_os=${ImageOS:-unknown}' "$history_workflow")
history_image_version_entries=$(grep -Fc 'runner_image_version=${ImageVersion:-unknown}' "$history_workflow")
[ "$history_image_os_entries" -eq 2 ] || fail "$history_workflow: fast and slow jobs must record runner image OS"
[ "$history_image_version_entries" -eq 2 ] || fail "$history_workflow: fast and slow jobs must record runner image version"
reject_text "$history_workflow" 'lighthouse_revision=[0-9a-f]{40}' 'hardcoded Lighthouse revision'
require_literal "$history_workflow" 'benchmarks/comparison/Cargo.toml' 'history Lighthouse revision manifest source'
require_literal "$history_workflow" "grep -Eq '^[0-9a-f]{40}$'" 'history Lighthouse revision validation'
require_literal "$history_workflow" 'echo "lighthouse_revision=$lighthouse_revision"' 'history derived Lighthouse revision emission'
history_manifest_reads=$(grep -Fc 'benchmarks/comparison/Cargo.toml' "$history_workflow")
history_revision_emissions=$(grep -Fc 'echo "lighthouse_revision=$lighthouse_revision"' "$history_workflow")
[ "$history_manifest_reads" -eq 2 ] || fail "$history_workflow: fast and slow jobs must derive the Lighthouse revision from the manifest"
[ "$history_revision_emissions" -eq 2 ] || fail "$history_workflow: fast and slow jobs must emit their derived Lighthouse revision"
require_literal "$history_workflow" "if: always() && vars.BENCHMARK_HISTORY_ENABLED == 'true'" 'always-evaluated publication gate'
require_literal "$history_workflow" "needs.fast.result == 'success' || needs.slow.result == 'success'" 'successful measurement publication gate'
require_literal "$history_workflow" "if: needs.fast.result == 'success'" 'conditional fast artifact handling'
require_literal "$history_workflow" "if: needs.slow.result == 'success'" 'conditional slow artifact handling'
require_literal "$history_workflow" 'benchmark-data-dir-path: dev/bench/fast' 'fast history path'
require_literal "$history_workflow" 'benchmark-data-dir-path: dev/bench/slow' 'slow history path'
require_literal "$history_workflow" 'auto-push: true' 'history auto-push'
require_literal "$history_workflow" 'max-items-in-chart: 100' 'bounded history chart'
require_literal "$history_workflow" 'github-token: ${{ github.token }}' 'scoped publication token'
history_summaries=$(grep -Fc 'GITHUB_STEP_SUMMARY' "$history_workflow")
[ "$history_summaries" -eq 2 ] || fail "$history_workflow: both measurement jobs must show current results"
reject_text "$history_workflow" 'alert-threshold:' 'untested automated history threshold'
require_text "$history_workflow" 'de0fac2e4500dabe0009e67214ff5f5447ce83dd' 'pinned history checkout action'
require_text "$history_workflow" '35d8a35b823d6c20db516f5c35eb0a9640942c17' 'pinned history Rust toolchain action'
require_text "$history_workflow" '401aff9a7a08acb9d27b64936a90db81024cff97' 'pinned history Rust cache action'
require_text "$history_workflow" '043fb46d1a93c77aae656e7c1c64a875d1fc6a0a' 'pinned history artifact upload action'
require_text "$history_workflow" '3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c' 'pinned history artifact download action'
require_text "$history_workflow" '52576c92bccf6ac60c8223ec7eb2565637cae9ba' 'pinned history benchmark reporting action'
check_full_action_pins "$history_workflow"

measurement_jobs=$(sed '/^  publish:/,$d' "$history_workflow")
if printf '%s\n' "$measurement_jobs" | grep -Eq 'write-all|:[[:space:]]*write([[:space:]]|$)'; then
  fail "$history_workflow: measurement jobs must not have write permissions"
fi
publication_job=$(awk '
  /^  publish:/ { in_publish = 1 }
  in_publish && !/^  publish:/ && /^  [[:alnum:]_-]+:/ { in_publish = 0 }
  in_publish { print }
' "$history_workflow")
[ -n "$publication_job" ] || fail "$history_workflow: missing publication job"
printf '%s\n' "$publication_job" | grep -Eq '^    permissions:$' || fail "$history_workflow: publication job needs explicit permissions"
printf '%s\n' "$publication_job" | grep -Eq '^      contents: write$' || fail "$history_workflow: publication job needs contents write"
printf '%s\n' "$publication_job" | grep -Eq '^      deployments: write$' || fail "$history_workflow: publication job needs deployments write"
if printf '%s\n' "$publication_job" | grep -Eq '^[[:space:]]+run:|scripts/'; then
  fail "$history_workflow: publication job must not execute repository code"
fi
if printf '%s\n' "$publication_job" | grep -Eq '^[[:space:]]+ref:'; then
  fail "$history_workflow: publication checkout must use the trusted event ref"
fi
publication_checkout_refs=$(printf '%s\n' "$publication_job" | grep -Fc 'actions/checkout@' || true)
publication_checkouts=$(printf '%s\n' "$publication_job" | grep -Fc 'actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd' || true)
[ "$publication_checkout_refs" -eq 1 ] || fail "$history_workflow: publication must contain exactly one checkout"
[ "$publication_checkouts" -eq 1 ] || fail "$history_workflow: publication must initialize Git with one pinned checkout"
publication_checkout_step=$(printf '%s\n' "$publication_job" | awk '
  /uses: actions\/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd/ { in_checkout = 1 }
  in_checkout && /^      - name:/ { exit }
  in_checkout { print }
')
publication_credentials=$(printf '%s\n' "$publication_checkout_step" | grep -Fc 'persist-credentials: false' || true)
publication_fetch_depth=$(printf '%s\n' "$publication_checkout_step" | grep -Fc 'fetch-depth: 1' || true)
[ "$publication_credentials" -eq 1 ] || fail "$history_workflow: publication checkout must not persist credentials"
[ "$publication_fetch_depth" -eq 1 ] || fail "$history_workflow: publication checkout must use minimal fetch depth"
publication_checkout_line=$(printf '%s\n' "$publication_job" | grep -n -m 1 'actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd' | cut -d: -f1)
publication_download_line=$(printf '%s\n' "$publication_job" | grep -n -m 1 'actions/download-artifact@' | cut -d: -f1)
publication_reporter_line=$(printf '%s\n' "$publication_job" | grep -n -m 1 'benchmark-action/github-action-benchmark@' | cut -d: -f1)
[ "$publication_checkout_line" -lt "$publication_download_line" ] || fail "$history_workflow: publication checkout must precede artifact downloads"
[ "$publication_checkout_line" -lt "$publication_reporter_line" ] || fail "$history_workflow: publication checkout must precede benchmark reporters"
history_downloads=$(printf '%s\n' "$publication_job" | grep -Fc 'actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c')
history_publishers=$(printf '%s\n' "$publication_job" | grep -Fc 'benchmark-action/github-action-benchmark@52576c92bccf6ac60c8223ec7eb2565637cae9ba')
history_auto_pushes=$(printf '%s\n' "$publication_job" | grep -Fc 'auto-push: true')
history_fast_conditions=$(printf '%s\n' "$publication_job" | grep -Fc "if: needs.fast.result == 'success'")
history_slow_conditions=$(printf '%s\n' "$publication_job" | grep -Fc "if: needs.slow.result == 'success'")
history_tokens=$(printf '%s\n' "$publication_job" | grep -Fc 'github-token: ${{ github.token }}')
[ "$history_downloads" -eq 2 ] || fail "$history_workflow: publication must conditionally download both suites"
[ "$history_publishers" -eq 2 ] || fail "$history_workflow: publication must publish both suites"
[ "$history_auto_pushes" -eq 2 ] || fail "$history_workflow: both history suites must auto-push"
[ "$history_fast_conditions" -eq 2 ] || fail "$history_workflow: fast download and publication must depend on fast success"
[ "$history_slow_conditions" -eq 2 ] || fail "$history_workflow: slow download and publication must depend on slow success"
[ "$history_tokens" -eq 2 ] || fail "$history_workflow: both history suites must use the scoped job token"

require_file "$ci_workflow"
if ! awk '
  /^  rust:/ { in_rust = 1; next }
  in_rust && /^  [[:alnum:]_-]+:/ { in_rust = 0 }
  in_rust && /sh scripts\/check-benchmark-ci\.sh/ { found = 1 }
  END { exit !found }
' "$ci_workflow"; then
  fail "$ci_workflow: Rust job must run sh scripts/check-benchmark-ci.sh"
fi

echo "benchmark CI policy checks passed"
