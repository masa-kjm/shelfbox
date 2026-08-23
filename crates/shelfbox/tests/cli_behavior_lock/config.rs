use serde_json::Value;
use tempfile::TempDir;

use crate::common::{self, run_command, CliFixture};

use super::row_by_key;

#[test]
fn config_get_store_source_follows_precedence() {
    let fixture = CliFixture::new();
    let cwd = TempDir::new().unwrap();
    let file_store = TempDir::new().unwrap();
    let env_store = TempDir::new().unwrap();
    let cli_store = TempDir::new().unwrap();

    fixture.write_config(&format!(
        "store = '{}'\n",
        common::toml_literal_path(file_store.path())
    ));

    let mut command = fixture.command(cwd.path());
    command
        .arg("--store")
        .arg(cli_store.path())
        .args(["config", "get", "store", "--source"])
        .env("SHELFBOX_STORE", env_store.path());
    let output = run_command(command);
    output.assert_success();
    assert_eq!(
        output.stdout,
        format!("{}\nsource: cli\n", cli_store.path().display())
    );
    assert_eq!(output.stderr, "");

    let mut command = fixture.command(cwd.path());
    command
        .args(["config", "get", "store", "--source"])
        .env("SHELFBOX_STORE", env_store.path());
    let output = run_command(command);
    output.assert_success();
    assert_eq!(
        output.stdout,
        format!(
            "{}\nsource: env:SHELFBOX_STORE\n",
            env_store.path().display()
        )
    );
    assert_eq!(output.stderr, "");

    let output = fixture.run(cwd.path(), ["config", "get", "store", "--source"]);
    output.assert_success();
    assert_eq!(
        output.stdout,
        format!("{}\nsource: config\n", file_store.path().display())
    );
    assert_eq!(output.stderr, "");
}

#[test]
fn config_list_json_uses_isolated_defaults_without_creating_config_file() {
    let fixture = CliFixture::new();
    let cwd = TempDir::new().unwrap();

    let output = fixture.run(cwd.path(), ["config", "list", "--format", "json"]);
    output.assert_success();
    assert_eq!(output.stderr, "");
    assert!(
        !fixture.config_file_path().exists(),
        "read-only config list must not create config.toml"
    );

    let rows: Vec<Value> = serde_json::from_str(&output.stdout).unwrap();
    assert_eq!(rows.len(), 4);

    let store = row_by_key(&rows, "store");
    assert_eq!(store["type"], "path");
    assert_eq!(store["source"], "default");
    assert_eq!(
        store["current"],
        fixture.default_store_path().display().to_string()
    );

    let default_format = row_by_key(&rows, "default_format");
    assert_eq!(default_format["type"], "enum");
    assert_eq!(default_format["source"], "default");
    assert_eq!(default_format["current"], "table");

    let materialization = row_by_key(&rows, "materialization");
    assert_eq!(materialization["type"], "enum");
    assert_eq!(materialization["default"], "symlink");
    assert_eq!(materialization["source"], "default");
    assert_eq!(materialization["current"], "symlink");

    let durability = row_by_key(&rows, "mutation_durability");
    assert_eq!(durability["type"], "enum");
    assert_eq!(durability["default"], "require");
    assert_eq!(durability["source"], "default");
    assert_eq!(durability["current"], "require");
}

#[test]
fn mutation_durability_config_is_local_and_requires_explicit_opt_in() {
    let fixture = CliFixture::new();
    let cwd = TempDir::new().unwrap();

    let get = fixture.run(
        cwd.path(),
        ["config", "get", "mutation_durability", "--source"],
    );
    get.assert_success();
    assert_eq!(get.stdout, "require\nsource: default\n");

    let set = fixture.run(
        cwd.path(),
        ["config", "set", "mutation_durability", "best-effort"],
    );
    set.assert_success();
    assert_eq!(set.stdout, "set mutation_durability = best-effort\n");
    assert!(set.stderr.contains("power loss or forced termination"));
    assert_eq!(
        std::fs::read_to_string(fixture.config_file_path()).unwrap(),
        "mutation_durability = \"best-effort\"\n"
    );

    let explain = fixture.run(cwd.path(), ["config", "explain", "mutation_durability"]);
    explain.assert_success();
    assert!(explain.stdout.contains("Local parent-directory durability"));
    assert!(explain.stdout.contains("best-effort"));
}
