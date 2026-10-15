param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'environment.ps1')
Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Cần cài Rust stable MSVC.' }
if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) { throw 'Cần Microsoft C++ Build Tools và Windows SDK.' }
pnpm install --frozen-lockfile
if ($LASTEXITCODE -ne 0) { throw 'Không cài được dependencies.' }
if (-not $SkipTests) {
    pnpm typecheck
    if ($LASTEXITCODE -ne 0) { throw 'Typecheck thất bại.' }
    pnpm lint
    if ($LASTEXITCODE -ne 0) { throw 'Lint thất bại.' }
    pnpm test
    if ($LASTEXITCODE -ne 0) { throw 'Frontend tests thất bại.' }
    cargo test --manifest-path src-tauri/Cargo.toml -p task-note-core
    if ($LASTEXITCODE -ne 0) { throw 'Rust tests thất bại.' }
}
cargo build --manifest-path src-tauri/Cargo.toml --release -p task-note-core --bin tasknote
if ($LASTEXITCODE -ne 0) { throw 'Build CLI thất bại.' }
if (-not $SkipTests) {
    cargo test --manifest-path src-tauri/Cargo.toml --workspace
    if ($LASTEXITCODE -ne 0) { throw 'Rust desktop tests thất bại.' }
}
pnpm tauri build
if ($LASTEXITCODE -ne 0) { throw 'Build NSIS thất bại.' }
New-Item -ItemType Directory -Path artifacts -Force | Out-Null
Get-ChildItem -LiteralPath 'src-tauri\target\release\bundle\nsis' -Filter '*.exe' | Copy-Item -Destination artifacts
Copy-Item -LiteralPath 'src-tauri\target\release\tasknote.exe' -Destination artifacts
Copy-Item -LiteralPath 'src-tauri\target\release\task-note-ai.exe' -Destination artifacts
Get-ChildItem -LiteralPath artifacts -Filter '*.exe' | Get-FileHash -Algorithm SHA256
