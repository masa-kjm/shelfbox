pub mod add;
pub(crate) mod add_recovery;
pub mod detect_transitions;
pub mod gc;
pub mod info;
pub mod integrity;
pub(crate) mod lifecycle_recovery;
pub mod list;
pub mod materialize;
pub mod migrate_manifest;
pub mod move_item;
pub(crate) mod path;
pub mod rebuild_index;
pub mod reclaim;
pub(crate) mod recovery;
pub mod relink;
pub mod repair;
pub mod repo_materialize;
pub mod repo_sync;
pub mod restore;
pub mod status;
pub mod store_verify;
pub mod sync;

#[cfg(test)]
mod materialization_boundary_tests;
