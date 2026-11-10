# VS Code launch and task profiles

OpenIDEA discovers supported configurations in project-root `.vscode/launch.json` and tasks in `.vscode/tasks.json` automatically. They appear in the existing Run/Debug configuration chooser and use OpenIDEA's Run, Debug and Services tool windows. Changes to either JSON file refresh the chooser.

### Supported launch profiles

| VS Code type                                             | Run                           | Debug                                                           |
| -------------------------------------------------------- | ----------------------------- | --------------------------------------------------------------- |
| `node`, `pwa-node`, `node-terminal`, `debugpy`, `python` | Yes                           | Existing Node/Python DAP adapters                               |
| `pwa-chrome`, `chrome`, `pwa-msedge`, `msedge`, `edge`   | Yes                           | `js-debug` browser adapter (must be installed)                  |
| `firefox`                                                | Yes                           | Not wired to a Firefox DAP adapter                              |
| `cppdbg`, `cppvsdbg`, `lldb`, `codelldb`                 | Yes                           | CodeLLDB (adapter required; `cppvsdbg` semantics may differ)    |
| `coreclr`, `clr`, `dotnet`, `netcoredbg`                 | Yes                           | netcoredbg adapter required                                     |
| `go`, `delve`                                            | Yes                           | Delve adapter required                                          |
| `extensionHost`, `extension-host`, `vscode-extension`    | Yes (starts VS Code/VSCodium) | Requires VS Code's own Extension Host debugger; not implemented |

The converter handles `cwd`, `env`, `envFile`, arguments, and `preLaunchTask`, and it resolves `workspaceFolder` and other supported static VS Code variables. Dynamic variables such as `input` and `command` can be unsupported; skipped configurations are reported by **Import Run Configurations from .vscode/launch.json**.

### Tasks

`shell`, `process`, and `npm` tasks become `Task: <label>` configurations. `dependsOn` and `preLaunchTask` are matched against these labels and executed using the existing sequential Before Launch mechanism. Background tasks are started without waiting for process exit. **Background readiness problem matchers are not yet supported**; launches may race with server startup. Tasks that require compound parallel dependency semantics or custom VS Code task providers also need additional integration.

### Windows portable build

Run **Actions → Manual Windows portable EXE → Run workflow** on a branch containing the workflow. The workflow tests the importer, typechecks the frontend, builds a Windows x64 release without an installer, and uploads the `OpenIDEA.exe` artifact along with a SHA-256 checksum and README. The EXE needs the WebView2 runtime supplied by modern Windows installations. Portable here means unpack-and-run; OpenIDEA preferences still use their normal per-user directories.

### Debugger prerequisites

Executable launch does not imply debugger support: the relevant DAP adapter must be installed and compatible with the target program. Launch and Debug use the same OpenIDEA chooser, but breakpoints, attaching and stepping require the actual adapter.

### Rust / CodeLLDB Cargo profiles

Rust launch profiles with `type: "lldb"` or `type: "codelldb"` and a CodeLLDB `cargo` block are imported as Cargo run configurations. Cargo `build` / `test --no-run` / `--example` targets and the `cargo.filter` selection are preserved for DAP launch, and `stopOnEntry` is passed to CodeLLDB. Run uses Cargo's native CLI; Debug uses the CodeLLDB adapter and Cargo build artifacts. `preLaunchTask` is still executed before debugging. Install CodeLLDB in OpenIDEA before using Rust breakpoints.

### Interactive Rust and Tauri debugging

OpenIDEA already provides DAP actions for breakpoints, resume/pause, step in/over/out, call stacks and variable inspection. Rust binaries and the **native Rust core of Tauri** can use these actions via CodeLLDB. CodeLLDB and Cargo may be provisioned by the user's scripts; neither needs to be embedded in the portable EXE.

For a Tauri v2 project, OpenIDEA imports the [official CodeLLDB Cargo launch recipe](https://v2.tauri.app/develop/debug/vscode/): `type: "lldb"` with `cargo.args: ["build", "--manifest-path=./src-tauri/Cargo.toml"]` and optional `preLaunchTask: "ui:dev"`. The debug adapter builds and directly launches the Rust executable. **Do not start `tauri dev` as a concurrent pre-launch task** because it starts another application instance; start only the web dev server in that task. Tauri CLI-only startup hooks are not automatically reproduced.

Tauri's frontend runs inside a platform WebView and is **not controlled by the Rust DAP session**. Debug builds expose WebView DevTools (`Ctrl+Shift+I` on Windows/Linux) for JavaScript/TypeScript breakpoints and inspection. A unified frontend WebView debugging session inside OpenIDEA's own Debug tool window is not implemented. Background `ui:dev` tasks currently do not await a `problemMatcher` readiness condition; the dev server must be ready when the Rust app opens its WebView.
