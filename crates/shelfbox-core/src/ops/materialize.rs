//! Explicit conversion of one existing repository materialization.
//!
//! The operation never changes manifest identity or ownership. It accepts
//! only a healthy managed symlink or an equal isolated regular copy, then asks
//! `Materializer` to atomically replace that entry after a fresh no-follow,
//! Git, and exclude validation. Copy conversion uses the standard artifact
//! lease protocol through `RepairMutationJournal`.

use std::path::Path;

use crate::{
    context::RepoContext,
    domain::{
        materialization::{CopyContentState, MaterializationStrategy},
        path::{RepoRelativePath, StoreRelativePath},
    },
    error::{AppError, Result},
    fs::{
        materializer::{
            DefaultMaterializer, InspectionPurpose, MaterializationAction, MaterializationFacts,
            MaterializationInspectionRequest, MaterializationLocation, Materializer,
            MutationJournal, RepoEntryKind,
        },
        mutation_journal::RepairMutationJournal,
    },
    git,
    ignore::IgnoreBackend,
    plan::item_materialize::{
        ItemMaterializeAction, ItemMaterializePlan, ItemMaterializeReport, ItemMaterializeRequest,
        MaterializeOutcome,
    },
    policy::materialize_policy::{self, MaterializeDecision, MaterializeState},
};

use super::path::repo_relative_string;

pub(crate) fn materialize_report(
    ctx: &RepoContext,
    abs_path: &Path,
    request: ItemMaterializeRequest,
    ignore: &dyn IgnoreBackend,
) -> Result<ItemMaterializeReport> {
    let plan = build_materialize_plan(ctx, abs_path, request.strategy, ignore)?;
    if request.dry_run {
        return Ok(ItemMaterializeReport {
            outcome: dry_run_outcome(plan.action),
            plan,
            dry_run: true,
        });
    }

    let outcome = execute_materialize_plan(ctx, &plan, ignore)?;
    Ok(ItemMaterializeReport {
        plan,
        outcome,
        dry_run: false,
    })
}

/// Builds a complete, read-only plan. Repo-level orchestration reuses this
/// before performing its first write, so batch validation uses exactly the
/// same policy as `item materialize`.
pub(crate) fn build_materialize_plan(
    ctx: &RepoContext,
    abs_path: &Path,
    strategy: MaterializationStrategy,
    ignore: &dyn IgnoreBackend,
) -> Result<ItemMaterializePlan> {
    let path = repo_relative_string(&ctx.repo_root, abs_path)?;
    let item = ctx
        .manifest
        .get(&path)
        .ok_or_else(|| AppError::NotManagedLink {
            path: abs_path.to_path_buf(),
        })?;
    if git::is_tracked(&ctx.repo_root, abs_path)? {
        return Err(AppError::PathIsTracked {
            path: abs_path.to_path_buf(),
        });
    }
    if !ignore.has_entry(&ctx.repo_root, &path)? {
        return Err(AppError::Internal(
            "managed materialization exclude is missing; restore its exact exclude entry before conversion"
                .into(),
        ));
    }

    let location = materialization_location(ctx, &path, &item.store_path)?;
    let store_path = ctx.config.store.join(location.store_path.as_str());
    let materializer = DefaultMaterializer::new(ctx.repo_root.clone(), ctx.config.store.clone());
    let facts = materializer.inspect(MaterializationInspectionRequest {
        location,
        purpose: InspectionPurpose::Planning,
    })?;
    validate_store_facts(&store_path, &facts)?;
    if facts.repo_entry_kind == RepoEntryKind::RegularFile && !facts.hardlink_free {
        return Err(AppError::HardlinkedFile {
            path: abs_path.to_path_buf(),
        });
    }

    let action = match materialize_policy::decide_materialize(strategy, materialize_state(&facts)) {
        MaterializeDecision::NoOp => ItemMaterializeAction::AlreadyMaterialized,
        MaterializeDecision::Replace { from } => {
            ItemMaterializeAction::Replace { to: strategy, from }
        }
        MaterializeDecision::RequiresSync => {
            return Err(AppError::ContentDivergedRequiresSync {
                path: abs_path.to_path_buf(),
            });
        }
        MaterializeDecision::Reject => {
            return Err(AppError::UnsafeFilesystemEntry {
                path: abs_path.to_path_buf(),
                reason: "materialize requires a managed symlink or an isolated equal regular copy",
            });
        }
    };

    Ok(ItemMaterializePlan {
        path,
        abs_path: abs_path.to_path_buf(),
        store_path,
        strategy,
        action,
    })
}

/// Executes a previously validated item plan. The method repeats all mutable
/// preconditions immediately before commit; a changed item is rejected rather
/// than silently receiving a conversion selected from stale facts.
pub(crate) fn execute_materialize_plan(
    ctx: &RepoContext,
    plan: &ItemMaterializePlan,
    ignore: &dyn IgnoreBackend,
) -> Result<MaterializeOutcome> {
    if plan.action == ItemMaterializeAction::AlreadyMaterialized {
        return Ok(MaterializeOutcome::AlreadyMaterialized);
    }

    let item = ctx.manifest.get(&plan.path).ok_or_else(|| {
        AppError::Internal("materialize plan item disappeared from the manifest".into())
    })?;
    let location = materialization_location(ctx, &plan.path, &item.store_path)?;
    let mut materializer =
        DefaultMaterializer::new(ctx.repo_root.clone(), ctx.config.store.clone());
    let facts = materializer.inspect(MaterializationInspectionRequest {
        location: location.clone(),
        purpose: InspectionPurpose::PreCommit,
    })?;
    validate_precommit_facts(ctx, plan, ignore, &facts)?;

    let action = MaterializationAction::Replace {
        location: location.clone(),
        strategy: plan.strategy,
        expected: facts.expected(),
    };
    let mut journal = RepairMutationJournal::new(
        &ctx.config.store,
        &ctx.repo_root,
        ignore,
        ctx.repo_id.clone(),
        plan.path.clone(),
        plan.abs_path.clone(),
        plan.store_path.clone(),
    )
    .with_durability(ctx.config.mutation_durability);
    let prepared = materializer.prepare(action, &mut journal)?;

    let fresh = materializer.inspect(MaterializationInspectionRequest {
        location: location.clone(),
        purpose: InspectionPurpose::PreCommit,
    })?;
    validate_precommit_facts(ctx, plan, ignore, &fresh)?;
    let permit =
        journal.issue_commit_permit(fresh.write_precondition_guard(prepared.commit_context()))?;
    materializer.commit(prepared, permit)?;

    let post = materializer.inspect(MaterializationInspectionRequest {
        location,
        purpose: InspectionPurpose::PostCommit,
    })?;
    if !matches_requested_strategy(plan.strategy, &post)
        || !ignore.has_entry(&ctx.repo_root, &plan.path)?
        || git::is_tracked(&ctx.repo_root, &plan.abs_path)?
    {
        return Err(AppError::Internal(
            "materialize postconditions failed; replacement was retained for inspection".into(),
        ));
    }
    journal.cleanup_all()?;
    Ok(MaterializeOutcome::Materialized)
}

fn materialization_location(
    ctx: &RepoContext,
    repo_path: &str,
    item_store_path: &str,
) -> Result<MaterializationLocation> {
    let repo_path = RepoRelativePath::new(repo_path.to_owned()).ok_or_else(|| {
        AppError::UnsafeFilesystemEntry {
            path: ctx.repo_root.join(repo_path),
            reason: "materialize repository path is not normalized",
        }
    })?;
    let store_absolute = ctx.repo_store.join(item_store_path);
    let store_relative = store_absolute
        .strip_prefix(&ctx.config.store)
        .map_err(|_| AppError::UnsafeFilesystemEntry {
            path: store_absolute.clone(),
            reason: "materialize store path escapes the configured store root",
        })?;
    let store_path = StoreRelativePath::new(store_relative.to_string_lossy().replace('\\', "/"))
        .ok_or(AppError::UnsafeFilesystemEntry {
            path: store_absolute,
            reason: "materialize store path is not normalized",
        })?;
    Ok(MaterializationLocation::new(repo_path, store_path))
}

fn validate_store_facts(store_path: &Path, facts: &MaterializationFacts) -> Result<()> {
    if !facts.store_exists {
        return Err(AppError::StoreMissing {
            path: store_path.to_path_buf(),
            store_path: store_path.to_path_buf(),
        });
    }
    if !facts.store_regular || !facts.store_hardlink_free {
        return Err(AppError::UnsafeFilesystemEntry {
            path: store_path.to_path_buf(),
            reason: "materialize store entry is not an isolated regular file",
        });
    }
    Ok(())
}

fn materialize_state(facts: &MaterializationFacts) -> MaterializeState {
    match facts.repo_entry_kind {
        RepoEntryKind::ManagedSymlink => MaterializeState::ManagedSymlink,
        RepoEntryKind::RegularFile if facts.hardlink_free => match facts.copy_content {
            CopyContentState::Equal => MaterializeState::EqualRegularCopy,
            CopyContentState::Diverged => MaterializeState::DivergedRegularCopy,
            CopyContentState::NotCompared
            | CopyContentState::Unreadable
            | CopyContentState::ComparisonFailed => MaterializeState::Unsafe,
        },
        RepoEntryKind::Missing => MaterializeState::Missing,
        RepoEntryKind::RegularFile
        | RepoEntryKind::UnmanagedSymlinkOrReparsePoint
        | RepoEntryKind::Directory
        | RepoEntryKind::Other => MaterializeState::Unsafe,
    }
}

fn validate_precommit_facts(
    ctx: &RepoContext,
    plan: &ItemMaterializePlan,
    ignore: &dyn IgnoreBackend,
    facts: &MaterializationFacts,
) -> Result<()> {
    validate_store_facts(&plan.store_path, facts)?;
    if !facts.hardlink_free {
        return Err(AppError::HardlinkedFile {
            path: plan.abs_path.clone(),
        });
    }
    if git::is_tracked(&ctx.repo_root, &plan.abs_path)? {
        return Err(AppError::PathIsTracked {
            path: plan.abs_path.clone(),
        });
    }
    if !ignore.has_entry(&ctx.repo_root, &plan.path)? {
        return Err(AppError::Internal(
            "managed materialization exclude was removed before commit authorization".into(),
        ));
    }
    let expected = match plan.action {
        ItemMaterializeAction::AlreadyMaterialized => return Ok(()),
        ItemMaterializeAction::Replace { from, to } if to == plan.strategy => from,
        ItemMaterializeAction::Replace { .. } => {
            return Err(AppError::Internal(
                "materialize plan strategy does not match its replace action".into(),
            ));
        }
    };
    if observed_strategy(facts) != Some(expected)
        || matches!(
            materialize_policy::decide_materialize(plan.strategy, materialize_state(facts)),
            MaterializeDecision::NoOp
                | MaterializeDecision::RequiresSync
                | MaterializeDecision::Reject
        )
    {
        return Err(AppError::FilesystemEntryChanged {
            path: plan.abs_path.clone(),
        });
    }
    Ok(())
}

fn observed_strategy(facts: &MaterializationFacts) -> Option<MaterializationStrategy> {
    match facts.repo_entry_kind {
        RepoEntryKind::ManagedSymlink => Some(MaterializationStrategy::Symlink),
        RepoEntryKind::RegularFile if facts.hardlink_free => Some(MaterializationStrategy::Copy),
        RepoEntryKind::Missing
        | RepoEntryKind::RegularFile
        | RepoEntryKind::UnmanagedSymlinkOrReparsePoint
        | RepoEntryKind::Directory
        | RepoEntryKind::Other => None,
    }
}

fn matches_requested_strategy(
    strategy: MaterializationStrategy,
    facts: &MaterializationFacts,
) -> bool {
    match strategy {
        MaterializationStrategy::Symlink => facts.repo_entry_kind == RepoEntryKind::ManagedSymlink,
        MaterializationStrategy::Copy => {
            facts.repo_entry_kind == RepoEntryKind::RegularFile
                && facts.hardlink_free
                && facts.copy_content == CopyContentState::Equal
        }
    }
}

fn dry_run_outcome(action: ItemMaterializeAction) -> MaterializeOutcome {
    match action {
        ItemMaterializeAction::AlreadyMaterialized => MaterializeOutcome::AlreadyMaterialized,
        ItemMaterializeAction::Replace { .. } => MaterializeOutcome::WouldMaterialize,
    }
}
