use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    context::RepoContext,
    domain::path::StoreRelativePath,
    error::{AppError, Result},
    fs::materializer::{
        InspectionPurpose, MaterializationInspectionRequest, MaterializationLocation, Materializer,
    },
    git::exclude::IgnoreBackend,
};

use super::path::repo_relative_string;

const INFO_INSPECTION_STORE_PATH: &str = ".shelfbox-info-inspection";

/// Diagnostic metadata for a single shelved item.
///
/// Returned by item info operations and intended for use as a debugging /
/// scripting API.
/// Every field is always populated regardless of the item's current state,
/// making this the canonical source for "why does this item look broken?"
/// diagnostics.
#[derive(Debug, Serialize)]
pub struct ItemInfo {
    /// Repo-relative path of the item (forward slashes, no leading `/`).
    pub path: String,

    /// Absolute path to the repository root.
    pub repo_root: PathBuf,

    /// Absolute path to the store-side file.
    /// `None` if the item is not in the manifest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_path: Option<PathBuf>,

    /// Target returned by `readlink(2)` at the repo path.
    /// `None` if no symlink exists at that path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_target: Option<PathBuf>,

    /// `true` if a symlink exists at the repo path *and* its target matches
    /// `store_path` exactly.
    pub symlink_ok: bool,

    /// `true` if the item appears in the manifest.
    pub tracked: bool,

    /// `true` if the path appears in `.git/info/exclude`.
    pub in_exclude: bool,
}

/// Returns diagnostic metadata for the item at `abs_path`.
///
/// `abs_path` must be absolute and located under `ctx.repo_root`.
/// Returns [`crate::error::AppError::PathOutsideRepo`] if it is not.
///
pub fn info(
    ctx: &RepoContext,
    abs_path: &Path,
    materializer: &dyn Materializer,
    ignore: &dyn IgnoreBackend,
) -> Result<ItemInfo> {
    // Convert abs_path → repo-relative string (forward slashes).
    let rel_str = repo_relative_string(&ctx.repo_root, abs_path)?;

    // Look up the manifest entry.
    let manifest_item = ctx.manifest.items.iter().find(|item| item.path == rel_str);

    // Resolve the absolute store path from the manifest entry.
    let store_path = manifest_item.map(|item| ctx.repo_store.join(&item.store_path));
    let store_relative = manifest_item
        .map(|item| item.store_path.parse())
        .transpose()
        .map_err(|_| AppError::Internal("invalid manifest store path".into()))?
        .unwrap_or_else(info_inspection_store_path);

    info_with_location(
        &ctx.repo_root,
        rel_str,
        store_path,
        store_relative,
        materializer,
        ignore,
    )
}

/// Returns diagnostic metadata when no managed repository context exists.
///
/// The dedicated inspection store path exists only to satisfy the typed
/// materializer location. It is never read as a managed item or mutated.
pub(crate) fn info_without_manifest(
    repo_root: &Path,
    abs_path: &Path,
    materializer: &dyn Materializer,
    ignore: &dyn IgnoreBackend,
) -> Result<ItemInfo> {
    let rel_str = repo_relative_string(repo_root, abs_path)?;
    info_with_location(
        repo_root,
        rel_str,
        None,
        info_inspection_store_path(),
        materializer,
        ignore,
    )
}

fn info_with_location(
    repo_root: &Path,
    rel_str: String,
    store_path: Option<PathBuf>,
    store_relative: StoreRelativePath,
    materializer: &dyn Materializer,
    ignore: &dyn IgnoreBackend,
) -> Result<ItemInfo> {
    let tracked = store_path.is_some();
    let repo_relative = rel_str
        .parse()
        .map_err(|_| AppError::Internal("invalid repository-relative item path".into()))?;
    let inspection = materializer.inspect(MaterializationInspectionRequest {
        location: MaterializationLocation::new(repo_relative, store_relative),
        purpose: InspectionPurpose::Planning,
    })?;

    // Materializer inspection preserves the immediate target spelling for diagnostics while keeping platform-specific link access inside `fs`.
    let link_target = inspection.symlink_target;

    // A symlink is healthy when it points exactly at the expected store path.
    let symlink_ok = match (&link_target, &store_path) {
        (Some(target), Some(expected)) => target == expected,
        _ => false,
    };

    // Check .git/info/exclude membership.
    let in_exclude = ignore.has_entry(repo_root, &rel_str)?;

    Ok(ItemInfo {
        path: rel_str,
        repo_root: repo_root.to_path_buf(),
        store_path,
        link_target,
        symlink_ok,
        tracked,
        in_exclude,
    })
}

fn info_inspection_store_path() -> StoreRelativePath {
    INFO_INSPECTION_STORE_PATH
        .parse()
        .expect("info inspection store path must be normalized")
}
