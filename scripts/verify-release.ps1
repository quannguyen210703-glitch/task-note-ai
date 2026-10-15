$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'environment.ps1')
Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
$taskPython = 'C:\Users\HP\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe'
New-Item -ItemType Directory -Path artifacts/release-checks -Force | Out-Null
pnpm typecheck *> artifacts/release-checks/typecheck.log
if ($LASTEXITCODE -ne 0) { throw 'Typecheck failed' }
pnpm lint *> artifacts/release-checks/lint.log
if ($LASTEXITCODE -ne 0) { throw 'Lint failed' }
pnpm test *> artifacts/release-checks/frontend-test.log
if ($LASTEXITCODE -ne 0) { throw 'Frontend tests failed' }
cargo test --locked --offline --manifest-path src-tauri/Cargo.toml -p task-note-core --tests -j 1 *> artifacts/release-checks/core-test.log
if ($LASTEXITCODE -ne 0) { throw 'Core tests failed' }
cargo build --locked --offline --release --manifest-path src-tauri/Cargo.toml -p task-note-core --bin tasknote *> artifacts/release-checks/cli-build.log
if ($LASTEXITCODE -ne 0) { throw 'CLI build failed' }
cargo test --locked --offline --manifest-path src-tauri/Cargo.toml -p task-note-ai --lib -j 1 *> artifacts/release-checks/desktop-test.log
if ($LASTEXITCODE -ne 0) { throw 'Desktop library tests failed' }
pnpm tauri build *> artifacts/release-checks/installer-build.log
if ($LASTEXITCODE -ne 0) { throw 'Installer build failed' }
& $taskPython scripts/cli_smoke.py src-tauri/target/release/tasknote.exe *> artifacts/release-checks/cli-smoke.log
if ($LASTEXITCODE -ne 0) { throw 'CLI smoke failed' }
& $taskPython scripts/staging_upgrade.py src-tauri/target/release/tasknote.exe artifacts/preservation/handoff-v1-20261015.sqlite *> artifacts/release-checks/staging.log
if ($LASTEXITCODE -ne 0) { throw 'Staging verification failed' }
& $taskPython scripts/native_ipc_smoke.py src-tauri/target/release/task-note-ai.exe src-tauri/target/release/tasknote.exe *> artifacts/release-checks/native-ipc.log
if ($LASTEXITCODE -ne 0) { throw 'Native IPC smoke failed' }
'All release checks passed. Installer was built; V1 was not installed over.'
