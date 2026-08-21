//! Manual P2 profiling for a single 64 MiB symlink restore.
//!
//! This is deliberately ignored by the ordinary test suite. Run it through
//! `scripts/benchmarks/add-restore/profile-restore.sh`, which provides an
//! isolated configuration and writes the raw measurements under `tmp/`.

use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use tempfile::TempDir;

use crate::{api, context, perf_profile};

use crate::integration_test_common as common;

const FILE_SIZE_BYTES: u64 = 64 * 1024 * 1024;
const WRITE_CHUNK_BYTES: usize = 1024 * 1024;
const EXPECTED_SYMLINK_RESTORE_FINGERPRINT_CALLS: u64 = 6;

#[test]
#[ignore = "manual P2 64 MiB restore profile; use scripts/benchmarks/add-restore/profile-restore.sh"]
fn p2_restore_profile() {
    assert!(
        common::require_symlink_support(),
        "P2 restore profiling requires file-symlink support"
    );

    let output_path = required_path("SHELFBOX_P2_PROFILE_OUTPUT");
    assert!(
        output_path.is_absolute(),
        "SHELFBOX_P2_PROFILE_OUTPUT must be an absolute path because Cargo runs this test from the package directory"
    );
    let samples = positive_usize_env("SHELFBOX_P2_PROFILE_SAMPLES", 10);
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
    writeln!(output, "sample,phase,calls,bytes,elapsed_ms").unwrap();

    for sample in 1..=samples {
        let snapshot = profile_restore_sample(sample, durability);
        write_snapshot(&mut output, sample, &snapshot);
        output.flush().unwrap();
    }
}

fn profile_restore_sample(sample: usize, durability: &str) -> perf_profile::Snapshot {
    let repo_dir = common::init_git_repo();
    let store_dir = TempDir::new().unwrap();
    let file_path = repo_dir.path().join("profile-64mib.bin");
    write_fixture(&file_path, sample as u8);

    let mut ctx = context::build_create_or_load(repo_dir.path(), Some(store_dir.path())).unwrap();
    assert_eq!(ctx.config.mutation_durability.to_string(), durability);
    api::item::add_file(&mut ctx, &file_path, false).unwrap();
    assert!(
        file_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink(),
        "P2 profile must exercise the symlink restore path"
    );

    perf_profile::begin();
    api::item::restore_file(&mut ctx, &file_path, false, false, false).unwrap();
    let snapshot = perf_profile::finish();

    assert!(file_path.is_file());
    assert!(
        !file_path
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink(),
        "restore must leave a regular file"
    );
    assert!(ctx.manifest.items.is_empty());

    let fingerprint = snapshot
        .measurements()
        .find_map(|(phase, measurement)| {
            (phase == perf_profile::Phase::RecoveryFingerprint).then_some(measurement)
        })
        .expect("profile did not record any recovery fingerprints");
    assert_eq!(
        fingerprint.calls, EXPECTED_SYMLINK_RESTORE_FINGERPRINT_CALLS,
        "the P2 profile no longer covers every symlink-restore recovery fingerprint"
    );
    assert_eq!(
        fingerprint.bytes,
        FILE_SIZE_BYTES * EXPECTED_SYMLINK_RESTORE_FINGERPRINT_CALLS,
        "the P2 profile did not report every byte read for recovery fingerprints"
    );

    snapshot
}

fn write_fixture(path: &Path, sample_seed: u8) {
    let mut file = File::create(path)
        .unwrap_or_else(|error| panic!("failed to create fixture {}: {error}", path.display()));
    let mut chunk = vec![0_u8; WRITE_CHUNK_BYTES];
    for (index, byte) in chunk.iter_mut().enumerate() {
        *byte = sample_seed.wrapping_add((index % 251) as u8);
    }
    for _ in 0..(FILE_SIZE_BYTES as usize / WRITE_CHUNK_BYTES) {
        file.write_all(&chunk)
            .unwrap_or_else(|error| panic!("failed to write fixture {}: {error}", path.display()));
    }
    file.sync_all()
        .unwrap_or_else(|error| panic!("failed to sync fixture {}: {error}", path.display()));
}

fn write_snapshot(output: &mut BufWriter<File>, sample: usize, snapshot: &perf_profile::Snapshot) {
    write_row(output, sample, "total", 1, 0, snapshot.elapsed());
    for (phase, measurement) in snapshot.measurements() {
        write_row(
            output,
            sample,
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
    phase: &str,
    calls: u64,
    bytes: u64,
    elapsed: Duration,
) {
    writeln!(
        output,
        "{sample},{phase},{calls},{bytes},{:.3}",
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
    match env::var("SHELFBOX_P2_PROFILE_DURABILITY") {
        Ok(value) if value == "require" => "require",
        Ok(value) if value == "best-effort" => "best-effort",
        Ok(value) => {
            panic!("SHELFBOX_P2_PROFILE_DURABILITY must be require or best-effort, got {value:?}")
        }
        Err(env::VarError::NotPresent) => {
            if cfg!(windows) {
                "best-effort"
            } else {
                "require"
            }
        }
        Err(error) => panic!("failed to read SHELFBOX_P2_PROFILE_DURABILITY: {error}"),
    }
}

fn ensure_output_parent(output_path: &Path) {
    let parent = output_path.parent().unwrap_or_else(|| {
        panic!(
            "SHELFBOX_P2_PROFILE_OUTPUT must include an existing parent directory: {}",
            output_path.display()
        )
    });
    assert!(
        parent.is_dir(),
        "profile output parent does not exist or is not a directory: {}",
        parent.display()
    );
}

/// Holds the isolated config path and restores the caller's process
/// environment after the ignored test completes.
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
