# VS Code launch and task profiles

OpenIDEA discovers supported configurations in project-root `.vscode/launch.json` and tasks in `.vscode/tasks.json` automatically. They appear in the existing Run/Debug configuration chooser and use OpenIDEA's Run, Debug and Services tool windows. Changes to either JSON file refresh the chooser.

### Supported launch profiles

| VS Code type | Run | Debug |
| --- | --- | --- |
| `node`, `pwa-node`, `node-terminal`, `debugpy`, `python` | Yes | Existing Node/Python DAP adapters |
| `pwa-chrome`, `chrome`, `pwa-msedge`, `msedge`, `edge` | Yes | `js-debug` browser adapter (must be installed) |
| `firefox` | Yes | Not wired to a Firefox DAP adapter |
| `cppdbg`, `cppvsdbg`, `lldb`, `codelldb` | Yes | CodeLLDB (adapter required; `cppvsdbg` semantics may differ) |
| `coreclr`, `clr`, `dotnet`, `netcoredbg` | Yes | netcoredbg adapter required |
| `go`, `delve` | Yes | Delve adapter required |
| `extensionHost`, `extension-host`, `vscode-extension` | Yes (starts VS Code/VSCodium) | Requires VS Code's own Extension Host debugger; not implemented |

The converter handles `cwd`, `env`, `envFile`, arguments, and `preLaunchTask`, and it resolves `workspaceFolder` and other supported static VS Code variables. Dynamic variables such as `input` and `command` can be unsupported; skipped configurations are reported by **Import Run Configurations from .vscode/launch.json**.

### Tasks

`shell`, `process`, and `npm` tasks become `Task: <label>` configurations. `dependsOn` and `preLaunchTask` are matched against these labels and executed using the existing sequential Before Launch mechanism. Background tasks are started without waiting for process exit. **Background readiness problem matchers are not yet supported**; launches may race with server startup. Tasks that require compound parallel dependency semantics or custom VS Code task providers also need additional integration.

### Windows portable build

Run **Actions → Manual Windows portable EXE → Run workflow** on a branch containing the workflow. The workflow tests the importer, typechecks the frontend, builds a Windows x64 release without an installer, and uploads the `OpenIDEA.exe` artifact along with a SHA-256 checksum and README. The EXE needs the WebView2 runtime supplied by modern Windows installations. Portable here means unpack-and-run; OpenIDEA preferences still use their normal per-user directories.

### Debugger prerequisites

Executable launch does not imply debugger support: the relevant DAP adapter must be installed and compatible with the target program. Launch and Debug use the same OpenIDEA chooser, but breakpoints, attaching and stepping require the actual adapter.
