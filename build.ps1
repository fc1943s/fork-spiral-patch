param(
    $ScriptDir = $PSScriptRoot
)
Set-Location $ScriptDir
$ErrorActionPreference = "Stop"
. deps/spiral/scripts/core.ps1

Remove-Item bin -Recurse -Force -ErrorAction Ignore
{ pwsh test-fork.ps1 -BinDir bin } | Invoke-Block
