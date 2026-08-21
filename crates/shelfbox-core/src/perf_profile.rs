//! Test-only phase profiler for manual performance investigations.
//!
//! This module is compiled only for the core crate's tests. Production
//! binaries never include it, and a profile records measurements only between
//! an explicit [`begin`] and [`finish`] call on the same thread.

use std::{
    cell::RefCell,
    collections::BTreeMap,
    time::{Duration, Instant},
};

/// Restore phases that the P2 investigation needs to attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Phase {
    RecoveryFingerprint,
    Materializer,
    Transfer,
    RecordSync,
    Manifest,
    Exclude,
    GitValidation,
}

impl Phase {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::RecoveryFingerprint => "recovery_fingerprint",
            Self::Materializer => "materializer",
            Self::Transfer => "transfer",
            Self::RecordSync => "record_sync",
            Self::Manifest => "manifest",
            Self::Exclude => "exclude",
            Self::GitValidation => "git_validation",
        }
    }
}

/// Aggregate exclusive timing and byte counts for one phase in one operation
/// sample. Nested profiled work is charged to its own phase rather than being
/// counted again in its caller.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Measurement {
    pub(crate) calls: u64,
    pub(crate) bytes: u64,
    pub(crate) elapsed: Duration,
}

/// Measurements collected for one manually profiled operation.
#[derive(Debug)]
pub(crate) struct Snapshot {
    elapsed: Duration,
    measurements: BTreeMap<Phase, Measurement>,
}

impl Snapshot {
    pub(crate) const fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub(crate) fn measurements(&self) -> impl Iterator<Item = (Phase, Measurement)> + '_ {
        self.measurements
            .iter()
            .map(|(phase, measurement)| (*phase, *measurement))
    }
}

#[derive(Debug)]
struct ActiveProfile {
    started: Instant,
    measurements: BTreeMap<Phase, Measurement>,
    phases: Vec<ActivePhase>,
}

#[derive(Debug)]
struct ActivePhase {
    phase: Phase,
    started: Instant,
    child_elapsed: Duration,
}

thread_local! {
    static ACTIVE_PROFILE: RefCell<Option<ActiveProfile>> = const { RefCell::new(None) };
}

/// Starts one profile on the current thread.
///
/// Nested profiles would make phase ownership ambiguous, so they are rejected.
pub(crate) fn begin() {
    ACTIVE_PROFILE.with(|active| {
        let mut active = active.borrow_mut();
        assert!(active.is_none(), "a performance profile is already active");
        *active = Some(ActiveProfile {
            started: Instant::now(),
            measurements: BTreeMap::new(),
            phases: Vec::new(),
        });
    });
}

/// Completes and returns the current thread's profile.
pub(crate) fn finish() -> Snapshot {
    ACTIVE_PROFILE.with(|active| {
        let active = active
            .borrow_mut()
            .take()
            .expect("no performance profile is active");
        Snapshot {
            elapsed: active.started.elapsed(),
            measurements: active.measurements,
        }
    })
}

/// Measures a phase when a profile is active; otherwise runs `operation`
/// directly. The operation's result is returned unchanged.
pub(crate) fn measure<T>(phase: Phase, operation: impl FnOnce() -> T) -> T {
    let active = enter(phase);
    let result = operation();
    if active {
        exit(phase, 0);
    }
    result
}

/// Measures a fallible streaming operation that returns its actual byte count.
/// Failed reads still contribute elapsed time but never claim bytes.
pub(crate) fn measure_result_with_bytes<T, E>(
    phase: Phase,
    operation: impl FnOnce() -> std::result::Result<(T, u64), E>,
) -> std::result::Result<T, E> {
    let active = enter(phase);
    let result = operation();
    if active {
        let bytes = result.as_ref().map_or(0, |(_, bytes)| *bytes);
        exit(phase, bytes);
    }
    result.map(|(value, _)| value)
}

fn enter(phase: Phase) -> bool {
    ACTIVE_PROFILE.with(|active| {
        let mut active = active.borrow_mut();
        let Some(active) = active.as_mut() else {
            return false;
        };
        active.phases.push(ActivePhase {
            phase,
            started: Instant::now(),
            child_elapsed: Duration::ZERO,
        });
        true
    })
}

fn exit(expected_phase: Phase, bytes: u64) {
    ACTIVE_PROFILE.with(|active| {
        let mut active = active.borrow_mut();
        let active = active
            .as_mut()
            .expect("performance profile became inactive before a phase completed");
        let completed = active
            .phases
            .pop()
            .expect("performance profile phase stack is empty");
        assert_eq!(
            completed.phase, expected_phase,
            "performance profile phases completed out of order"
        );
        let elapsed = completed.started.elapsed();
        let exclusive_elapsed = elapsed.saturating_sub(completed.child_elapsed);
        let measurement = active.measurements.entry(expected_phase).or_default();
        measurement.calls += 1;
        measurement.bytes += bytes;
        measurement.elapsed += exclusive_elapsed;
        if let Some(parent) = active.phases.last_mut() {
            parent.child_elapsed += elapsed;
        }
    });
}
