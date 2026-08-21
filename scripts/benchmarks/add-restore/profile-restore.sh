#!/usr/bin/env bash

# Manual P2 profiler for the 1 x 64 MiB symlink restore path.
#
# The ignored Rust test writes phase-level raw data. This runner owns the reproducible invocation, output-directory safety checks, metadata, and CSV summary so generated records remain outside the tracked benchmark harness.

set -Eeuo pipefail

usage() {
    cat <<'EOF'
Usage:
  bash scripts/benchmarks/add-restore/profile-restore.sh \
    --output-dir <new-directory-under-tmp> [options]

Options:
  --samples <n>                 Restore samples (default: 10).
  --durability <require|best-effort>
                                Mutation durability (default: require).
  --help                        Show this message.

The runner builds and executes the ignored P2 profile test. The output directory must not exist and must be located below this checkout's tmp/.
EOF
}

fail() {
    printf 'P2 profile failed: %s\n' "$*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "required command is unavailable: $1"
}

find_python() {
    local candidate
    for candidate in python3 python; do
        if command -v "$candidate" >/dev/null 2>&1 \
            && "$candidate" -c 'import sys; raise SystemExit(sys.version_info < (3, 0))'; then
            printf '%s\n' "$candidate"
            return
        fi
    done
    return 1
}

is_positive_integer() {
    [[ $1 =~ ^[1-9][0-9]*$ ]]
}

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(cd "$script_dir/../../.." && pwd -P)
output_dir=
samples=10
durability=require

while (($# > 0)); do
    case $1 in
        --output-dir)
            output_dir=${2:?missing value for --output-dir}
            shift 2
            ;;
        --samples)
            samples=${2:?missing value for --samples}
            shift 2
            ;;
        --durability)
            durability=${2:?missing value for --durability}
            shift 2
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            usage >&2
            fail "unknown argument: $1"
            ;;
    esac
done

[[ -n $output_dir ]] || fail "--output-dir is required"
is_positive_integer "$samples" || fail "--samples must be a positive integer"
[[ $durability == require || $durability == best-effort ]] || \
    fail "--durability must be require or best-effort"

require_command cargo
require_command git
python_command=$(find_python) || fail "required command is unavailable: Python 3 (python3 or python)"

mkdir -p "$(dirname "$output_dir")"
output_parent=$(cd "$(dirname "$output_dir")" && pwd -P)
output_dir="$output_parent/$(basename "$output_dir")"
case $output_dir in
    "$repo_root"/tmp/*) ;;
    *) fail "--output-dir must be below $repo_root/tmp" ;;
esac
[[ ! -e $output_dir ]] || fail "output directory already exists: $output_dir"
mkdir "$output_dir"

raw_csv="$output_dir/raw.csv"
summary_csv="$output_dir/summary.csv"
metadata="$output_dir/metadata.txt"
logs_dir="$output_dir/logs"
mkdir "$logs_dir"

{
    printf 'timestamp_utc='
    date -u +%FT%TZ
    printf 'commit='
    git -C "$repo_root" rev-parse HEAD
    printf 'worktree_dirty='
    if git -C "$repo_root" diff --quiet && git -C "$repo_root" diff --cached --quiet; then
        printf 'false\n'
    else
        printf 'true\n'
    fi
    printf 'samples=%s\n' "$samples"
    printf 'file_count=1\n'
    printf 'file_size_bytes=67108864\n'
    printf 'strategy=symlink\n'
    printf 'durability=%s\n' "$durability"
    printf 'timing=exclusive phase time; nested profiled work is charged to its own phase\n'
    printf 'fingerprint_bytes=actual bytes read by RecoveryFingerprint::from_file\n'
    printf 'platform='
    uname -srm
} >"$metadata"

cd "$repo_root"
SHELFBOX_P2_PROFILE_OUTPUT="$raw_csv" \
SHELFBOX_P2_PROFILE_SAMPLES="$samples" \
SHELFBOX_P2_PROFILE_DURABILITY="$durability" \
    cargo test --release --locked -p shelfbox-core p2_restore_profile -- --ignored --nocapture \
    2>&1 | tee "$logs_dir/test.log"

[[ -s $raw_csv ]] || fail "profile test did not produce raw.csv"

"$python_command" - "$raw_csv" "$summary_csv" <<'PY'
import csv
import math
import statistics
import sys
from collections import defaultdict

raw_path, summary_path = sys.argv[1:]
by_phase = defaultdict(list)
total_by_sample = {}
phase_sum_by_sample = defaultdict(float)

with open(raw_path, newline="", encoding="utf-8") as raw_file:
    for row in csv.DictReader(raw_file):
        sample = int(row["sample"])
        phase = row["phase"]
        value = {
            "calls": int(row["calls"]),
            "bytes": int(row["bytes"]),
            "elapsed_ms": float(row["elapsed_ms"]),
        }
        by_phase[phase].append(value)
        if phase == "total":
            total_by_sample[sample] = value["elapsed_ms"]
        else:
            phase_sum_by_sample[sample] += value["elapsed_ms"]

if not total_by_sample:
    raise SystemExit("raw.csv has no total rows")

for sample, total in total_by_sample.items():
    by_phase["unattributed"].append(
        {"calls": 0, "bytes": 0, "elapsed_ms": max(0.0, total - phase_sum_by_sample[sample])}
    )

def percentile_nearest_rank(values, percentile):
    values = sorted(values)
    return values[max(0, math.ceil(percentile * len(values)) - 1)]

with open(summary_path, "w", newline="", encoding="utf-8") as summary_file:
    writer = csv.writer(summary_file)
    writer.writerow([
        "phase", "samples", "calls_min", "calls_max", "bytes_min", "bytes_max",
        "p50_ms", "p95_ms", "mean_ms",
    ])
    for phase in sorted(by_phase):
        values = by_phase[phase]
        elapsed = [value["elapsed_ms"] for value in values]
        writer.writerow([
            phase,
            len(values),
            min(value["calls"] for value in values),
            max(value["calls"] for value in values),
            min(value["bytes"] for value in values),
            max(value["bytes"] for value in values),
            f"{statistics.median(elapsed):.3f}",
            f"{percentile_nearest_rank(elapsed, 0.95):.3f}",
            f"{statistics.mean(elapsed):.3f}",
        ])
PY

printf 'P2 profile complete: %s\n' "$output_dir"
