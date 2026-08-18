//! Repository-wide orchestration for explicit item synchronization.
//!
//! This module intentionally contains no content policy of its own. It
//! validates the complete attached-item target set with the item-level plan,
//! then executes those exact plans in lexical path order. Runtime failures
//! halt the batch and are returned as structured partial progress.

use crate::{
    context::RepoContext,
    domain::ownership::OwnershipState,
    error::{AppError, Result},
    git::exclude::IgnoreBackend,
    plan::{
        item_sync::{ItemSyncAction, SyncDirection},
        repo_sync::{RepoSyncItemReport, RepoSyncPlan, RepoSyncReport, RepoSyncRequest},
    },
};

use super::sync;

pub(crate) fn sync_repo_report(
    ctx: &mut RepoContext,
    request: RepoSyncRequest,
    ignore: &dyn IgnoreBackend,
) -> Result<RepoSyncReport> {
    let plan = build_repo_sync_plan(ctx, request.direction, ignore)?;
    if !request.dry_run && requires_confirmation(request.direction, &plan) && !request.confirmed {
        return Err(AppError::SyncConfirmationRequired);
    }

    if request.dry_run {
        let items = plan
            .items
            .iter()
            .map(|item| RepoSyncItemReport {
                path: item.path.clone(),
                outcome: Some(sync::dry_run_outcome(item.action)),
                error: None,
            })
            .collect();
        return Ok(RepoSyncReport {
            plan,
            items,
            dry_run: true,
            halted: false,
        });
    }

    let mut items = Vec::with_capacity(plan.items.len());
    for item in &plan.items {
        match sync::execute_sync_plan(ctx, item, ignore) {
            Ok(outcome) => items.push(RepoSyncItemReport {
                path: item.path.clone(),
                outcome: Some(outcome),
                error: None,
            }),
            Err(error) => {
                items.push(RepoSyncItemReport {
                    path: item.path.clone(),
                    outcome: None,
                    error: Some(error.to_string()),
                });
                return Ok(RepoSyncReport {
                    plan,
                    items,
                    dry_run: false,
                    halted: true,
                });
            }
        }
    }

    Ok(RepoSyncReport {
        plan,
        items,
        dry_run: false,
        halted: false,
    })
}

fn build_repo_sync_plan(
    ctx: &RepoContext,
    direction: SyncDirection,
    ignore: &dyn IgnoreBackend,
) -> Result<RepoSyncPlan> {
    let mut paths: Vec<_> = ctx
        .manifest
        .items
        .iter()
        .filter(|item| item.ownership_state == OwnershipState::Attached)
        .map(|item| item.path.clone())
        .collect();
    paths.sort();

    // This loop is intentionally complete before execution begins. A single
    // stale/tracked/diverged target rejects the whole batch without allowing
    // a preceding path to write.
    let mut items = Vec::with_capacity(paths.len());
    for path in paths {
        let abs_path = ctx.repo_root.join(&path);
        items.push(sync::build_sync_plan(ctx, &abs_path, direction, ignore)?);
    }

    Ok(RepoSyncPlan { direction, items })
}

fn requires_confirmation(direction: SyncDirection, plan: &RepoSyncPlan) -> bool {
    match direction {
        SyncDirection::FromStore => plan
            .items
            .iter()
            .any(|item| item.action == ItemSyncAction::ReplaceRepoFromStore),
        SyncDirection::FromRepo => plan
            .items
            .iter()
            .any(|item| item.action == ItemSyncAction::ReplaceStoreFromRepo),
    }
}
