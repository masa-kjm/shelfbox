//! Repository-wide orchestration for explicit materialization conversion.
//!
//! The target set is the current repository's attached items. Detached items
//! retain independent lifecycle semantics and can be converted explicitly via
//! `item materialize`; they are never silently included in a repo-wide write.

use crate::{
    context::RepoContext,
    domain::ownership::OwnershipState,
    error::Result,
    git::exclude::IgnoreBackend,
    plan::{
        item_materialize::MaterializeOutcome,
        repo_materialize::{
            RepoMaterializeItemReport, RepoMaterializePlan, RepoMaterializeReport,
            RepoMaterializeRequest,
        },
    },
};

use super::materialize;

pub(crate) fn materialize_repo_report(
    ctx: &RepoContext,
    request: RepoMaterializeRequest,
    ignore: &dyn IgnoreBackend,
) -> Result<RepoMaterializeReport> {
    let plan = build_repo_materialize_plan(ctx, request, ignore)?;
    if request.dry_run {
        let items = plan
            .items
            .iter()
            .map(|item| RepoMaterializeItemReport {
                path: item.path.clone(),
                outcome: Some(match item.action {
                    crate::plan::item_materialize::ItemMaterializeAction::AlreadyMaterialized => {
                        MaterializeOutcome::AlreadyMaterialized
                    }
                    crate::plan::item_materialize::ItemMaterializeAction::Replace { .. } => {
                        MaterializeOutcome::WouldMaterialize
                    }
                }),
                error: None,
            })
            .collect();
        return Ok(RepoMaterializeReport {
            plan,
            items,
            dry_run: true,
            halted: false,
        });
    }

    let mut items = Vec::with_capacity(plan.items.len());
    for item in &plan.items {
        match materialize::execute_materialize_plan(ctx, item, ignore) {
            Ok(outcome) => items.push(RepoMaterializeItemReport {
                path: item.path.clone(),
                outcome: Some(outcome),
                error: None,
            }),
            Err(error) => {
                items.push(RepoMaterializeItemReport {
                    path: item.path.clone(),
                    outcome: None,
                    error: Some(error.to_string()),
                });
                return Ok(RepoMaterializeReport {
                    plan,
                    items,
                    dry_run: false,
                    halted: true,
                });
            }
        }
    }

    Ok(RepoMaterializeReport {
        plan,
        items,
        dry_run: false,
        halted: false,
    })
}

fn build_repo_materialize_plan(
    ctx: &RepoContext,
    request: RepoMaterializeRequest,
    ignore: &dyn IgnoreBackend,
) -> Result<RepoMaterializePlan> {
    let mut paths: Vec<_> = ctx
        .manifest
        .items
        .iter()
        .filter(|item| item.ownership_state == OwnershipState::Attached)
        .map(|item| item.path.clone())
        .collect();
    paths.sort();

    let mut items = Vec::with_capacity(paths.len());
    for path in paths {
        let abs_path = ctx.repo_root.join(&path);
        items.push(materialize::build_materialize_plan(
            ctx,
            &abs_path,
            request.strategy,
            ignore,
        )?);
    }

    Ok(RepoMaterializePlan {
        strategy: request.strategy,
        items,
    })
}
