param(
    $ScriptDir = $PSScriptRoot
)
Set-Location $ScriptDir
$ErrorActionPreference = "Stop"


$binaries = 'biturbo_spiral.dll', 'biturbo_spiral.pdb', 'verify.exe'
$buildDirs = 'target', 'deps'
& ../deps/spiral/scripts/publish-tree.ps1 -Root .. -Include $binaries -Exclude $buildDirs
