//! Batch plan and report types for `repo sync`.
//!
//! Planning validates every attached item before the first write. Execution is
//! deliberately ordered and stops at the first runtime race or I/O failure;
//! completed earlier items are retained in the report rather than hidden by a
//! generic error.

use crate::plan::item_sync::{ItemSyncPlan, SyncDirection, SyncOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepoSyncRequest {
    pub direction: SyncDirection,
    pub dry_run: bool,
    /// Required only when the selected direction would overwrite at least one
    /// regular-copy target. Dry-runs and all-no-op plans do not require
    /// confirmation.
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSyncPlan {
    pub direction: SyncDirection,
    pub items: Vec<ItemSyncPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSyncItemReport {
    pub path: String,
    pub outcome: Option<SyncOutcome>,
    /// A post-validation execution failure. It is surfaced in the report so
    /// callers can see exactly which prior items completed before execution
    /// was halted.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSyncReport {
    pub plan: RepoSyncPlan,
    pub items: Vec<RepoSyncItemReport>,
    pub dry_run: bool,
    /// `true` means all plans passed initial validation but an execution-time
    /// precondition or I/O failure stopped the ordered batch.
    pub halted: bool,
}

impl RepoSyncReport {
    pub fn has_failures(&self) -> bool {
        self.items.iter().any(|item| item.error.is_some())
    }
}
