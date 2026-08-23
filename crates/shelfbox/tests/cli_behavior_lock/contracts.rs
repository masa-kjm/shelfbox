use serde_json::Value;
use tempfile::TempDir;

use crate::common::{self, snapshot_tree, CliFixture};

use super::{assert_absent, assert_json_golden, row_by_key, store_args, store_args_slice};

#[cfg(windows)]
#[test]
fn windows_require_fails_closed_and_best_effort_copy_add_is_opt_in() {
    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let strict_store = TempDir::new().unwrap();
    let source = repo.path().join("secret.txt");
    std::fs::write(&source, "windows secret").unwrap();
    let before = snapshot_tree(strict_store.path());

    let strict = fixture.run(
        repo.path(),
        store_args(strict_store.path(), ["item", "add", "secret.txt"]),
    );
    assert!(!strict.status.success());
    assert!(strict
        .stderr
        .contains("requires crash-safe directory durability"));
    assert!(strict
        .stderr
        .contains("config set mutation_durability best-effort"));
    assert_eq!(snapshot_tree(strict_store.path()), before);
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "windows secret");

    // This administrative write intentionally remains available while strict
    // shelf mutations are fail-closed.
    let set = fixture.run(
        repo.path(),
        ["config", "set", "mutation_durability", "best-effort"],
    );
    set.assert_success();
    fixture.write_config("mutation_durability = \"best-effort\"\nmaterialization = \"copy\"\n");

    let best_effort_store = TempDir::new().unwrap();
    let best_effort = fixture.run(
        repo.path(),
        store_args(best_effort_store.path(), ["item", "add", "secret.txt"]),
    );
    best_effort.assert_success();
    assert!(!best_effort
        .stderr
        .contains("best-effort mutation durability is active"));
    assert!(std::fs::symlink_metadata(&source).unwrap().is_file());
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "windows secret");
}

#[test]
fn copy_materialization_is_available_through_config_and_item_paths() {
    let fixture = CliFixture::new();
    let cwd = TempDir::new().unwrap();
    fixture.write_config("materialization = \"copy\"\n");

    let from_file = fixture.run(cwd.path(), ["config", "get", "materialization", "--source"]);
    from_file.assert_success();
    assert_eq!(from_file.stdout, "copy\nsource: config\n");
    assert_eq!(from_file.stderr, "");

    let list = fixture.run(cwd.path(), ["config", "list", "--format", "json"]);
    list.assert_success();
    let rows: Vec<Value> = serde_json::from_str(&list.stdout).unwrap();
    let materialization = row_by_key(&rows, "materialization");
    assert_eq!(materialization["type"], "enum");
    assert_eq!(materialization["default"], "symlink");
    assert_eq!(materialization["source"], "config");
    assert_eq!(materialization["current"], "copy");

    let explain = fixture.run(cwd.path(), ["config", "explain", "materialization"]);
    explain.assert_success();
    assert!(explain
        .stdout
        .contains("Default strategy for future materializations."));
    assert!(explain.stdout.contains("Valid values: symlink, copy."));

    let fresh_fixture = CliFixture::new();
    let from_set = fresh_fixture.run(cwd.path(), ["config", "set", "materialization", "copy"]);
    from_set.assert_success();
    assert_eq!(from_set.stdout, "set materialization = copy\n");
    assert_eq!(from_set.stderr, "");
    assert_eq!(
        std::fs::read_to_string(fresh_fixture.config_file_path()).unwrap(),
        "materialization = \"copy\"\n"
    );

    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let repo_item = repo.path().join("secret.txt");
    std::fs::write(&repo_item, "canonical").unwrap();
    fixture
        .run(
            repo.path(),
            store_args(store.path(), ["item", "add", "secret.txt"]),
        )
        .assert_success();
    assert!(
        !std::fs::symlink_metadata(&repo_item)
            .unwrap()
            .file_type()
            .is_symlink(),
        "copy mode must materialize a regular repository file"
    );
    assert_eq!(std::fs::read_to_string(&repo_item).unwrap(), "canonical");

    let healthy = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "status", "--format", "json"]),
    );
    healthy.assert_code(0);
    let statuses: Vec<Value> = serde_json::from_str(&healthy.stdout).unwrap();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0]["configured_strategy"], "copy");
    assert_eq!(statuses[0]["observed_materialization"], "regular_file");
    assert_eq!(statuses[0]["content_state"], "equal");
    assert!(statuses[0]["link_exists"].is_null());
    assert!(statuses[0]["link_valid"].is_null());

    std::fs::write(&repo_item, "local edit").unwrap();
    let diverged = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "status", "--format", "plain"]),
    );
    diverged.assert_code(2);
    assert_eq!(diverged.stdout, "ERROR secret.txt content diverged\n");

    let missing_direction = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "sync", "secret.txt"]),
    );
    missing_direction.assert_code(2);
    assert!(missing_direction.stderr.contains("--from <FROM>"));

    let missing_confirmation = fixture.run(
        repo.path(),
        store_args(
            store.path(),
            ["item", "sync", "secret.txt", "--from", "repo"],
        ),
    );
    assert!(!missing_confirmation.status.success());
    assert!(missing_confirmation.stderr.contains("--yes"));

    let sync = fixture.run(
        repo.path(),
        store_args(
            store.path(),
            ["item", "sync", "secret.txt", "--from", "store"],
        ),
    );
    sync.assert_success();
    assert_eq!(sync.stdout, "synchronized from store: secret.txt\n");
    assert_eq!(std::fs::read_to_string(&repo_item).unwrap(), "canonical");
}

#[test]
fn v091_materialize_and_repo_batch_dry_runs_are_explicit_and_mutation_free() {
    let fixture = CliFixture::new();
    fixture.write_config("materialization = \"copy\"\n");
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let item = repo.path().join("secret.txt");
    std::fs::write(&item, "canonical").unwrap();

    fixture
        .run(
            repo.path(),
            store_args(store.path(), ["item", "add", "secret.txt"]),
        )
        .assert_success();
    let before_store = snapshot_tree(store.path());

    let item_no_op = fixture.run(
        repo.path(),
        store_args(
            store.path(),
            [
                "item",
                "materialize",
                "secret.txt",
                "--strategy",
                "copy",
                "--dry-run",
            ],
        ),
    );
    item_no_op.assert_success();
    assert_eq!(
        item_no_op.stdout,
        "ok (already materialized as copy): secret.txt\n"
    );

    std::fs::write(&item, "local edit").unwrap();
    let repo_after_edit = snapshot_tree(repo.path());
    let sync_dry_run = fixture.run(
        repo.path(),
        store_args(
            store.path(),
            ["repo", "sync", "--from", "store", "--dry-run"],
        ),
    );
    sync_dry_run.assert_success();
    assert_eq!(
        sync_dry_run.stdout,
        "[dry-run] would synchronize from store: secret.txt\n"
    );
    assert_eq!(std::fs::read_to_string(&item).unwrap(), "local edit");

    let materialize_dry_run = fixture.run(
        repo.path(),
        store_args(
            store.path(),
            ["repo", "materialize", "--strategy", "copy", "--dry-run"],
        ),
    );
    materialize_dry_run.assert_success();
    assert_eq!(
        materialize_dry_run.stdout,
        "[dry-run] already materialized as copy: secret.txt\n"
    );
    assert_eq!(snapshot_tree(store.path()), before_store);
    assert_eq!(snapshot_tree(repo.path()), repo_after_edit);
    assert_eq!(std::fs::read_to_string(&item).unwrap(), "local edit");
}
#[test]
fn symlink_status_json_matches_v2_golden_fixtures() {
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

    let item_status = fixture.run(
        repo.path(),
        store_args(store.path(), ["item", "status", "--format", "json"]),
    );
    item_status.assert_code(0);
    assert_eq!(item_status.stderr, "");
    assert_json_golden(
        &item_status.stdout,
        include_str!("../../../shelfbox-core/tests/fixtures/item-status-symlink-v2.json"),
    );

    let repo_status = fixture.run(
        repo.path(),
        store_args(store.path(), ["repo", "status", "--format", "json"]),
    );
    repo_status.assert_code(0);
    assert_eq!(repo_status.stderr, "");
    assert_json_golden(
        &repo_status.stdout,
        include_str!("../../../shelfbox-core/tests/fixtures/repo-status-symlink-v2.json"),
    );
}

#[test]
fn read_only_item_and_repo_gc_commands_do_not_initialize_absent_store() {
    let fixture = CliFixture::new();
    let repo = common::init_git_repo();
    let store = TempDir::new().unwrap();
    let before = snapshot_tree(store.path());

    for (command, expected_stdout) in [
        (vec!["item", "list", "--format", "plain"], ""),
        (vec!["item", "status", "--format", "plain"], ""),
        (vec!["item", "info", "missing.txt", "--format", "plain"], ""),
        (
            vec!["repo", "gc", "--dry-run"],
            "no unreferenced current-repository store files found\n",
        ),
    ] {
        let output = fixture.run(repo.path(), store_args_slice(store.path(), &command));

        output.assert_code(0);
        assert_eq!(output.stdout, expected_stdout);
        assert_eq!(output.stderr, "");
        assert_eq!(snapshot_tree(store.path()), before);
        assert_absent(store.path(), "meta.json");
        assert_absent(store.path(), "index.json");
        assert_absent(store.path(), "repos");
        assert_absent(store.path(), ".lock");
    }
}
#[test]
fn read_only_cli_commands_do_not_update_last_seen_at() {
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

    let index_path = store.path().join("index.json");
    let mut index_json: Value =
        serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap())
            .expect("index.json should be valid JSON");
    let repos = index_json
        .get_mut("repos")
        .and_then(Value::as_object_mut)
        .expect("index.json should contain repos object");
    for entry in repos.values_mut() {
        entry["last_seen_at"] = Value::String("2026-01-01T00:00:00Z".to_string());
    }
    std::fs::write(
        &index_path,
        serde_json::to_string_pretty(&index_json).unwrap(),
    )
    .unwrap();

    let index_before = std::fs::read_to_string(&index_path).unwrap();
    for command in [
        vec!["item", "list", "--format", "plain"],
        vec!["item", "status", "--format", "plain"],
        vec!["item", "info", "secret.txt", "--format", "plain"],
        vec!["repo", "status", "--format", "plain"],
        vec!["repo", "gc", "--dry-run"],
        vec!["doctor", "--format", "plain"],
    ] {
        let output = fixture.run(repo.path(), store_args_slice(store.path(), &command));
        output.assert_code(0);
        assert_eq!(
            std::fs::read_to_string(&index_path).unwrap(),
            index_before,
            "{command:?} must not update last_seen_at"
        );
    }
}
