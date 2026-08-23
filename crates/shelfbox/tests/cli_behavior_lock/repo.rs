use serde_json::Value;
use tempfile::TempDir;

use crate::common::{self, snapshot_tree, CliFixture};

use super::{assert_absent, single_repo_id, single_repo_store_dir, store_args, store_args_slice};

#[test]
fn repo_status_exit_codes_are_locked_for_associated_repo() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item_path = repo.path().join("repo-secret.txt");
    std::fs::write(&item_path, "secret").unwrap();

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "add", "repo-secret.txt"]),
    );
    output.assert_success();
    assert_eq!(output.stderr, "");

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["repo", "status", "--format", "plain"]),
    );
    output.assert_code(0);
    assert_eq!(output.stdout, "OK repo-secret.txt\n");
    assert_eq!(output.stderr, "");

    std::fs::write(repo.path().join(".git").join("info").join("exclude"), "").unwrap();
    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["repo", "status", "--format", "plain"]),
    );
    output.assert_code(1);
    assert_eq!(output.stdout, "WARN repo-secret.txt\n");
    assert_eq!(output.stderr, "");

    std::fs::remove_file(&item_path).unwrap();
    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["repo", "status", "--format", "plain"]),
    );
    output.assert_code(2);
    assert_eq!(output.stdout, "ERROR repo-secret.txt\n");
    assert_eq!(output.stderr, "");
}

#[test]
fn repo_status_for_unassociated_repo_does_not_initialize_store() {
    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let before = snapshot_tree(store.path());

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["repo", "status", "--format", "plain"]),
    );

    output.assert_code(0);
    assert_eq!(output.stdout, "");
    assert_eq!(output.stderr, "");
    assert_eq!(snapshot_tree(store.path()), before);
    assert_absent(store.path(), "meta.json");
    assert_absent(store.path(), "index.json");
    assert_absent(store.path(), "repos");
    assert_absent(store.path(), ".lock");

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["doctor", "--format", "plain"]),
    );

    output.assert_code(0);
    assert_eq!(output.stdout, "");
    assert_eq!(output.stderr, "");
    assert_eq!(snapshot_tree(store.path()), before);
    assert_absent(store.path(), "meta.json");
    assert_absent(store.path(), "index.json");
    assert_absent(store.path(), "repos");
    assert_absent(store.path(), ".lock");
}
#[test]
fn repo_repair_for_unassociated_repo_refuses_without_initializing_store() {
    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let before = snapshot_tree(store.path());

    let output = fixture.run(repo.path(), store_args(store.path(), ["repo", "repair"]));

    output.assert_code(255);
    assert!(output.stderr.contains("Run `shelfbox repo reclaim` first"));
    assert_eq!(output.stdout, "");
    assert_eq!(snapshot_tree(store.path()), before);
    assert_absent(store.path(), "meta.json");
    assert_absent(store.path(), "index.json");
    assert_absent(store.path(), "repos");
    assert_absent(store.path(), ".lock");
}

#[test]
fn repo_repair_dry_run_reports_plan_without_writing() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item_path = repo.path().join("repo-dry.txt");
    std::fs::write(&item_path, "secret").unwrap();

    fixture
        .run(
            repo.path(),
            store_args(store.path(), ["item", "add", "repo-dry.txt"]),
        )
        .assert_success();
    let repo_store = store
        .path()
        .join("repos")
        .join(single_repo_store_dir(store.path()));
    std::fs::remove_file(&item_path).unwrap();
    let repo_before = snapshot_tree(repo.path());
    let store_before = snapshot_tree(store.path());

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["repo", "repair", "--dry-run"]),
    );

    output.assert_success();
    assert_eq!(output.stderr, "");
    let normalized = common::normalize_output(
        &output.stdout,
        &[(&repo_store, "<repo-store>"), (repo.path(), "<repo>")],
    );
    assert_eq!(
        normalized,
        concat!(
            "[dry-run] repair 'repo-dry.txt'\n",
            "  recreate symlink <repo>/repo-dry.txt → <repo-store>/items/repo-dry.txt\n",
            "repo repair:\n",
            "  materializations would repair: 1\n",
            "  materializations already healthy: 0\n",
            "  materializations failed: 0\n",
            "  exclude: already current\n",
            "  index: already current\n",
            "  identity hints: would update\n"
        )
    );
    assert_eq!(snapshot_tree(repo.path()), repo_before);
    assert_eq!(snapshot_tree(store.path()), store_before);
}

#[test]
fn repo_reclaim_explicit_target_updates_association_without_repairing_symlinks() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let original = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item_path = original.path().join("secret.txt");
    std::fs::write(&item_path, "secret").unwrap();

    fixture
        .run(
            original.path(),
            store_args(store.path(), ["item", "add", "secret.txt"]),
        )
        .assert_success();
    let repo_id = single_repo_id(store.path());

    let reclone = common::init_git_repo();
    let output = fixture.run(
        reclone.path(),
        store_args_slice(
            store.path(),
            &["repo", "reclaim", "--repo-id", repo_id.as_str()],
        ),
    );

    output.assert_success();
    assert_eq!(
        output.stdout,
        format!(
            "Associated with {repo_id}. Run `shelfbox repo repair` to restore materializations.\n"
        )
    );
    assert_eq!(output.stderr, "");
    assert!(
        !reclone.path().join("secret.txt").exists(),
        "repo reclaim must not repair repo-side materializations"
    );

    let index_json: Value =
        serde_json::from_str(&std::fs::read_to_string(store.path().join("index.json")).unwrap())
            .unwrap();
    let repo_entry = &index_json["repos"][repo_id.as_str()];
    assert_eq!(
        common::normalize_path_text(repo_entry["root"].as_str().unwrap()),
        common::normalize_path_value(reclone.path())
    );
    let repo_store_dir = repo_entry["repo_store_dir"]
        .as_str()
        .expect("repo entry should record repo_store_dir");
    assert!(store
        .path()
        .join("repos")
        .join(repo_store_dir)
        .join("items")
        .join("secret.txt")
        .exists());
}
