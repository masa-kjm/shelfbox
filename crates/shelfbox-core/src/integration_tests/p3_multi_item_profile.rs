//! Manual P3 profiling for the 100 × 4 KiB explicit-path workflow.

use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use tempfile::TempDir;

use crate::{
    context,
    error::Result,
    fs::{
        canonical_transfer::{
            CanonicalTransfer, CanonicalTransferAction, CanonicalTransferCommitOutcome,
            CanonicalTransferFacts, CanonicalTransferInspectionRequest, DefaultCanonicalTransfer,
            PreparedCanonicalTransfer,
        },
        materializer::{
            CommitPermit, DefaultMaterializer, MaterializationAction, MaterializationCommitOutcome,
            MaterializationFacts, MaterializationInspectionRequest, Materializer, MutationJournal,
            PreparedMaterialization,
        },
    },
    git::exclude::{GitInfoExclude, GitInfoExcludeSession, IgnoreBackend},
    ops::{add, restore},
    perf_profile::{self, Measurement, Phase, Snapshot},
};

use crate::integration_test_common as common;

const FILE_COUNT: usize = 100;
const FILE_SIZE_BYTES: usize = 4 * 1024;

#[test]
#[ignore = "manual P3 100 x 4 KiB profile; use scripts/benchmarks/add-restore/profile-multi-item.sh"]
fn p3_multi_item_profile() {
    assert!(
        common::require_symlink_support(),
        "P3 profiling requires file-symlink support"
    );

    let output_path = required_path("SHELFBOX_P3_PROFILE_OUTPUT");
    assert!(
        output_path.is_absolute(),
        "SHELFBOX_P3_PROFILE_OUTPUT must be absolute because Cargo runs this test from the package directory"
    );
    let samples = positive_usize_env("SHELFBOX_P3_PROFILE_SAMPLES", 10);
    let durability = durability_env();
    ensure_output_parent(&output_path);
    let _config_home = ProfileConfigHome::new(durability);

    let output_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_path)
        .unwrap_or_else(|error| {
            panic!(
                "failed to create profile output {}: {error}",
                output_path.display()
            )
        });
    let mut output = BufWriter::new(output_file);
    writeln!(output, "sample,operation,phase,calls,bytes,elapsed_ms").unwrap();

    for sample in 1..=samples {
        let (add_snapshot, restore_snapshot) = profile_sample(sample, durability);
        write_snapshot(&mut output, sample, "add", &add_snapshot);
        write_snapshot(&mut output, sample, "restore", &restore_snapshot);
        output.flush().unwrap();
    }
}

fn profile_sample(sample: usize, durability: &str) -> (Snapshot, Snapshot) {
    let repo_dir = common::init_git_repo();
    let store_dir = TempDir::new().unwrap();
    let paths = write_fixture(repo_dir.path(), sample);
    let mut ctx = context::build_create_or_load(repo_dir.path(), Some(store_dir.path())).unwrap();
    assert_eq!(ctx.config.mutation_durability.to_string(), durability);

    perf_profile::begin();
    let mut add_materializer = ProfiledMaterializer::new(&ctx);
    let mut add_transfer = ProfiledTransfer::new(&ctx);
    let add_ignore = ProfiledIgnore::new(GitInfoExclude::session(&ctx.repo_root));
    for path in &paths {
        add::add_report(
            &mut ctx,
            path,
            false,
            &mut add_materializer,
            &mut add_transfer,
            &add_ignore,
        )
        .unwrap();
    }
    let add_snapshot = perf_profile::finish();

    assert_eq!(ctx.manifest.items.len(), FILE_COUNT);
    assert_phase_count(&add_snapshot, "add", Phase::Manifest, FILE_COUNT as u64);
    assert_fingerprint_coverage(&add_snapshot, "add", FILE_COUNT as u64);

    perf_profile::begin();
    let mut restore_materializer = ProfiledMaterializer::new(&ctx);
    let mut restore_transfer = ProfiledTransfer::new(&ctx);
    let restore_ignore = ProfiledIgnore::new(GitInfoExclude::session(&ctx.repo_root));
    let mut ports = restore::RestorePorts {
        materializer: &mut restore_materializer,
        transfer: &mut restore_transfer,
    };
    for path in &paths {
        restore::restore(
            &mut ctx,
            path,
            false,
            false,
            false,
            &mut ports,
            &restore_ignore,
        )
        .unwrap();
    }
    let restore_snapshot = perf_profile::finish();

    assert!(ctx.manifest.items.is_empty());
    assert_phase_count(
        &restore_snapshot,
        "restore",
        Phase::Manifest,
        FILE_COUNT as u64,
    );
    assert_fingerprint_coverage(&restore_snapshot, "restore", (FILE_COUNT * 6) as u64);
    for path in &paths {
        assert!(path.is_file());
        assert!(
            !path.symlink_metadata().unwrap().file_type().is_symlink(),
            "restore must leave a regular file at {}",
            path.display()
        );
    }

    (add_snapshot, restore_snapshot)
}

fn assert_phase_count(snapshot: &Snapshot, operation: &str, phase: Phase, expected: u64) {
    assert_eq!(
        measurement(snapshot, phase).calls,
        expected,
        "{operation} profile no longer covers the expected {phase:?} calls"
    );
}

fn assert_fingerprint_coverage(snapshot: &Snapshot, operation: &str, expected_calls: u64) {
    let measurement = measurement(snapshot, Phase::RecoveryFingerprint);
    assert_eq!(
        measurement.calls, expected_calls,
        "{operation} profile no longer covers every recovery fingerprint"
    );
    assert_eq!(
        measurement.bytes,
        expected_calls * FILE_SIZE_BYTES as u64,
        "{operation} profile did not report every recovery-fingerprint byte"
    );
}

fn measurement(snapshot: &Snapshot, phase: Phase) -> Measurement {
    snapshot
        .measurements()
        .find_map(|(candidate, measurement)| (candidate == phase).then_some(measurement))
        .unwrap_or_else(|| panic!("profile did not record {phase:?}"))
}

fn write_fixture(repo_root: &Path, sample: usize) -> Vec<PathBuf> {
    (0..FILE_COUNT)
        .map(|index| {
            let path = repo_root.join(format!("profile-{index:03}.bin"));
            let mut bytes = vec![0_u8; FILE_SIZE_BYTES];
            for (offset, byte) in bytes.iter_mut().enumerate() {
                *byte = (sample + index + offset) as u8;
            }
            fs::write(&path, bytes).unwrap_or_else(|error| {
                panic!("failed to write fixture {}: {error}", path.display())
            });
            path
        })
        .collect()
}

fn write_snapshot(
    output: &mut BufWriter<File>,
    sample: usize,
    operation: &str,
    snapshot: &Snapshot,
) {
    write_row(output, sample, operation, "total", 1, 0, snapshot.elapsed());
    for (phase, measurement) in snapshot.measurements() {
        write_row(
            output,
            sample,
            operation,
            phase.as_str(),
            measurement.calls,
            measurement.bytes,
            measurement.elapsed,
        );
    }
}

fn write_row(
    output: &mut BufWriter<File>,
    sample: usize,
    operation: &str,
    phase: &str,
    calls: u64,
    bytes: u64,
    elapsed: Duration,
) {
    writeln!(
        output,
        "{sample},{operation},{phase},{calls},{bytes},{:.3}",
        elapsed.as_secs_f64() * 1_000.0
    )
    .unwrap();
}

fn required_path(name: &str) -> PathBuf {
    env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} must name a new profile CSV file"))
}

fn positive_usize_env(name: &str, default: usize) -> usize {
    match env::var(name) {
        Ok(value) => value
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or_else(|| panic!("{name} must be a positive integer, got {value:?}")),
        Err(env::VarError::NotPresent) => default,
        Err(error) => panic!("failed to read {name}: {error}"),
    }
}

fn durability_env() -> &'static str {
    match env::var("SHELFBOX_P3_PROFILE_DURABILITY") {
        Ok(value) if value == "require" => "require",
        Ok(value) if value == "best-effort" => "best-effort",
        Ok(value) => {
            panic!("SHELFBOX_P3_PROFILE_DURABILITY must be require or best-effort, got {value:?}")
        }
        Err(env::VarError::NotPresent) => {
            if cfg!(windows) {
                "best-effort"
            } else {
                "require"
            }
        }
        Err(error) => panic!("failed to read SHELFBOX_P3_PROFILE_DURABILITY: {error}"),
    }
}

fn ensure_output_parent(output_path: &Path) {
    let parent = output_path.parent().unwrap_or_else(|| {
        panic!(
            "SHELFBOX_P3_PROFILE_OUTPUT must include an existing parent directory: {}",
            output_path.display()
        )
    });
    assert!(
        parent.is_dir(),
        "profile output parent does not exist or is not a directory: {}",
        parent.display()
    );
}

struct ProfileConfigHome {
    _directory: TempDir,
    previous: Option<OsString>,
}

impl ProfileConfigHome {
    fn new(durability: &str) -> Self {
        let directory = TempDir::new().expect("failed to create profile config directory");
        let config_dir = directory.path().join("shelfbox");
        fs::create_dir_all(&config_dir).unwrap_or_else(|error| {
            panic!(
                "failed to create profile config directory {}: {error}",
                config_dir.display()
            )
        });
        fs::write(
            config_dir.join("config.toml"),
            format!("mutation_durability = {durability:?}\n"),
        )
        .unwrap_or_else(|error| panic!("failed to write profile config: {error}"));

        let previous = env::var_os("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", directory.path());
        Self {
            _directory: directory,
            previous,
        }
    }
}

impl Drop for ProfileConfigHome {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => env::set_var("XDG_CONFIG_HOME", previous),
            None => env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}

struct ProfiledMaterializer {
    inner: DefaultMaterializer,
}

impl ProfiledMaterializer {
    fn new(ctx: &context::RepoContext) -> Self {
        Self {
            inner: DefaultMaterializer::new(ctx.repo_root.clone(), ctx.config.store.clone()),
        }
    }
}

impl Materializer for ProfiledMaterializer {
    fn inspect(&self, request: MaterializationInspectionRequest) -> Result<MaterializationFacts> {
        perf_profile::measure(Phase::Materializer, || self.inner.inspect(request))
    }

    fn prepare(
        &mut self,
        action: MaterializationAction,
        journal: &mut dyn MutationJournal,
    ) -> Result<PreparedMaterialization> {
        perf_profile::measure(Phase::Materializer, || self.inner.prepare(action, journal))
    }

    fn commit(
        &mut self,
        prepared: PreparedMaterialization,
        permit: CommitPermit,
    ) -> Result<MaterializationCommitOutcome> {
        perf_profile::measure(Phase::Materializer, || self.inner.commit(prepared, permit))
    }

    fn abort(
        &mut self,
        prepared: PreparedMaterialization,
        journal: &mut dyn MutationJournal,
    ) -> Result<()> {
        perf_profile::measure(Phase::Materializer, || self.inner.abort(prepared, journal))
    }
}

struct ProfiledTransfer {
    inner: DefaultCanonicalTransfer,
}

impl ProfiledTransfer {
    fn new(ctx: &context::RepoContext) -> Self {
        Self {
            inner: DefaultCanonicalTransfer::new(ctx.repo_root.clone(), ctx.config.store.clone()),
        }
    }
}

impl CanonicalTransfer for ProfiledTransfer {
    fn inspect(
        &self,
        request: CanonicalTransferInspectionRequest,
    ) -> Result<CanonicalTransferFacts> {
        perf_profile::measure(Phase::Transfer, || self.inner.inspect(request))
    }

    fn prepare(
        &mut self,
        action: CanonicalTransferAction,
        journal: &mut dyn MutationJournal,
    ) -> Result<PreparedCanonicalTransfer> {
        perf_profile::measure(Phase::Transfer, || self.inner.prepare(action, journal))
    }

    fn commit(
        &mut self,
        prepared: PreparedCanonicalTransfer,
        permit: CommitPermit,
    ) -> Result<CanonicalTransferCommitOutcome> {
        perf_profile::measure(Phase::Transfer, || self.inner.commit(prepared, permit))
    }

    fn abort(
        &mut self,
        prepared: PreparedCanonicalTransfer,
        journal: &mut dyn MutationJournal,
    ) -> Result<()> {
        perf_profile::measure(Phase::Transfer, || self.inner.abort(prepared, journal))
    }

    fn prune_empty_item_ancestors(&self, repo_store: &Path, item_path: &Path) -> Result<()> {
        perf_profile::measure(Phase::Transfer, || {
            self.inner.prune_empty_item_ancestors(repo_store, item_path)
        })
    }
}

struct ProfiledIgnore {
    inner: GitInfoExcludeSession,
}

impl ProfiledIgnore {
    fn new(inner: GitInfoExcludeSession) -> Self {
        Self { inner }
    }
}

impl IgnoreBackend for ProfiledIgnore {
    fn add_entries(&self, repo_root: &Path, entries: &[&str]) -> Result<()> {
        perf_profile::measure(Phase::Exclude, || {
            self.inner.add_entries(repo_root, entries)
        })
    }

    fn remove_entries(&self, repo_root: &Path, entries: &[&str]) -> Result<()> {
        perf_profile::measure(Phase::Exclude, || {
            self.inner.remove_entries(repo_root, entries)
        })
    }

    fn has_entry(&self, repo_root: &Path, entry: &str) -> Result<bool> {
        perf_profile::measure(Phase::Exclude, || self.inner.has_entry(repo_root, entry))
    }
}
