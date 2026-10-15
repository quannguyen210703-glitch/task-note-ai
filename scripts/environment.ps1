$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
if (Test-Path -LiteralPath (Join-Path $taskRoot '.tools\cargo\bin\cargo.exe')) {
    $env:CARGO_HOME = Join-Path $taskRoot '.tools\cargo'
    $env:RUSTUP_HOME = Join-Path $taskRoot '.tools\rustup'
    $env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH
}
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    $taskPnpm = 'C:\Users\HP\.cache\codex-runtimes\codex-primary-runtime\dependencies\bin\fallback'
    if (Test-Path -LiteralPath (Join-Path $taskPnpm 'pnpm.cmd')) { $env:PATH = $taskPnpm + ';' + $env:PATH }
}
$taskVs = Join-Path $taskRoot '.tools\msvc'
if (-not (Test-Path -LiteralPath (Join-Path $taskVs 'VC\Tools\MSVC'))) {
    $taskVsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $taskVsWhere) { $taskVs = & $taskVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath }
}
if ($taskVs -and (Test-Path -LiteralPath (Join-Path $taskVs 'VC\Tools\MSVC'))) {
    $taskVc = Get-ChildItem -LiteralPath (Join-Path $taskVs 'VC\Tools\MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty FullName
    $taskSdk = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10'
    $taskSdkVersion = Get-ChildItem -LiteralPath (Join-Path $taskSdk 'Lib') -Directory -ErrorAction SilentlyContinue | Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty Name
    $env:PATH = (Join-Path $taskVc 'bin\Hostx64\x64') + ';' + (Join-Path $taskSdk "bin\$taskSdkVersion\x64") + ';' + $env:PATH
    $env:INCLUDE = (Join-Path $taskVc 'include') + ';' + (Join-Path $taskSdk "Include\$taskSdkVersion\ucrt") + ';' + (Join-Path $taskSdk "Include\$taskSdkVersion\shared") + ';' + (Join-Path $taskSdk "Include\$taskSdkVersion\um") + ';' + (Join-Path $taskSdk "Include\$taskSdkVersion\winrt")
    $env:LIB = (Join-Path $taskVc 'lib\x64') + ';' + (Join-Path $taskSdk "Lib\$taskSdkVersion\ucrt\x64") + ';' + (Join-Path $taskSdk "Lib\$taskSdkVersion\um\x64")
    $env:VCINSTALLDIR = Join-Path $taskVs 'VC'
    $env:WindowsSdkDir = $taskSdk
    $env:VCToolsInstallDir = $taskVc
}
