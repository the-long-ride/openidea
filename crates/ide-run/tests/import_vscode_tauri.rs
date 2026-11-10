//! Regression test for Tauri v2's documented CodeLLDB/Cargo launch configuration.
//! Validates the DAP-ready import rather than requiring a local Tauri installation.

use ide_run::import::{RunImportSource, import};
use std::collections::BTreeMap;
use std::fs;

#[test]
fn tauri_dev_launch_preserves_cargo_build_and_before_launch_task() {
    let project = tempfile::tempdir().unwrap();
    let vscode = project.path().join(".vscode");
    fs::create_dir_all(&vscode).unwrap();
    fs::write(
        vscode.join("launch.json"),
        r#"{
          "version": "0.2.0",
          "configurations": [{
            "name": "Tauri Development Debug",
            "type": "lldb",
            "request": "launch",
            "cargo": {
              "args": ["build", "--manifest-path=./src-tauri/Cargo.toml", "--no-default-features"]
            },
            "preLaunchTask": "ui:dev"
          }]
        }"#,
    )
    .unwrap();
    fs::write(
        vscode.join("tasks.json"),
        r#"{
          "version": "2.0.0",
          "tasks": [{
            "label": "ui:dev",
            "type": "npm",
            "script": "dev",
            "isBackground": true
          }]
        }"#,
    )
    .unwrap();
    let imported = import(project.path(), RunImportSource::Vscode, &[], &BTreeMap::new()).unwrap();
    assert!(imported.skipped.is_empty(), "{:#?}", imported.skipped);
    assert_eq!(imported.imported.len(), 2);
    let debug = &imported.imported[0].config;
    let task = &imported.imported[1].config;
    assert_eq!(debug.name, "Tauri Development Debug");
    assert_eq!(debug.kind, "cargo");
    assert_eq!(debug.command, "cargo");
    assert_eq!(debug.debug_adapter.as_deref(), Some("codelldb"));
    assert_eq!(debug.args, ["run", "--manifest-path=./src-tauri/Cargo.toml", "--no-default-features"]);
    let cargo: serde_json::Value = serde_json::from_str(&debug.extras.params["vscode.cargo"]).unwrap();
    assert_eq!(
        cargo["args"],
        serde_json::json!(["build", "--manifest-path=./src-tauri/Cargo.toml", "--no-default-features"])
    );
    assert_eq!(debug.extras.before_launch.len(), 1);
    assert_eq!(debug.extras.before_launch[0].kind, "run");
    assert_eq!(debug.extras.before_launch[0].target, task.id);
    assert_eq!(task.name, "Task: ui:dev");
    assert_eq!(task.extras.params["vscode.background"], "true");
}
