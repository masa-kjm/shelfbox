use std::{ffi::OsString, path::Path};

use serde_json::Value;

mod config;
mod contracts;
mod item;
mod repo;
mod store;

const REPO_ID: &str = "01JWPQ3VKGE93V9BDHAENVXFA5";
const ITEM_ID: &str = "01JWPQ3VKGE93V9BDHAENVXFA6";

fn row_by_key<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["key"] == key)
        .unwrap_or_else(|| panic!("missing config list row for {key}"))
}

fn assert_json_golden(actual: &str, expected: &str) {
    let _: Value = serde_json::from_str(actual).expect("command output should be valid JSON");
    let _: Value = serde_json::from_str(expected).expect("golden fixture should be valid JSON");
    assert_eq!(actual, expected);
}

fn assert_absent(root: &Path, rel: &str) {
    assert!(
        !root.join(rel).exists(),
        "expected {} to remain absent",
        root.join(rel).display()
    );
}

fn store_args<const N: usize>(store: &Path, args: [&str; N]) -> Vec<OsString> {
    let mut out = vec![OsString::from("--store"), store.as_os_str().to_os_string()];
    out.extend(args.into_iter().map(OsString::from));
    out
}

fn store_args_slice(store: &Path, args: &[&str]) -> Vec<OsString> {
    let mut out = vec![OsString::from("--store"), store.as_os_str().to_os_string()];
    out.extend(args.iter().map(OsString::from));
    out
}

fn single_repo_id(store: &Path) -> String {
    let index_json: Value =
        serde_json::from_str(&std::fs::read_to_string(store.join("index.json")).unwrap())
            .expect("index.json should be valid JSON");
    let repos = index_json
        .get("repos")
        .and_then(Value::as_object)
        .expect("index.json should contain repos object");
    assert_eq!(repos.len(), 1, "expected exactly one repo in index");
    repos.keys().next().unwrap().to_string()
}

fn single_repo_store_dir(store: &Path) -> String {
    let index_json: Value =
        serde_json::from_str(&std::fs::read_to_string(store.join("index.json")).unwrap())
            .expect("index.json should be valid JSON");
    let repos = index_json
        .get("repos")
        .and_then(Value::as_object)
        .expect("index.json should contain repos object");
    assert_eq!(repos.len(), 1, "expected exactly one repo in index");
    repos
        .values()
        .next()
        .and_then(|entry| entry.get("repo_store_dir"))
        .and_then(Value::as_str)
        .expect("repo entry should record repo_store_dir")
        .to_string()
}

fn write_v3_manifest(store: &Path, repo_store_dir: &str, repo_id: &str) {
    let repo_store = store.join("repos").join(repo_store_dir);
    std::fs::create_dir_all(&repo_store).unwrap();
    std::fs::write(
        repo_store.join("manifest.json"),
        format!(
            r#"{{
  "version": 3,
  "repo_id": "{repo_id}",
  "created_at": "2026-04-29T00:00:00Z",
  "identity_hints": {{}},
  "items": []
}}"#
        ),
    )
    .unwrap();
}

fn write_orphaned_v3_manifest(store: &Path, repo_store_dir: &str, repo_id: &str, item_id: &str) {
    let repo_store = store.join("repos").join(repo_store_dir);
    std::fs::create_dir_all(repo_store.join("items")).unwrap();
    std::fs::write(repo_store.join("items").join("old.env"), "orphan-data").unwrap();
    std::fs::write(
        repo_store.join("manifest.json"),
        format!(
            r#"{{
  "version": 3,
  "repo_id": "{repo_id}",
  "created_at": "2026-04-29T00:00:00Z",
  "identity_hints": {{}},
  "items": [{{
    "item_id": "{item_id}",
    "origin_repo_id": "{repo_id}",
    "path": "old.env",
    "store_path": "items/old.env",
    "ownership_state": "orphaned",
    "created_at": "2026-04-29T00:00:00Z",
    "updated_at": "2026-04-30T00:00:00Z"
  }}]
}}"#
        ),
    )
    .unwrap();
}

fn write_v2_manifest(
    store: &Path,
    repo_store_dir: &str,
    repo_id: &str,
    item_id: &str,
    ownership_state: &str,
) {
    let repo_store = store.join("repos").join(repo_store_dir);
    std::fs::create_dir_all(&repo_store).unwrap();
    std::fs::write(
        repo_store.join("manifest.json"),
        format!(
            r#"{{
  "version": 2,
  "repo": {{
    "id": "{repo_id}",
    "name": "my-project",
    "remote": "git@github.com:example/my-project.git"
  }},
  "items": [{{
    "item_id": "{item_id}",
    "origin_repo_id": "{repo_id}",
    "path": ".env",
    "store_path": "items/.env",
    "kind": "file",
    "link": {{"type": "symlink"}},
    "git": {{"was_tracked": false}},
    "ownership_state": "{ownership_state}",
    "created_at": "2026-04-29T00:00:00Z",
    "updated_at": "2026-04-30T00:00:00Z"
  }}],
  "namespaces": [{{
    "path": "secrets/",
    "created_at": "2026-04-29T00:00:00Z",
    "updated_at": "2026-04-29T00:00:00Z"
  }}]
}}"#
        ),
    )
    .unwrap();
}
