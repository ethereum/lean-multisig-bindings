#!/bin/sh
set -eu

pr_workflow=.github/workflows/benchmarks-pr.yml

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
require_text "$pr_workflow" 'de0fac2e4500dabe0009e67214ff5f5447ce83dd' 'pinned checkout action'
require_text "$pr_workflow" '35d8a35b823d6c20db516f5c35eb0a9640942c17' 'pinned Rust toolchain action'
require_text "$pr_workflow" '401aff9a7a08acb9d27b64936a90db81024cff97' 'pinned Rust cache action'
require_text "$pr_workflow" '043fb46d1a93c77aae656e7c1c64a875d1fc6a0a' 'pinned artifact upload action'
require_text "$pr_workflow" '52576c92bccf6ac60c8223ec7eb2565637cae9ba' 'pinned benchmark reporting action'
check_full_action_pins "$pr_workflow"

echo "benchmark CI policy checks passed"
