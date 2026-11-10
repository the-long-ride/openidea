use ide_run::import::{RunImportSource, import};
use std::collections::BTreeMap;
use std::fs;

#[test]
fn launch_profiles_and_task_dependencies_are_imported() {
    let dir = tempfile::tempdir().unwrap();
    let vscode = dir.path().join(".vscode");
    fs::create_dir_all(&vscode).unwrap();
    fs::write(
        vscode.join("launch.json"),
        r#"{
      "version": "0.2.0",
      "configurations": [
        {"name":"Web", "type":"pwa-chrome","request":"launch","url":"http://localhost:3000","preLaunchTask":"serve"},
        {"name":"CLI", "type":"cppdbg","request":"launch","program":"bin/tool","args":["--help"]},
        {"name":"Desktop", "type":"coreclr","request":"launch","program":"bin/App.dll"},
        {"name":"Extension", "type":"extensionHost","request":"launch","args":["--disable-extensions"]},
        {"name":"Go", "type":"go","request":"launch","program":"."}
      ],
      "compounds": [{"name":"Full stack","configurations":["Web","CLI"],"preLaunchTask":"build"}]
    }"#,
    )
    .unwrap();
    fs::write(
        vscode.join("tasks.json"),
        r#"{
      "version":"2.0.0", "tasks":[
        {"label":"build","type":"process","command":"cargo","args":["build"]},
        {"label":"serve","type":"npm","script":"dev","dependsOn":"build"}
      ]
    }"#,
    )
    .unwrap();
    let result = import(dir.path(), RunImportSource::Vscode, &[], &BTreeMap::new()).unwrap();
    assert!(result.skipped.is_empty(), "{:#?}", result.skipped);
    assert_eq!(result.imported.len(), 8);
    let find = |name: &str| result.imported.iter().find(|c| c.config.name == name).unwrap().config.clone();
    assert_eq!(find("Web").debug_adapter.as_deref(), Some("js-debug"));
    assert_eq!(find("Web").extras.params["vscode.type"], "pwa-chrome");
    assert_eq!(find("Web").extras.before_launch[0].target, find("Task: serve").id);
    assert_eq!(find("Task: serve").extras.before_launch[0].target, find("Task: build").id);
    assert_eq!(find("Full stack").extras.before_launch[0].target, find("Task: build").id);
    assert_eq!(find("CLI").command, "bin/tool");
    assert_eq!(find("Desktop").extras.params["targetPath"], "bin/App.dll");
    assert_eq!(find("Extension").debug_adapter, None);
    assert!(find("Extension").args.iter().any(|x| x.starts_with("--extensionDevelopmentPath=")));
    assert_eq!(find("Go").debug_adapter.as_deref(), Some("delve"));
}

#[test]
fn standalone_tasks_json_is_importable() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".vscode")).unwrap();
    fs::write(
        dir.path().join(".vscode/tasks.json"),
        r#"{"tasks":[{"label":"build","type":"process","command":"cargo","args":["check"]}]}"#,
    )
    .unwrap();
    let result = import(dir.path(), RunImportSource::Vscode, &[], &BTreeMap::new()).unwrap();
    assert_eq!(result.imported.len(), 1);
    assert_eq!(result.imported[0].config.name, "Task: build");
}

#[test]
fn codelldb_cargo_profiles_build_bins_tests_and_examples() {
    let dir = tempfile::tempdir().unwrap();
    let vscode = dir.path().join(".vscode");
    fs::create_dir_all(&vscode).unwrap();
    fs::write(
        vscode.join("launch.json"),
        r#"{
      "configurations": [
        {"name":"Rust binary", "type":"lldb", "request":"launch",
          "cargo":{"args":["build","--package","cli","--bin","cli"],"filter":{"name":"cli","kind":"bin"}},
          "args":["--help"], "stopOnEntry":true},
        {"name":"Rust tests", "type":"lldb", "request":"launch",
          "cargo":{"args":["test","--no-run","--lib"],"filter":{"kind":"lib"}}},
        {"name":"Rust example", "type":"codelldb", "request":"launch",
          "cargo":{"args":["build","--example","hello"],"filter":{"name":"hello","kind":"example"}}}
      ]
    }"#,
    )
    .unwrap();
    let result = import(dir.path(), RunImportSource::Vscode, &[], &BTreeMap::new()).unwrap();
    assert!(result.skipped.is_empty(), "{:#?}", result.skipped);
    assert_eq!(result.imported.len(), 3);
    let find = |name: &str| result.imported.iter().find(|c| c.config.name == name).unwrap().config.clone();
    let binary = find("Rust binary");
    assert_eq!(binary.kind, "cargo");
    assert_eq!(binary.args, ["run", "--package", "cli", "--bin", "cli", "--", "--help"]);
    assert_eq!(binary.debug_adapter.as_deref(), Some("codelldb"));
    assert_eq!(binary.extras.params["vscode.stopOnEntry"], "true");
    let cargo: serde_json::Value = serde_json::from_str(&binary.extras.params["vscode.cargo"]).unwrap();
    assert_eq!(cargo["filter"]["name"], "cli");
    assert_eq!(find("Rust tests").args, ["test", "--lib"]);
    assert_eq!(find("Rust example").args, ["run", "--example", "hello"]);
}
