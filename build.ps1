param(
    $ScriptDir = $PSScriptRoot
)
Set-Location $ScriptDir
$ErrorActionPreference = "Stop"
. deps/spiral/scripts/core.ps1

Remove-Item bin -Recurse -Force -ErrorAction Ignore
foreach ($version in "latest", "2.15.3") {
    { pwsh test-fork.ps1 -Version $version -BinDir bin } | Invoke-Block
}
