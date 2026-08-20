#!/usr/bin/env bash

# Interleaved end-to-end benchmark for `shelfbox item add` and `item restore`.
# It writes only to a newly created output directory supplied by the caller.

set -Eeuo pipefail

usage() {
    cat <<'EOF'
Usage:
  bash scripts/benchmarks/add-restore/run.sh \
    --baseline <release-binary> \
    --candidate <release-binary> \
    --output-dir <new-directory-under-tmp> [options]

Options:
  --pairs <n>                   Interleaved A/B pairs (default: 30).
  --file-count <n>              Files in each fixture (default: 10).
  --file-size-bytes <n>         Bytes in each fixture file (default: 4096).
  --strategy <symlink|copy>     Configured materialization (default: symlink).
  --durability <require|best-effort>
                                Mutation durability (default: require).
  --gate <none|p1>              Evaluate the Priority 1 primary-case gate.
  --help                        Show this message.

The output directory must not already exist. Fixture creation, source-data generation, logs, and raw results are written beneath it and are excluded from the measured command interval.
EOF
}

fail() {
    printf 'benchmark failed: %s\n' "$*" >&2
    exit 1
}

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "required command is unavailable: $1"
}

absolute_existing_file() {
    local path=$1
    local parent
    parent=$(cd "$(dirname "$path")" && pwd -P)
    printf '%s/%s\n' "$parent" "$(basename "$path")"
}

sha256_file() {
    local path=$1
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{print $1}'
    else
        fail "required command is unavailable: sha256sum or shasum"
    fi
}

now_ns() {
    "$python_command" -c 'import time; print(time.monotonic_ns())'
}

elapsed_ms() {
    "$python_command" - "$1" "$2" <<'PY'
import sys

start, end = map(int, sys.argv[1:])
print(f"{(end - start) / 1_000_000:.3f}")
PY
}

is_positive_integer() {
    [[ $1 =~ ^[1-9][0-9]*$ ]]
}

baseline=
candidate=
output_dir=
pairs=30
file_count=10
file_size_bytes=4096
strategy=symlink
durability=require
gate=none
python_command=

while (($# > 0)); do
    case $1 in
        --baseline)
            baseline=${2:?missing value for --baseline}
            shift 2
            ;;
        --candidate)
            candidate=${2:?missing value for --candidate}
            shift 2
            ;;
        --output-dir)
            output_dir=${2:?missing value for --output-dir}
            shift 2
            ;;
        --pairs)
            pairs=${2:?missing value for --pairs}
            shift 2
            ;;
        --file-count)
            file_count=${2:?missing value for --file-count}
            shift 2
            ;;
        --file-size-bytes)
            file_size_bytes=${2:?missing value for --file-size-bytes}
            shift 2
            ;;
        --strategy)
            strategy=${2:?missing value for --strategy}
            shift 2
            ;;
        --durability)
            durability=${2:?missing value for --durability}
            shift 2
            ;;
        --gate)
            gate=${2:?missing value for --gate}
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

[[ -n $baseline ]] || fail "--baseline is required"
[[ -n $candidate ]] || fail "--candidate is required"
[[ -n $output_dir ]] || fail "--output-dir is required"
is_positive_integer "$pairs" || fail "--pairs must be a positive integer"
is_positive_integer "$file_count" || fail "--file-count must be a positive integer"
is_positive_integer "$file_size_bytes" || fail "--file-size-bytes must be a positive integer"
[[ $strategy == symlink || $strategy == copy ]] || fail "--strategy must be symlink or copy"
[[ $durability == require || $durability == best-effort ]] || fail "--durability must be require or best-effort"
[[ $gate == none || $gate == p1 ]] || fail "--gate must be none or p1"

require_command git
require_command awk
require_command sort
require_command dd

for candidate_python in python3 python; do
    if command -v "$candidate_python" >/dev/null 2>&1 \
        && "$candidate_python" -c 'import sys; raise SystemExit(sys.version_info < (3, 0))'; then
        python_command=$candidate_python
        break
    fi
done
[[ -n $python_command ]] || fail "required command is unavailable: Python 3 (python3 or python)"

baseline=$(absolute_existing_file "$baseline")
candidate=$(absolute_existing_file "$candidate")
[[ -x $baseline ]] || fail "baseline binary is not executable: $baseline"
[[ -x $candidate ]] || fail "candidate binary is not executable: $candidate"

mkdir -p "$(dirname "$output_dir")"
mkdir "$output_dir" || fail "output directory must not already exist: $output_dir"
output_dir=$(cd "$output_dir" && pwd -P)

raw_csv="$output_dir/raw.csv"
summary_csv="$output_dir/summary.csv"
metadata="$output_dir/metadata.txt"
acceptance="$output_dir/acceptance.txt"
logs_dir="$output_dir/logs"
work_dir="$output_dir/work"
mkdir -p "$logs_dir" "$work_dir"

printf 'variant,operation,pair,order,file_count,file_size_bytes,strategy,durability,elapsed_ms\n' >"$raw_csv"

{
    printf 'timestamp_utc='
    date -u +%FT%TZ
    printf 'baseline_binary=%s\n' "$baseline"
    printf 'baseline_sha256=%s\n' "$(sha256_file "$baseline")"
    printf 'baseline_version='
    "$baseline" --version
    printf 'candidate_binary=%s\n' "$candidate"
    printf 'candidate_sha256=%s\n' "$(sha256_file "$candidate")"
    printf 'candidate_version='
    "$candidate" --version
    printf 'pairs=%s\nfile_count=%s\nfile_size_bytes=%s\nstrategy=%s\ndurability=%s\ngate=%s\n' \
        "$pairs" "$file_count" "$file_size_bytes" "$strategy" "$durability" "$gate"
    git --version
    uname -a
} >"$metadata"

paths=()
for ((index = 1; index <= file_count; index++)); do
    paths+=("file-$(printf '%03d' "$index").bin")
done

setup_fixture() {
    local pair=$1
    local variant=$2
    fixture_root="$work_dir/pair-$pair-$variant"
    repo="$fixture_root/repo"
    store="$fixture_root/store"
    home="$fixture_root/home"

    mkdir -p "$repo" "$store" "$home/config/shelfbox" "$home/data"
    git -C "$repo" init -q
    printf 'materialization = "%s"\nmutation_durability = "%s"\n' "$strategy" "$durability" \
        >"$home/config/shelfbox/config.toml"

    local path
    for path in "${paths[@]}"; do
        dd if=/dev/urandom of="$repo/$path" bs="$file_size_bytes" count=1 2>/dev/null
    done
}

measure() {
    local variant=$1
    local operation=$2
    local pair=$3
    local order=$4
    local binary=$5
    local start end elapsed

    start=$(now_ns)
    if ! (
        cd "$repo"
        HOME="$home" XDG_CONFIG_HOME="$home/config" XDG_DATA_HOME="$home/data" \
            "$binary" --store "$store" item "$operation" "${paths[@]}"
    ) >"$logs_dir/$variant-$operation-$pair.out" 2>"$logs_dir/$variant-$operation-$pair.err"; then
        fail "$variant $operation pair $pair failed; see $logs_dir/$variant-$operation-$pair.err"
    fi
    end=$(now_ns)
    elapsed=$(elapsed_ms "$start" "$end")
    printf '%s,%s,%s,%s,%s,%s,%s,%s,%s\n' \
        "$variant" "$operation" "$pair" "$order" "$file_count" "$file_size_bytes" \
        "$strategy" "$durability" "$elapsed" >>"$raw_csv"
}

validate_added_item() {
    local path=$1
    if [[ $strategy == symlink ]]; then
        [[ -L $repo/$path ]] || fail "add did not create a symlink for $path"
    else
        [[ -f $repo/$path && ! -L $repo/$path ]] || fail "add did not create a regular copy for $path"
    fi
    if git -C "$repo" check-ignore -q -- "$path"; then
        return
    else
        local status=$?
        [[ $status == 1 ]] || fail "could not check Git exclude state for $path"
        fail "add did not exclude $path"
    fi
}

validate_restored_item() {
    local path=$1
    [[ -f $repo/$path && ! -L $repo/$path ]] || fail "restore did not recreate a regular file for $path"
    if git -C "$repo" check-ignore -q -- "$path"; then
        fail "restore left $path in Git exclude"
    else
        local status=$?
        [[ $status == 1 ]] || fail "could not check Git exclude state for $path"
    fi
}

run_variant() {
    local variant=$1
    local pair=$2
    local order=$3
    local binary=$4
    local path

    setup_fixture "$pair" "$variant"
    measure "$variant" add "$pair" "$order" "$binary"
    for path in "${paths[@]}"; do
        validate_added_item "$path"
    done
    measure "$variant" restore "$pair" "$order" "$binary"
    for path in "${paths[@]}"; do
        validate_restored_item "$path"
    done
}

for ((pair = 1; pair <= pairs; pair++)); do
    if ((pair % 2 == 1)); then
        run_variant baseline "$pair" first "$baseline"
        run_variant candidate "$pair" second "$candidate"
    else
        run_variant candidate "$pair" first "$candidate"
        run_variant baseline "$pair" second "$baseline"
    fi
done

printf 'variant,operation,samples,min_ms,p50_ms,p95_ms,max_ms\n' >"$summary_csv"
for variant in baseline candidate; do
    for operation in add restore; do
        awk -F, -v variant="$variant" -v operation="$operation" \
            '$1 == variant && $2 == operation { print $9 }' "$raw_csv" \
            | LC_ALL=C sort -n \
            | awk -v variant="$variant" -v operation="$operation" '
                { values[++count] = $1 }
                END {
                    if (count == 0) {
                        exit 1
                    }
                    if (count % 2 == 1) {
                        p50 = values[(count + 1) / 2]
                    } else {
                        p50 = (values[count / 2] + values[count / 2 + 1]) / 2
                    }
                    p95_index = int((95 * count + 99) / 100)
                    printf "%s,%s,%d,%.3f,%.3f,%.3f,%.3f\n", \
                        variant, operation, count, values[1], p50, values[p95_index], values[count]
                }
            ' >>"$summary_csv"
    done
done

{
    printf 'Interleaved A/B benchmark completed.\n\n'
    column -s, -t "$summary_csv" 2>/dev/null || cat "$summary_csv"
    printf '\nRaw samples: raw.csv\nCommand logs: logs/\nFixture data: work/\n'
} | tee "$output_dir/summary.txt"

if [[ $gate == p1 ]]; then
    [[ $pairs -ge 30 ]] || fail "Priority 1 gate requires at least 30 pairs"
    [[ $file_count == 10 && $file_size_bytes == 4096 ]] \
        || fail "Priority 1 gate requires the 10 × 4 KiB primary case"

    if awk -F, '
        NR == 1 { next }
        {
            key = $1 SUBSEP $2
            p50[key] = $5
            p95[key] = $6
        }
        END {
            failed = 0
            for (operation_index = 1; operation_index <= 2; operation_index++) {
                operation = operation_index == 1 ? "add" : "restore"
                baseline_p50 = p50["baseline" SUBSEP operation]
                candidate_p50 = p50["candidate" SUBSEP operation]
                baseline_p95 = p95["baseline" SUBSEP operation]
                candidate_p95 = p95["candidate" SUBSEP operation]
                improvement = (baseline_p50 - candidate_p50) / baseline_p50 * 100
                p95_change = (candidate_p95 - baseline_p95) / baseline_p95 * 100
                printf "%s: P50 improvement %.2f%%; P95 change %.2f%%\n", \
                    operation, improvement, p95_change
                if (improvement < 15 || candidate_p95 > baseline_p95 * 1.05) {
                    failed = 1
                }
            }
            exit failed
        }
    ' "$summary_csv" >"$acceptance"; then
        printf 'Priority 1 gate: passed (host stability must still be reviewed).\n' >>"$acceptance"
    else
        printf 'Priority 1 gate: failed.\n' >>"$acceptance"
        cat "$acceptance" >&2
        exit 1
    fi
    cat "$acceptance"
fi
