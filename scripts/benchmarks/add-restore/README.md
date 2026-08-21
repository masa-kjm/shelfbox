# Add/Restore Benchmark Harness

This directory contains the tracked, reproducible benchmark harness for `shelfbox item add` and `shelfbox item restore`. Generated measurements do not belong in Git: write them to a newly named directory under the repository's `tmp/` directory.

## What It Measures

Each baseline/candidate sample receives a fresh uncommitted Git repository, Shelfbox store, HOME, and XDG configuration. Fixture creation and random-data generation happen before timing. The timed commands are:

```text
shelfbox --store <store> item add <paths...>
shelfbox --store <store> item restore <paths...>
```

The runner verifies the configured materialization after add, then verifies that restore produces regular files and removes the managed Git excludes.
Baseline and candidate order alternate for every pair.

## Bash Runner

`run.sh` supports Linux, macOS, WSL, and native Windows Git Bash with Bash,
Git, Python 3 (`python3` or `python`), `awk`, `sort`, and either `sha256sum`
or `shasum`. Pass two already-built release binaries and an output directory
that does not yet exist:

Prepare two release builds in separate directories, then run:

```sh
BASELINE_REF=<baseline_commit_hash>
CANDIDATE_REF=<candidate_commit_hash>
BASELINE_DIR=/c/src/shelfbox-p1-baseline
CANDIDATE_DIR=/c/src/shelfbox-p1-candidate

git worktree add --detach "$BASELINE_DIR" "$BASELINE_REF"
git worktree add --detach "$CANDIDATE_DIR" "$CANDIDATE_REF"

cd "$BASELINE_DIR"
cargo build --release --locked -p shelfbox
cd "$CANDIDATE_DIR"
cargo build --release --locked -p shelfbox
```

Execute the benchmark harness:

```sh
bash scripts/benchmarks/add-restore/run.sh \
  --baseline /absolute/path/to/baseline/shelfbox \
  --candidate /absolute/path/to/candidate/shelfbox \
  --output-dir tmp/20260820-p1-primary \
  --pairs 30 \
  --file-count 10 \
  --file-size-bytes 4096 \
  --strategy symlink \
  --durability require \
  --gate p1
```

The runner never removes a directory. It refuses an output directory that already exists and writes `raw.csv`, `summary.csv`, `metadata.txt`, logs, and fresh fixtures beneath the new directory.

Use `--strategy copy` for the Copy path. On native Windows, the default `require` durability mode is unavailable; use `--durability best-effort` and compare only runs with the same durability setting. Symlink mode on Windows also requires Developer Mode or an elevated shell.

For a native Windows run from Git Bash, pass Windows-built `.exe` binaries using Git-Bash paths (for example `/c/src/shelfbox/target/release/shelfbox.exe`) and write the result under the native checkout's `tmp/` directory. PowerShell is not required by this runner.

`--gate p1` applies the plan's primary acceptance thresholds only when the input is the 10 × 4 KiB case with at least 30 pairs:

- add and restore P50 must each improve by at least 15%; and
- neither P95 may exceed its own baseline P95 by more than 5%.

The gate does not establish host stability. If a baseline has large outliers, retain the raw data and repeat the run on a stable native runner before using P95 as a pass/fail result.

## P2 Restore Profile

`profile-restore.sh` profiles the merged P1 baseline's 1 × 64 MiB symlink restore path before any P2 implementation. It runs an ignored core test; fixture creation and `add` occur outside the profiled interval. The profiled interval starts immediately before `restore_file`, so it includes P1's restore-session construction and the restore workflow, but not CLI process startup or context construction.

```sh
bash scripts/benchmarks/add-restore/profile-restore.sh \
  --output-dir tmp/20260820-p2-restore-wsl \
  --samples 10 \
  --durability require
```

The runner refuses to reuse an output directory and writes `raw.csv`, `summary.csv`, `metadata.txt`, and `logs/test.log`. `metadata.txt` records the source commit and whether its worktree was dirty, because the test-only profiler is compiled from the current checkout. Phase rows use exclusive elapsed time: nested profiled work is attributed to its own phase, and `summary.csv` reports the remaining `unattributed` time. `recovery_fingerprint.bytes` is the actual stream-read byte count, not a file-size estimate.

For native Windows Git Bash, use a new directory below that checkout's `tmp/` and pass `--durability best-effort`. File-symlink support still requires Developer Mode or an elevated shell.

## P3 Multi-Item Profile

`profile-multi-item.sh` profiles the merged P1 baseline's 100 × 4 KiB symlink add and restore workflow before any P3 implementation.
It determines whether manifest, managed exclude, and recovery-record persistence account for enough of the multi-item operation to justify a command-level recovery coordinator.
The ignored core test shares the P1 operation ports and exclude session across 100 explicit file paths.
Fixture creation and context construction occur outside each profiled interval.

```sh
bash scripts/benchmarks/add-restore/profile-multi-item.sh \
  --output-dir tmp/20260821-p3-profile-windows \
  --samples 10 \
  --durability require
```

The runner refuses to reuse an output directory and writes `raw.csv`, `summary.csv`, `metadata.txt`, and `logs/test.log`.
`metadata.txt` records the source commit and whether its worktree was dirty, because the test-only profiler is compiled from the current checkout.
Phase rows use exclusive elapsed time: nested profiled work is attributed to its own phase, and `summary.csv` reports the remaining `unattributed` time.
The profile records recovery-fingerprint calls and actual streamed bytes alongside materialization, transfer, record synchronization, manifest, exclude, and Git-validation time.

For native Windows Git Bash, use a new directory below that checkout's `tmp/` and pass `--durability best-effort`.
File-symlink support still requires Developer Mode or an elevated shell.

The P3 gate requires batchable manifest, exclude, and recovery-record persistence to account for at least 25% of the relevant P50 before designing a command-level coordinator.
Do not treat a large aggregate record-sync measurement alone as permission to defer writes: every item must retain a durable pre-mutation record and its recovery boundaries.

## Follow-Up Matrix

Run the primary case first. Then use the same harness for the 100 × 4 KiB scale case and the 1 × 64 MiB transfer case. Run 10 × 64 MiB only for a candidate intended to improve byte-transfer paths. Do not run 100 × 64 MiB as a routine benchmark.

The current Bash runner creates repo and store fixtures below its output directory, so it measures same-filesystem transfer. A forced EXDEV benchmark requires an explicitly configured fixture/store arrangement and should be added with a platform-specific runner rather than inferred from this case.
