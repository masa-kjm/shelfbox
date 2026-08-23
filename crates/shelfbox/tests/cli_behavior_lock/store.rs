use serde_json::Value;
use tempfile::TempDir;

use crate::common::{self, snapshot_tree, CliFixture};

use super::{
    single_repo_id, single_repo_store_dir, store_args, write_orphaned_v3_manifest,
    write_v2_manifest, write_v3_manifest, ITEM_ID, REPO_ID,
};

#[test]
fn store_rebuild_index_dry_run_reports_without_writing_index() {
    let fixture = CliFixture::new();
    let cwd = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    write_v3_manifest(store.path(), "project-a", REPO_ID);
    let before = snapshot_tree(store.path());

    let output = fixture.run(
        cwd.path(),
        store_args(store.path(), ["store", "rebuild-index", "--dry-run"]),
    );

    output.assert_success();
    assert_eq!(output.stderr, "");
    assert_eq!(
        output.stdout,
        "Dry run - no index written.\nWould rebuild index: 1 repositories, 0 warnings\n"
    );
    assert!(!store.path().join("index.json").exists());
    assert_eq!(snapshot_tree(store.path()), before);
}

#[test]
fn store_migrate_manifests_dry_run_reports_without_writing_manifest() {
    let fixture = CliFixture::new();
    let cwd = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    write_v2_manifest(store.path(), "my-project", REPO_ID, ITEM_ID, "stale");
    let before = snapshot_tree(store.path());

    let output = fixture.run(
        cwd.path(),
        store_args(store.path(), ["store", "migrate-manifests", "--dry-run"]),
    );

    output.assert_success();
    assert_eq!(output.stderr, "");
    assert_eq!(
        output.stdout,
        concat!(
            "Dry run - no manifests written.\n",
            "target manifest version: 3\n",
            "manifests converted: 1\n",
            "  v2 -> v3: 1\n",
            "manifests unchanged: 0\n",
            "skipped/failed: 0\n",
            "ownership mappings: stale -> unreachable: 1, adopted -> detached: 0\n",
            "namespace entries dropped: 1\n"
        )
    );
    assert_eq!(snapshot_tree(store.path()), before);
}

#[test]
fn store_gc_dry_run_reports_orphaned_items_without_writing() {
    let fixture = CliFixture::new();
    let cwd = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    write_orphaned_v3_manifest(store.path(), "project-a", REPO_ID, ITEM_ID);
    let before = snapshot_tree(store.path());

    let output = fixture.run(
        cwd.path(),
        store_args(store.path(), ["store", "gc", "--dry-run"]),
    );

    output.assert_success();
    assert_eq!(output.stderr, "");
    assert_eq!(
        output.stdout,
        concat!(
            "Orphaned items eligible for deletion:\n",
            "  repos/project-a/items/old.env [01JWPQ3VKGE93V9BDHAENVXFA5] - 11 B\n",
            "Total: 1 item(s), 11 B.\n",
            "Dry run - no changes made.\n"
        )
    );
    assert_eq!(snapshot_tree(store.path()), before);
}

#[test]
fn store_verify_exit_contract_keeps_warning_and_error_labels_distinct() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    std::fs::write(repo.path().join("secret.txt"), "secret").unwrap();
    fixture
        .run(
            repo.path(),
            store_args(store.path(), ["item", "add", "secret.txt"]),
        )
        .assert_success();

    let before_healthy_verify = snapshot_tree(store.path());
    let healthy = fixture.run(repo.path(), store_args(store.path(), ["store", "verify"]));
    healthy.assert_code(0);
    assert_eq!(healthy.stdout, "OK — no issues found.\n");
    assert_eq!(healthy.stderr, "");
    assert_eq!(snapshot_tree(store.path()), before_healthy_verify);

    let index_path = store.path().join("index.json");
    let mut index: Value = serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap())
        .expect("index should remain valid JSON");
    let repo_id = single_repo_id(store.path());
    index["repos"][&repo_id]["root"] = Value::String(
        store
            .path()
            .join("unavailable-local-repo")
            .display()
            .to_string(),
    );
    std::fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();
    let before_warning_verify = snapshot_tree(store.path());

    let warning = fixture.run(repo.path(), store_args(store.path(), ["store", "verify"]));
    warning.assert_code(2);
    assert_eq!(
        warning.stdout,
        "1 issue(s) found. Run `shelfbox repo repair` to fix.\n"
    );
    assert!(warning.stderr.contains("WARNING "));
    assert!(!warning.stderr.contains("ERROR "));
    assert_eq!(snapshot_tree(store.path()), before_warning_verify);

    let canonical_item = store
        .path()
        .join("repos")
        .join(single_repo_store_dir(store.path()))
        .join("items/secret.txt");
    std::fs::remove_file(&canonical_item).unwrap();
    let both_labels = fixture.run(repo.path(), store_args(store.path(), ["store", "verify"]));
    both_labels.assert_code(2);
    assert!(both_labels.stderr.contains("WARNING "));
    assert!(both_labels.stderr.contains("ERROR "));
}
