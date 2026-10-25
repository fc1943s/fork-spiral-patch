param(
    [string]$Version = 'latest',
    [string]$Root = (Join-Path ([IO.Path]::GetTempPath()) 'fork-spiral-patch'),
    [int]$LaunchTimeoutSeconds = 120,
    [string]$BinDir
)
$ErrorActionPreference = 'Stop'
if ($BinDir) { $BinDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($BinDir) }
$feed = 'https://fork.dev/update/win/RELEASES'
$cdn = 'https://cdn.fork.dev/update/win'
$sha1MissingFromFeed = @{ '2.15.3' = '3F57C686B1A446CDD560825677D518E13BF01996' }
$modulesLoadedFromForkDir = 'biturbo.dll', 'biturbo_original.dll'

function Get-Sha1([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA1).Hash }

New-Item -ItemType Directory -Force -Path $Root | Out-Null
$Root = (Resolve-Path -LiteralPath $Root).Path
$feedFile = Join-Path $Root 'RELEASES'
Invoke-WebRequest -Uri $feed -OutFile $feedFile
$releases = @(Get-Content -LiteralPath $feedFile | ForEach-Object {
    if ($_ -match '([0-9A-Fa-f]{40}) Fork-([\d.]+)-full\.nupkg') {
        [pscustomobject]@{ Version = [version]$Matches[2]; Sha1 = $Matches[1].ToUpperInvariant() }
    }
})
$release = if ($Version -eq 'latest') {
    $releases | Sort-Object Version | Select-Object -Last 1
} else {
    $releases | Where-Object Version -eq ([version]$Version) | Select-Object -First 1
}
if (-not $release -and $Version -eq 'latest') { throw "No full package listed in $feed." }
$resolved = $release ? $release.Version.ToString() : $Version
$sha1 = $release ? $release.Sha1 : $sha1MissingFromFeed[$resolved]
if (-not $sha1) { throw "Fork $resolved is not listed in $feed and has no known SHA-1." }
Write-Host "Fork $resolved (sha1 $sha1)"

$package = Join-Path $Root "Fork-$resolved-full.nupkg"
if (-not (Test-Path -LiteralPath $package) -or (Get-Sha1 $package) -ne $sha1) {
    Invoke-WebRequest -Uri "$cdn/Fork-$resolved-full.nupkg" -OutFile $package
}
$downloaded = Get-Sha1 $package
if ($downloaded -ne $sha1) { throw "$package has SHA-1 $downloaded, expected $sha1." }

$forkDir = Join-Path $Root "Fork-$resolved"
$unpacked = "$forkDir.package"
foreach ($dir in $forkDir, $unpacked) {
    if (Test-Path -LiteralPath $dir) { Remove-Item -LiteralPath $dir -Recurse -Force }
}
[IO.Compression.ZipFile]::ExtractToDirectory($package, $unpacked)
Move-Item -LiteralPath (Join-Path $unpacked 'lib/app') -Destination $forkDir
Remove-Item -LiteralPath $unpacked -Recurse -Force
Write-Host "extracted: $forkDir"

$install = Join-Path $PSScriptRoot 'install.ps1'
foreach ($run in 1..2) {
    Write-Host "install run $run of 2"
    & $install -ForkDir $forkDir
}

if (Get-Process Fork -ErrorAction SilentlyContinue) {
    throw 'Close every other Fork first: the launch check needs the Fork under test to be the only one.'
}
$fork = Start-Process -FilePath (Join-Path $forkDir 'Fork.exe') -WorkingDirectory $forkDir -PassThru
try {
    $deadline = [DateTime]::UtcNow.AddSeconds($LaunchTimeoutSeconds)
    $loaded = @()
    while ($loaded.Count -lt $modulesLoadedFromForkDir.Count) {
        if ($fork.WaitForExit(1000)) {
            throw "Fork $resolved exited ($($fork.ExitCode)) before loading $($modulesLoadedFromForkDir -join ' and ')."
        }
        if ([DateTime]::UtcNow -gt $deadline) {
            throw "Fork $resolved loaded only [$($loaded -join ', ')] from $forkDir within $LaunchTimeoutSeconds s."
        }
        $fork.Refresh()
        $modules = @(try { $fork.Modules } catch [ComponentModel.Win32Exception] { })
        $loaded = @($modules | Where-Object {
            $modulesLoadedFromForkDir -contains $_.ModuleName -and [IO.Path]::GetDirectoryName($_.FileName) -eq $forkDir
        } | ForEach-Object ModuleName)
    }
    Write-Host "launch: Fork $resolved loaded $($loaded -join ' and ') from $forkDir"
} finally {
    if (-not $fork.HasExited) { taskkill /PID $fork.Id /T /F | Out-Null }
}

if ($BinDir) {
    $versionBin = Join-Path $BinDir "fork-$resolved"
    New-Item -ItemType Directory -Force -Path $versionBin | Out-Null
    foreach ($file in 'biturbo_spiral.dll', 'biturbo_spiral.pdb', 'verify.exe') {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot "target/release/$file") -Destination $versionBin -Force
    }
    Write-Host "binaries: $versionBin"
}
exit 0
