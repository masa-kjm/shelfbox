//! Typed plan and report vocabulary for explicit materialization conversion.
//!
//! A configured strategy is only a default for newly created materializations.
//! This request is the explicit, per-item opt-in for converting an existing
//! healthy materialization without changing its manifest or ownership state.

use std::path::PathBuf;

use crate::domain::materialization::MaterializationStrategy;

/// A policy-approved item materialization action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemMaterializeAction {
    /// The observed strategy already matches the requested strategy.
    AlreadyMaterialized,
    /// Atomically replace the observed healthy materialization.
    Replace {
        from: MaterializationStrategy,
        to: MaterializationStrategy,
    },
}

/// User-visible result of planning or executing a materialization request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializeOutcome {
    AlreadyMaterialized,
    Materialized,
    WouldMaterialize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMaterializePlan {
    pub path: String,
    pub abs_path: PathBuf,
    pub store_path: PathBuf,
    pub strategy: MaterializationStrategy,
    pub action: ItemMaterializeAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMaterializeReport {
    pub plan: ItemMaterializePlan,
    pub outcome: MaterializeOutcome,
    pub dry_run: bool,
}

/// Options for an explicit item strategy conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemMaterializeRequest {
    pub strategy: MaterializationStrategy,
    pub dry_run: bool,
}
