use serde_json::Value;
use tempfile::TempDir;

use crate::common::{self, snapshot_tree, CliFixture};

use super::{assert_absent, single_repo_store_dir, store_args};

#[test]
fn item_add_dry_run_does_not_initialize_store() {
    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item_path = repo.path().join("dry.txt");
    std::fs::write(&item_path, "dry-run").unwrap();
    let repo_before = snapshot_tree(repo.path());
    let store_before = snapshot_tree(store.path());

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "add", "dry.txt", "--dry-run"]),
    );

    output.assert_success();
    assert_eq!(output.stderr, "");
    assert!(output.stdout.contains("[dry-run] shelve 'dry.txt'\n"));
    assert!(output.stdout.contains("  exclude dry.txt\n"));
    assert_eq!(snapshot_tree(repo.path()), repo_before);
    assert_eq!(snapshot_tree(store.path()), store_before);
    assert_absent(store.path(), "meta.json");
    assert_absent(store.path(), "index.json");
    assert_absent(store.path(), "repos");
    assert_absent(store.path(), ".lock");
}

#[test]
fn item_restore_keep_store_dry_run_reports_plan_without_writing() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item_path = repo.path().join("secret.txt");
    std::fs::write(&item_path, "secret").unwrap();

    fixture
        .run(
            repo.path(),
            store_args(store.path(), ["item", "add", "secret.txt"]),
        )
        .assert_success();
    let repo_before = snapshot_tree(repo.path());
    let store_before = snapshot_tree(store.path());

    let output = fixture.run(
        repo.path(),
        store_args(
            store.path(),
            ["item", "restore", "secret.txt", "--dry-run", "--keep-store"],
        ),
    );

    output.assert_success();
    assert_eq!(output.stderr, "");
    assert_eq!(
        output.stdout,
        concat!(
            "[dry-run] detach managed item 'secret.txt'\n",
            "  ownership_state: attached -> detached\n",
            "  (observed materialization, store item, manifest, and exclude are retained)\n"
        )
    );
    assert_eq!(snapshot_tree(repo.path()), repo_before);
    assert_eq!(snapshot_tree(store.path()), store_before);
}

#[test]
fn item_restore_all_restores_every_managed_item() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let first = repo.path().join("first.env");
    let second = repo.path().join("nested/second.env");
    std::fs::create_dir_all(second.parent().unwrap()).unwrap();
    std::fs::write(&first, "first").unwrap();
    std::fs::write(&second, "second").unwrap();

    fixture
        .run(
            repo.path(),
            store_args(store.path(), ["item", "add", "first.env"]),
        )
        .assert_success();
    fixture
        .run(
            repo.path(),
            store_args(store.path(), ["item", "add", "nested/second.env"]),
        )
        .assert_success();

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "restore", "--all"]),
    );

    output.assert_success();
    assert_eq!(output.stderr, "");
    assert_eq!(
        output.stdout,
        concat!(
            "all managed items: 2 restored, 0 failed\n",
            "  restored: first.env\n",
            "  restored: nested/second.env\n"
        )
    );
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "first");
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "second");

    let items = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "list", "--format", "json"]),
    );
    items.assert_success();
    assert_eq!(items.stdout, "[]\n");
}

#[test]
fn item_restore_all_dry_run_reports_the_complete_set_without_writing() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let first = repo.path().join("first.env");
    let second = repo.path().join("nested/second.env");
    std::fs::create_dir_all(second.parent().unwrap()).unwrap();
    std::fs::write(&first, "first").unwrap();
    std::fs::write(&second, "second").unwrap();

    for path in ["first.env", "nested/second.env"] {
        fixture
            .run(repo.path(), store_args(store.path(), ["item", "add", path]))
            .assert_success();
    }
    let repo_before = snapshot_tree(repo.path());
    let store_before = snapshot_tree(store.path());

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "restore", "--all", "--dry-run"]),
    );

    output.assert_success();
    assert_eq!(output.stderr, "");
    assert_eq!(
        output.stdout,
        concat!(
            "[dry-run] all managed items: 2 restored, 0 failed\n",
            "  [dry-run] restore: first.env\n",
            "  [dry-run] restore: nested/second.env\n"
        )
    );
    assert_eq!(snapshot_tree(repo.path()), repo_before);
    assert_eq!(snapshot_tree(store.path()), store_before);
}

#[test]
fn item_restore_all_rejects_paths_without_writing() {
    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item_path = repo.path().join("secret.txt");
    std::fs::write(&item_path, "secret").unwrap();
    let repo_before = snapshot_tree(repo.path());
    let store_before = snapshot_tree(store.path());

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "restore", "secret.txt", "--all"]),
    );

    assert!(!output.status.success());
    assert!(output.stderr.contains("cannot be used with"));
    assert_eq!(snapshot_tree(repo.path()), repo_before);
    assert_eq!(snapshot_tree(store.path()), store_before);
}

#[test]
fn item_restore_all_outside_a_repository_does_not_write() {
    let fixture = CliFixture::new();
    let cwd = TempDir::new().unwrap();
    let store = TempDir::new().unwrap();
    let cwd_before = snapshot_tree(cwd.path());
    let store_before = snapshot_tree(store.path());

    let output = fixture.run(
        cwd.path(),
        store_args(store.path(), ["item", "restore", "--all"]),
    );

    assert!(!output.status.success());
    assert_eq!(snapshot_tree(cwd.path()), cwd_before);
    assert_eq!(snapshot_tree(store.path()), store_before);
}

#[test]
fn item_restore_all_for_an_unassociated_repository_does_not_write() {
    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let repo_before = snapshot_tree(repo.path());
    let store_before = snapshot_tree(store.path());

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "restore", "--all"]),
    );

    assert!(!output.status.success());
    assert!(output.stderr.contains("shelfbox repo reclaim"));
    assert_eq!(snapshot_tree(repo.path()), repo_before);
    assert_eq!(snapshot_tree(store.path()), store_before);
}

#[test]
fn item_restore_all_reports_partial_failures_without_rolling_back() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let first = repo.path().join("first.env");
    let second = repo.path().join("second.env");
    std::fs::write(&first, "first").unwrap();
    std::fs::write(&second, "second").unwrap();

    for path in ["first.env", "second.env"] {
        fixture
            .run(repo.path(), store_args(store.path(), ["item", "add", path]))
            .assert_success();
    }
    let repo_store = store
        .path()
        .join("repos")
        .join(single_repo_store_dir(store.path()));
    std::fs::remove_file(repo_store.join("items/second.env")).unwrap();

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "restore", "--all"]),
    );

    output.assert_success();
    assert_eq!(
        output.stdout,
        concat!(
            "all managed items: 1 restored, 1 failed\n",
            "  restored: first.env\n"
        )
    );
    assert!(output
        .stderr
        .contains("fail: second.env: store item not found"));
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "first");
    assert!(first.symlink_metadata().unwrap().is_file());
    assert!(second.symlink_metadata().unwrap().file_type().is_symlink());

    let items = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "list", "--format", "json"]),
    );
    items.assert_success();
    let items: Vec<Value> = serde_json::from_str(&items.stdout).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["path"], "second.env");
}

#[test]
fn item_move_dry_run_reports_plan_without_writing() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item_path = repo.path().join("original.txt");
    std::fs::write(&item_path, "secret").unwrap();

    fixture
        .run(
            repo.path(),
            store_args(store.path(), ["item", "add", "original.txt"]),
        )
        .assert_success();
    let repo_store = store
        .path()
        .join("repos")
        .join(single_repo_store_dir(store.path()));
    let repo_before = snapshot_tree(repo.path());
    let store_before = snapshot_tree(store.path());

    let output = fixture.run(
        repo.path(),
        store_args(
            store.path(),
            ["item", "move", "original.txt", "renamed.txt", "--dry-run"],
        ),
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
            "[dry-run] move 'original.txt' → 'renamed.txt'\n",
            "  store   <repo-store>/items/original.txt → <repo-store>/items/renamed.txt\n",
            "  materialization <repo>/original.txt → <repo>/renamed.txt\n",
            "  manifest: update path and store_path\n",
            "  exclude:  remove 'original.txt', add 'renamed.txt'\n"
        )
    );
    assert_eq!(snapshot_tree(repo.path()), repo_before);
    assert_eq!(snapshot_tree(store.path()), store_before);
}

#[test]
fn item_status_exit_codes_are_locked_for_associated_repo() {
    if !common::require_symlink_support() {
        return;
    }

    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item_path = repo.path().join("secret.txt");
    std::fs::write(&item_path, "secret").unwrap();

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "add", "secret.txt"]),
    );
    output.assert_success();
    assert_eq!(
        common::normalize_output(&output.stdout, &[]),
        "shelved: secret.txt\n"
    );
    assert_eq!(output.stderr, "");

    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "status", "--format", "plain"]),
    );
    output.assert_code(0);
    assert_eq!(output.stdout, "OK secret.txt\n");
    assert_eq!(output.stderr, "");

    std::fs::write(repo.path().join(".git").join("info").join("exclude"), "").unwrap();
    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "status", "--format", "plain"]),
    );
    output.assert_code(1);
    assert_eq!(output.stdout, "WARN secret.txt not in exclude\n");
    assert_eq!(output.stderr, "");

    std::fs::remove_file(&item_path).unwrap();
    let output = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "status", "--format", "plain"]),
    );
    output.assert_code(2);
    assert_eq!(
        output.stdout,
        "ERROR secret.txt materialization missing,not in exclude\n"
    );
    assert_eq!(output.stderr, "");
}
