use std::{fs, path::Path};

pub use crate::{
    context::{ReadOnlyRepoContext, RepoContext},
    error::AppError,
    ops::{
        add::{DirItemOutcome, DirectoryAddResult, SkipReason},
        info::ItemInfo,
        relink::{ItemRelinkRequest, RelinkDirection, RelinkOutcome},
        restore::{NamespaceRestoreResult, NsRestoreItemOutcome},
        status::{
            CopyContentState, ItemStatus, ItemStatusV2, MaterializationStrategy,
            ObservedMaterialization, StatusIssue, StatusIssueCode, StatusNote, StatusNoteCode,
            StatusOptions, StatusSchemaVersion, StatusSeverity, STATUS_SCHEMA_VERSION_V2,
        },
    },
    plan::{
        item_add::{ItemAddPlan, ItemAddReport},
        item_materialize::{
            ItemMaterializeAction, ItemMaterializePlan, ItemMaterializeReport,
            ItemMaterializeRequest, MaterializeOutcome,
        },
        item_move::{ItemMovePlan, ItemMoveReport, ItemMoveWarning},
        item_relink::{ItemRelinkPlan, ItemRelinkReport},
        item_repair::{ItemRepairReport, RepairOutcome},
        item_restore::{ItemRestoreAction, ItemRestorePlan, ItemRestoreReport},
        item_sync::{
            ItemSyncAction, ItemSyncPlan, ItemSyncReport, ItemSyncRequest, SyncDirection,
            SyncOutcome,
        },
        repo_repair::{RepoRepairAction, RepoRepairSymlinkAction},
    },
    store::manifest::Item,
};

use crate::{
    context,
    error::Result,
    fs::{
        canonical_transfer::DefaultCanonicalTransfer, materializer::DefaultMaterializer,
        DefaultLinkStrategy,
    },
    git::exclude::{GitInfoExclude, GitInfoExcludeSession, IgnoreBackend},
    ops::{
        add, info as info_ops, list as list_ops, materialize as materialize_ops,
        move_item as move_item_ops, path as path_ops, relink as relink_ops, repair as repair_ops,
        restore, status as status_ops, sync as sync_ops,
    },
};

/// Reusable ports for all add and restore operations in one top-level command.
///
/// The session is bound to the repository and store that constructed its ports. Every session-aware operation rejects a different context before it plans or writes anything.
pub struct ItemOperationSession {
    scope: ItemOperationSessionScope,
    materializer: DefaultMaterializer,
    transfer: DefaultCanonicalTransfer,
    ignore: GitInfoExcludeSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ItemOperationSessionScope {
    repo_root: std::path::PathBuf,
    store_root: std::path::PathBuf,
    repo_store: std::path::PathBuf,
    repo_id: String,
}

impl ItemOperationSession {
    pub fn new(ctx: &RepoContext) -> Self {
        Self {
            scope: ItemOperationSessionScope {
                repo_root: ctx.repo_root.clone(),
                store_root: ctx.config.store.clone(),
                repo_store: ctx.repo_store.clone(),
                repo_id: ctx.repo_id.clone(),
            },
            materializer: DefaultMaterializer::new(ctx.repo_root.clone(), ctx.config.store.clone()),
            transfer: DefaultCanonicalTransfer::new(
                ctx.repo_root.clone(),
                ctx.config.store.clone(),
            ),
            ignore: GitInfoExclude::session(&ctx.repo_root),
        }
    }

    fn ensure_context(&self, ctx: &RepoContext) -> Result<()> {
        let matches = self.scope.repo_root == ctx.repo_root
            && self.scope.store_root == ctx.config.store
            && self.scope.repo_store == ctx.repo_store
            && self.scope.repo_id == ctx.repo_id;
        if matches {
            Ok(())
        } else {
            Err(AppError::ItemOperationSessionContextMismatch)
        }
    }
}

pub fn build_create_or_load(cwd: &Path, store_override: Option<&Path>) -> Result<RepoContext> {
    context::build_create_or_load(cwd, store_override)
}

/// Performs the strict durability gate before a named item mutation starts.
/// This is side-effect-free and lets a CLI name the requested command in the
/// actionable error rather than a generic context-construction operation.
pub fn preflight_mutation_durability(store_override: Option<&Path>, operation: &str) -> Result<()> {
    context::preflight_mutation_durability_from_config(store_override, operation)
}

pub fn build_preview_create_or_load(
    cwd: &Path,
    store_override: Option<&Path>,
) -> Result<RepoContext> {
    context::build_preview_create_or_load(cwd, store_override)
}

pub fn build_read_only(cwd: &Path, store_override: Option<&Path>) -> Result<ReadOnlyRepoContext> {
    context::build_read_only(cwd, store_override)
}

pub fn add_file(ctx: &mut RepoContext, abs_path: &Path, dry_run: bool) -> Result<ItemAddReport> {
    let mut session = ItemOperationSession::new(ctx);
    add_file_with_session(ctx, abs_path, dry_run, &mut session)
}

/// Adds one file using ports shared by the current top-level command.
pub fn add_file_with_session(
    ctx: &mut RepoContext,
    abs_path: &Path,
    dry_run: bool,
    session: &mut ItemOperationSession,
) -> Result<ItemAddReport> {
    session.ensure_context(ctx)?;
    add::add_report(
        ctx,
        abs_path,
        dry_run,
        &mut session.materializer,
        &mut session.transfer,
        &session.ignore,
    )
}

pub fn add_directory(
    ctx: &mut RepoContext,
    abs_path: &Path,
    dry_run: bool,
) -> Result<DirectoryAddResult> {
    let mut session = ItemOperationSession::new(ctx);
    add_directory_with_session(ctx, abs_path, dry_run, &mut session)
}

/// Adds a directory using ports shared by the current top-level command.
pub fn add_directory_with_session(
    ctx: &mut RepoContext,
    abs_path: &Path,
    dry_run: bool,
    session: &mut ItemOperationSession,
) -> Result<DirectoryAddResult> {
    session.ensure_context(ctx)?;
    add::add_directory(
        ctx,
        abs_path,
        dry_run,
        &mut session.materializer,
        &mut session.transfer,
        &session.ignore,
    )
}

pub fn restore_file(
    ctx: &mut RepoContext,
    abs_path: &Path,
    dry_run: bool,
    keep_ignore: bool,
    keep_store: bool,
) -> Result<ItemRestoreReport> {
    let mut session = ItemOperationSession::new(ctx);
    restore_file_with_session(
        ctx,
        abs_path,
        dry_run,
        keep_ignore,
        keep_store,
        &mut session,
    )
}

/// Restores one file using ports shared by the current top-level command.
pub fn restore_file_with_session(
    ctx: &mut RepoContext,
    abs_path: &Path,
    dry_run: bool,
    keep_ignore: bool,
    keep_store: bool,
    session: &mut ItemOperationSession,
) -> Result<ItemRestoreReport> {
    session.ensure_context(ctx)?;
    let mut ports = restore::RestorePorts {
        materializer: &mut session.materializer,
        transfer: &mut session.transfer,
    };
    restore::restore(
        ctx,
        abs_path,
        dry_run,
        keep_ignore,
        keep_store,
        &mut ports,
        &session.ignore,
    )
}

pub fn restore_namespace(
    ctx: &mut RepoContext,
    ns_path: &str,
    dry_run: bool,
    keep_ignore: bool,
    keep_store: bool,
) -> Result<NamespaceRestoreResult> {
    let mut session = ItemOperationSession::new(ctx);
    restore_namespace_with_session(ctx, ns_path, dry_run, keep_ignore, keep_store, &mut session)
}

/// Restores a namespace using ports shared by the current top-level command.
pub fn restore_namespace_with_session(
    ctx: &mut RepoContext,
    ns_path: &str,
    dry_run: bool,
    keep_ignore: bool,
    keep_store: bool,
    session: &mut ItemOperationSession,
) -> Result<NamespaceRestoreResult> {
    session.ensure_context(ctx)?;
    let mut ports = restore::RestorePorts {
        materializer: &mut session.materializer,
        transfer: &mut session.transfer,
    };
    restore::restore_namespace(
        ctx,
        ns_path,
        dry_run,
        keep_ignore,
        keep_store,
        &mut ports,
        &session.ignore,
    )
}

pub fn list(ctx: &RepoContext) -> &[Item] {
    list_ops::list(ctx)
}

pub fn status(ctx: &RepoContext) -> Result<Vec<ItemStatus>> {
    let link = DefaultLinkStrategy;
    let ignore = GitInfoExclude;
    status_ops::status(ctx, &link, &ignore)
}

pub fn status_v2(ctx: &RepoContext, options: StatusOptions) -> Result<Vec<ItemStatusV2>> {
    let link = DefaultLinkStrategy;
    let ignore = GitInfoExclude;
    status_ops::status_v2(ctx, &link, &ignore, options)
}

pub fn repair(
    ctx: &RepoContext,
    abs_path: &Path,
    dry_run: bool,
    force: bool,
) -> Result<ItemRepairReport> {
    let link = DefaultLinkStrategy;
    repair_ops::repair_report(ctx, abs_path, &link, dry_run, force)
}

/// Explicitly converts one existing healthy materialization to `strategy`.
/// This never changes the item manifest or ownership state.
pub fn materialize(
    ctx: &RepoContext,
    abs_path: &Path,
    request: ItemMaterializeRequest,
) -> Result<ItemMaterializeReport> {
    let ignore = GitInfoExclude;
    materialize_ops::materialize_report(ctx, abs_path, request, &ignore)
}

/// Explicitly synchronizes one attached regular Copy in the requested
/// direction. Repository-to-store replacement requires `confirmed` unless
/// the request is a dry run or content is already equal.
pub fn sync(
    ctx: &mut RepoContext,
    abs_path: &Path,
    request: ItemSyncRequest,
) -> Result<ItemSyncReport> {
    let ignore = GitInfoExclude;
    sync_ops::sync_report(ctx, abs_path, request, &ignore)
}

pub fn relink(ctx: &mut RepoContext, abs_path: &Path, dry_run: bool) -> Result<ItemRelinkReport> {
    let link = DefaultLinkStrategy;
    relink_ops::relink_report(ctx, abs_path, dry_run, &link)
}

/// Re-attaches a detached item, optionally resolving a diverged regular Copy
/// in one explicit durable direction.
pub fn relink_with_request(
    ctx: &mut RepoContext,
    abs_path: &Path,
    request: ItemRelinkRequest,
) -> Result<ItemRelinkReport> {
    let ignore = GitInfoExclude;
    relink_ops::relink_report_with_request(ctx, abs_path, request, &ignore)
}

pub fn move_item(
    ctx: &mut RepoContext,
    old_abs: &Path,
    new_abs: &Path,
    dry_run: bool,
) -> Result<ItemMoveReport> {
    let link = DefaultLinkStrategy;
    let ignore = GitInfoExclude;
    move_item_ops::move_item(ctx, old_abs, new_abs, dry_run, &link, &ignore)
}

pub fn info(ctx: &RepoContext, abs_path: &Path) -> Result<ItemInfo> {
    let link = DefaultLinkStrategy;
    let ignore = GitInfoExclude;
    info_ops::info(ctx, abs_path, &link, &ignore)
}

pub fn info_read_only(read_only: &ReadOnlyRepoContext, abs_path: &Path) -> Result<ItemInfo> {
    if let Some(ctx) = &read_only.repo {
        return info(ctx, abs_path);
    }

    let rel_str = path_ops::repo_relative_string(&read_only.current.repo_root, abs_path)?;
    let ignore = GitInfoExclude;
    Ok(ItemInfo {
        path: rel_str.clone(),
        repo_root: read_only.current.repo_root.clone(),
        store_path: None,
        link_target: fs::read_link(abs_path).ok(),
        symlink_ok: false,
        tracked: false,
        in_exclude: ignore.has_entry(&read_only.current.repo_root, &rel_str)?,
    })
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use tempfile::TempDir;

    use super::*;
    use crate::{
        context,
        domain::materialization::MaterializationStrategy,
        git::exclude::{GitInfoExclude, IgnoreBackend},
    };

    fn init_git_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        for args in [
            ["init", "-b", "main"].as_slice(),
            ["config", "user.email", "test@example.com"].as_slice(),
            ["config", "user.name", "Test User"].as_slice(),
        ] {
            let output = Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap_or_else(|error| panic!("failed to spawn git {}: {error}", args[0]));
            assert!(
                output.status.success(),
                "git {} failed: {}",
                args[0],
                String::from_utf8_lossy(&output.stderr)
            );
        }
        dir
    }

    fn copy_context(repo: &Path, store: &Path) -> RepoContext {
        let mut ctx = context::build_create_or_load(repo, Some(store)).unwrap();
        ctx.config.materialization = MaterializationStrategy::Copy;
        ctx
    }

    #[test]
    fn session_rejects_a_different_context_before_any_mutation() {
        let repo_a = init_git_repo();
        let store_a = TempDir::new().unwrap();
        let ctx_a = copy_context(repo_a.path(), store_a.path());
        let mut session = ItemOperationSession::new(&ctx_a);

        let repo_b = init_git_repo();
        let store_b = TempDir::new().unwrap();
        let file_b = repo_b.path().join("secret.txt");
        let directory_b = repo_b.path().join("secrets");
        std::fs::create_dir(&directory_b).unwrap();
        std::fs::write(&file_b, "source stays put").unwrap();
        std::fs::write(directory_b.join("nested.txt"), "nested source stays put").unwrap();
        let mut ctx_b = copy_context(repo_b.path(), store_b.path());

        assert!(matches!(
            add_file_with_session(&mut ctx_b, &file_b, false, &mut session),
            Err(AppError::ItemOperationSessionContextMismatch)
        ));
        assert!(matches!(
            add_directory_with_session(&mut ctx_b, &directory_b, false, &mut session),
            Err(AppError::ItemOperationSessionContextMismatch)
        ));
        assert!(matches!(
            restore_file_with_session(&mut ctx_b, &file_b, false, false, false, &mut session),
            Err(AppError::ItemOperationSessionContextMismatch)
        ));
        assert!(matches!(
            restore_namespace_with_session(&mut ctx_b, "", false, false, false, &mut session),
            Err(AppError::ItemOperationSessionContextMismatch)
        ));

        assert!(ctx_a.manifest.items.is_empty());
        assert!(ctx_b.manifest.items.is_empty());
        assert_eq!(
            std::fs::read_to_string(&file_b).unwrap(),
            "source stays put"
        );
        assert_eq!(
            std::fs::read_to_string(directory_b.join("nested.txt")).unwrap(),
            "nested source stays put"
        );
        assert!(!ctx_a.store_path_for("secret.txt").exists());
        assert!(!ctx_b.store_path_for("secret.txt").exists());
        assert!(!GitInfoExclude
            .has_entry(&ctx_b.repo_root, "secret.txt")
            .unwrap());
    }

    #[test]
    fn session_reuses_ports_for_multiple_file_adds_and_restore_all() {
        let repo = init_git_repo();
        let store = TempDir::new().unwrap();
        let first = repo.path().join("first.txt");
        let second = repo.path().join("second.txt");
        std::fs::write(&first, "first").unwrap();
        std::fs::write(&second, "second").unwrap();
        let mut ctx = copy_context(repo.path(), store.path());
        let mut session = ItemOperationSession::new(&ctx);

        add_file_with_session(&mut ctx, &first, false, &mut session).unwrap();
        add_file_with_session(&mut ctx, &second, false, &mut session).unwrap();
        assert_eq!(ctx.manifest.items.len(), 2);

        restore_file_with_session(&mut ctx, &first, false, false, false, &mut session).unwrap();
        let result =
            restore_namespace_with_session(&mut ctx, "", false, false, false, &mut session)
                .unwrap();

        assert_eq!(result.results.len(), 1);
        assert!(ctx.manifest.items.is_empty());
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "first");
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "second");
        assert!(!GitInfoExclude
            .has_entry(&ctx.repo_root, "first.txt")
            .unwrap());
        assert!(!GitInfoExclude
            .has_entry(&ctx.repo_root, "second.txt")
            .unwrap());
    }

    #[test]
    fn session_reuses_ports_for_directory_add_and_namespace_restore() {
        let repo = init_git_repo();
        let store = TempDir::new().unwrap();
        let namespace = repo.path().join("secrets");
        std::fs::create_dir_all(namespace.join("nested")).unwrap();
        std::fs::write(namespace.join("first.txt"), "first").unwrap();
        std::fs::write(namespace.join("nested/second.txt"), "second").unwrap();
        let mut ctx = copy_context(repo.path(), store.path());
        let mut session = ItemOperationSession::new(&ctx);

        let add_result =
            add_directory_with_session(&mut ctx, &namespace, false, &mut session).unwrap();
        assert_eq!(add_result.results.len(), 2);
        let restore_result =
            restore_namespace_with_session(&mut ctx, "secrets/", false, false, false, &mut session)
                .unwrap();

        assert_eq!(restore_result.results.len(), 2);
        assert!(ctx.manifest.items.is_empty());
        assert_eq!(
            std::fs::read_to_string(namespace.join("nested/second.txt")).unwrap(),
            "second"
        );
    }

    #[test]
    fn session_rereads_exclude_and_fails_closed_between_operations() {
        let repo = init_git_repo();
        let store = TempDir::new().unwrap();
        let path = repo.path().join("guarded.txt");
        std::fs::write(&path, "protected content").unwrap();
        let mut ctx = copy_context(repo.path(), store.path());
        let mut session = ItemOperationSession::new(&ctx);

        add_file_with_session(&mut ctx, &path, false, &mut session).unwrap();
        let store_path = ctx.store_path_for("guarded.txt");
        GitInfoExclude
            .remove_entries(&ctx.repo_root, &["guarded.txt"])
            .unwrap();

        assert!(matches!(
            restore_file_with_session(&mut ctx, &path, false, false, false, &mut session),
            Err(AppError::Internal(message)) if message.contains("exclude is missing")
        ));
        assert!(ctx.manifest.contains("guarded.txt"));
        assert!(store_path.is_file());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "protected content");
    }
}
