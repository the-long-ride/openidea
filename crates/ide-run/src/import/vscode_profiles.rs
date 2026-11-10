use super::{Cx, Entry, Found, Res, SkipReason};
use crate::{BeforeLaunchTask, RunConfig};
use super::super::config;
use serde_json::{Map, Value};
use std::path::Path;

pub(super) fn task_name(label: &str) -> String { format!("Task: {label}") }

impl Cx<'_> {
    pub(super) fn browser_launch(&self, name: &str, ty: &str, obj: &Map<String, Value>) -> Res<RunConfig> {
        let edge = ty.contains("edge");
        let firefox = ty == "firefox";
        let browser = if edge { "msedge" } else if firefox { "firefox" } else { "chrome" };
        let exe = self.string(obj, "runtimeExecutable")?.unwrap_or_else(|| browser_executable(browser, self.env));
        let mut args = self.words(obj, "runtimeArgs")?;
        args.extend(self.words(obj, "args")?);
        let url = self.string(obj, "url")?.or(self.string(obj, "file")?);
        if let Some(url) = &url { args.push(url.clone()); }
        let mut c = config(name, "vscode-browser", exe, args, (!firefox).then_some("js-debug"));
        c.extras.params.insert("vscode.type".into(), if edge { "pwa-msedge" } else if firefox { "firefox" } else { "pwa-chrome" }.into());
        if let Some(url) = url { c.extras.params.insert("vscode.url".into(), url); }
        if let Some(web_root) = self.string(obj, "webRoot")? { c.extras.params.insert("vscode.webRoot".into(), web_root); }
        Ok(c)
    }

    pub(super) fn extension_launch(&self, name: &str, obj: &Map<String, Value>) -> Res<RunConfig> {
        let value = obj.get("runtimeExecutable").and_then(Value::as_str);
        let command = match value {
            Some(v) if v == concat!("$", "{execPath}") => vscode_executable(self.env),
            None => vscode_executable(self.env),
            Some(v) => self.expand(v)?,
        };
        let mut args = self.words(obj, "runtimeArgs")?;
        args.extend(self.words(obj, "args")?);
        let dev = self.string(obj, "extensionDevelopmentPath")?.unwrap_or_else(|| self.root.to_string_lossy().into_owned());
        if !args.iter().any(|a| a.starts_with("--extensionDevelopmentPath")) { args.push(format!("--extensionDevelopmentPath={dev}")); }
        if let Some(tests) = self.string(obj, "extensionTestsPath")? { args.push(format!("--extensionTestsPath={tests}")); }
        // VS Code's extensionHost debugger is not a generic Node DAP adapter.
        let mut c = config(name, "vscode-extension", command, args, None);
        c.extras.params.insert("vscode.type".into(), "extensionHost".into());
        Ok(c)
    }

    pub(super) fn native_launch(&self, name: &str, obj: &Map<String, Value>) -> Res<RunConfig> {
        let (dir, _) = self.cwd(obj)?;
        let program = self.string(obj, "program")?.ok_or(SkipReason::NothingToRun)?;
        Ok(config(name, "program", self.path_from(program, &dir), self.words(obj, "args")?, Some("codelldb")))
    }

    pub(super) fn dotnet_launch(&self, name: &str, obj: &Map<String, Value>) -> Res<RunConfig> {
        let (dir, _) = self.cwd(obj)?;
        let program = self.string(obj, "program")?.ok_or(SkipReason::NothingToRun)?;
        let program = self.path_from(program, &dir);
        let args = self.words(obj, "args")?;
        let mut c = if program.ends_with(".dll") {
            config(name, "vscode-dotnet", "dotnet".into(), std::iter::once(program.clone()).chain(args).collect(), Some("netcoredbg"))
        } else { config(name, "vscode-dotnet", program.clone(), args, Some("netcoredbg")) };
        c.extras.params.insert("targetPath".into(), program);
        Ok(c)
    }

    pub(super) fn go_launch(&self, name: &str, obj: &Map<String, Value>) -> Res<RunConfig> {
        let target = self.string(obj, "program")?.unwrap_or_else(|| ".".into());
        let mut c = config(name, "go", "go".into(), vec!["run".into(), target.clone()], Some("delve"));
        c.args.extend(self.words(obj, "args")?);
        c.extras.params.insert("goKind".into(), "run".into());
        c.extras.params.insert("target".into(), target);
        Ok(c)
    }

    pub(super) fn task_entries(&self) -> Vec<Entry> {
        let source = match std::fs::read_to_string(self.root.join(".vscode/tasks.json")) {
            Ok(value) => value,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Err(e) => return vec![Entry { name: ".vscode/tasks.json".into(), result: Err(SkipReason::InvalidFile { detail: e.to_string() }) }],
        };
        let json: Value = match serde_json::from_str(&ide_core::jsonc::strip(&source)) {
            Ok(value) => value,
            Err(e) => return vec![Entry { name: ".vscode/tasks.json".into(), result: Err(SkipReason::InvalidFile { detail: e.to_string() }) }],
        };
        json.get("tasks").and_then(Value::as_array).into_iter().flatten().map(|t| self.task_entry(t)).collect()
    }

    fn task_entry(&self, value: &Value) -> Entry {
        let label = value.get("label").and_then(Value::as_str).or_else(|| value.get("taskName").and_then(Value::as_str)).unwrap_or_default();
        let name = task_name(label);
        let result = (|| -> Res<Found> {
            let obj = value.as_object().ok_or(SkipReason::Invalid)?;
            if label.is_empty() { return Err(SkipReason::Invalid); }
            let options = obj.get("options").and_then(Value::as_object);
            let mut c = match obj.get("type").and_then(Value::as_str) {
                Some("npm") => {
                    let script = self.string(obj, "script")?.ok_or(SkipReason::NothingToRun)?;
                    config(&name, "vscode-task", if cfg!(windows) { "npm.cmd" } else { "npm" }.into(), vec!["run".into(), script], None)
                }
                Some("shell" | "process") | None => {
                    let words = self.words(obj, "args")?;
                    let cmd = self.string(obj, "command")?.or_else(|| {
                        obj.get("dependsOn").map(|_| if cfg!(windows) { "cmd /C exit 0".into() } else { "sh -c :".into() })
                    }).ok_or(SkipReason::NothingToRun)?;
                    if obj.get("type").and_then(Value::as_str) == Some("shell") && !words.is_empty() {
                        let args = words.iter().map(|w| if w.contains(' ') { format!("\"{w}\"") } else { w.clone() }).collect::<Vec<_>>();
                        config(&name, "vscode-task", format!("{cmd} {}", args.join(" ")), Vec::new(), None)
                    } else { config(&name, "vscode-task", cmd, words, None) }
                }
                Some(ty) => return Err(SkipReason::UnsupportedType { type_: ty.into() }),
            };
            c.extras.folder = Some("VS Code Tasks".into());
            if let Some(opts) = options {
                let (_, rel) = self.cwd(opts)?;
                c.cwd = rel.map(Into::into);
                self.environment(opts, &mut c)?;
            }
            if let Some(deps) = obj.get("dependsOn") {
                let names: Vec<&str> = match deps {
                    Value::String(v) => vec![v],
                    Value::Array(v) => v.iter().filter_map(Value::as_str).collect(),
                    _ => Vec::new(),
                };
                for name in names {
                    c.extras.before_launch.push(BeforeLaunchTask { kind: "run".into(), target: task_name(name) });
                }
            }
            if obj.get("isBackground").and_then(Value::as_bool) == Some(true) {
                c.extras.params.insert("vscode.background".into(), "true".into());
            }
            Ok(Found { config: c, members: Vec::new(), ignored: Vec::new() })
        })();
        Entry { name, result }
    }
}

fn vscode_executable(env: &std::collections::BTreeMap<String, String>) -> String {
    if cfg!(windows) {
        if let Some(local) = super::super::env_var(env, "LOCALAPPDATA", true) {
            for candidate in ["Programs/Microsoft VS Code/Code.exe", "Programs/VSCodium/VSCodium.exe"] {
                let f = Path::new(&local).join(candidate);
                if f.is_file() { return f.to_string_lossy().into_owned(); }
            }
        }
        "code.cmd".into()
    } else { "code".into() }
}

fn browser_executable(browser: &str, env: &std::collections::BTreeMap<String, String>) -> String {
    if cfg!(windows) {
        let home = super::super::env_var(env, "LOCALAPPDATA", true);
        let program_files = super::super::env_var(env, "ProgramFiles", true);
        let program_files_x86 = super::super::env_var(env, "ProgramFiles(x86)", true);
        let relative = match browser {
            "msedge" => "Microsoft/Edge/Application/msedge.exe",
            "firefox" => "Mozilla Firefox/firefox.exe",
            _ => "Google/Chrome/Application/chrome.exe",
        };
        for base in [program_files, program_files_x86, home].into_iter().flatten() {
            let candidate = Path::new(&base).join(relative);
            if candidate.is_file() { return candidate.to_string_lossy().into_owned(); }
        }
    }
    browser.into()
}
