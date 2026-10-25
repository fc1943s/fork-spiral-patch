param(
    [string]$ForkDir,
    [double]$MinFontSize = 1.0,
    [double]$FontSize = 0,
    [switch]$Restart
)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$invariant = [cultureinfo]::InvariantCulture
$unpatchableExitCode = 6
$proxyMarker = 'biturbo_spiral_proxy'

if (-not $ForkDir) {
    $ForkDir = (scoop prefix fork 6>$null | Select-Object -Last 1)
    if (-not $ForkDir -or -not (Test-Path -LiteralPath $ForkDir)) {
        throw 'Fork was not found through scoop; pass -ForkDir.'
    }
}
$ForkDir = (Get-Item -LiteralPath $ForkDir).ResolvedTarget ?? (Resolve-Path -LiteralPath $ForkDir).Path
$exe = Join-Path $ForkDir 'Fork.exe'
$biturbo = Join-Path $ForkDir 'biturbo.dll'
$original = Join-Path $ForkDir 'biturbo_original.dll'

function Resolve-Directory([string]$Path) {
    $item = Get-Item -LiteralPath $Path
    $item.ResolvedTarget ?? $item.FullName
}
$running = @(Get-Process Fork -ErrorAction SilentlyContinue | Where-Object {
    $_.Path -and (Resolve-Directory (Split-Path -Parent $_.Path)) -eq $ForkDir
})
if ($running.Count -gt 0) {
    if (-not $Restart) { throw "Fork is running from $ForkDir; close it or pass -Restart." }
    $running | ForEach-Object { [void]$_.CloseMainWindow() }
    $running | ForEach-Object {
        if (-not $_.WaitForExit(60000)) { throw "Fork (pid $($_.Id)) did not close; close its open dialogs." }
    }
}

$isProxy = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($biturbo)).Contains($proxyMarker)
if (-not $isProxy) {
    Copy-Item -LiteralPath $biturbo -Destination $original -Force
    Write-Host "original saved: $original"
} elseif (-not (Test-Path -LiteralPath $original)) {
    throw "$biturbo is the proxy but $original is missing; reinstall Fork."
}

$env:BITURBO_ORIGINAL = $original
try {
    cargo test --release
    if ($LASTEXITCODE -ne 0) { throw 'biturbo_spiral tests failed.' }
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'biturbo_spiral release build failed.' }
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'target/release/biturbo_spiral.dll') -Destination $biturbo -Force
    & (Join-Path $PSScriptRoot 'target/release/verify.exe') $ForkDir
    if ($LASTEXITCODE -ne 0) { throw "The proxy in $ForkDir failed verification." }
} finally {
    Remove-Item Env:BITURBO_ORIGINAL -ErrorAction SilentlyContinue
}

dotnet run (Join-Path $PSScriptRoot 'patcher/patch.cs') -- $ForkDir $MinFontSize.ToString($invariant)
$fontCapRemoved = $LASTEXITCODE -eq 0
if (-not $fontCapRemoved -and $LASTEXITCODE -ne $unpatchableExitCode) {
    throw "Patching Fork.exe failed ($LASTEXITCODE)."
}
if (-not $fontCapRemoved) {
    Write-Host 'Fork.exe is packed: the Preferences font box keeps its minimum of 10; -FontSize sets smaller sizes.'
}

if ($FontSize -gt 0) {
    $settings = Join-Path $env:LOCALAPPDATA 'Fork/settings.json'
    $json = Get-Content -LiteralPath $settings -Raw
    if ($json -notmatch '"CodeEditorFontSize"') { throw "CodeEditorFontSize not found in $settings." }
    $size = '"CodeEditorFontSize": ' + $FontSize.ToString($invariant)
    $updated = $json -replace '"CodeEditorFontSize":\s*[0-9.]+', $size
    Set-Content -LiteralPath $settings -Value $updated -NoNewline
    Write-Host "CodeEditorFontSize = $FontSize"
}

if ($running.Count -gt 0) { Start-Process -FilePath $exe -WorkingDirectory $ForkDir }
