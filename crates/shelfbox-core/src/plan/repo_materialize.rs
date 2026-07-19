//! Batch plan and report types for `repo materialize`.

use crate::{
    domain::materialization::MaterializationStrategy,
    plan::item_materialize::{ItemMaterializePlan, MaterializeOutcome},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepoMaterializeRequest {
    pub strategy: MaterializationStrategy,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMaterializePlan {
    pub strategy: MaterializationStrategy,
    pub items: Vec<ItemMaterializePlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMaterializeItemReport {
    pub path: String,
    pub outcome: Option<MaterializeOutcome>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMaterializeReport {
    pub plan: RepoMaterializePlan,
    pub items: Vec<RepoMaterializeItemReport>,
    pub dry_run: bool,
    pub halted: bool,
}

impl RepoMaterializeReport {
    pub fn has_failures(&self) -> bool {
        self.items.iter().any(|item| item.error.is_some())
    }
}
