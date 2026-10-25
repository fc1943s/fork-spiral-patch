param(
    $ScriptDir = $PSScriptRoot
)
Set-Location $ScriptDir
$ErrorActionPreference = "Stop"


$spiralUrl = "https://github.com/i574n/spiral.git"
$spiralSibling = [IO.Path]::GetFullPath((Join-Path $ScriptDir "../../spiral"))
Write-Output "init.ps1 / spiral: $spiralUrl / sibling: $spiralSibling"

if (!(Test-Path "../deps/spiral/scripts/core.ps1")) {
    if (!(Test-Path "$spiralSibling/scripts/core.ps1")) {
        git clone $spiralUrl $spiralSibling
        if ($LASTEXITCODE -ne 0) { throw "init.ps1 / git clone spiral failed (exit code $LASTEXITCODE)" }
    }
    . "$spiralSibling/scripts/core.ps1"
    EnsureSymbolicLink -Path "../deps/spiral" -Target $spiralSibling
}
