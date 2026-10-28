param(
    [Parameter(Mandatory = $true)]
    [string]$CandidateExe
)

$ErrorActionPreference = 'Stop'
$baselineUrl = 'https://github.com/ju0o/tray-left/releases/download/v0.2.0/jusage.exe'
$baselineSha = '5525dd81aff3d95813be20375bcb1f397efe513fc86c19a4a476e611a82e21d4'
$testRoot = Join-Path $env:RUNNER_TEMP ("jusage-install-fixture-" + [Guid]::NewGuid().ToString('N'))
$localRoot = Join-Path $testRoot 'local'
$installDir = Join-Path $localRoot 'Jusage'
$installExe = Join-Path $installDir 'jusage.exe'
$baselineExe = Join-Path $testRoot 'jusage-v0.2.0.exe'
$codexHome = Join-Path $testRoot 'empty-codex-home'
$runKeyPath = 'Software\Microsoft\Windows\CurrentVersion\Run'
$oldLocalAppData = $env:LOCALAPPDATA
$oldCodexHome = $env:CODEX_HOME
$registryKey = $null
$savedValues = @{}

function Get-JusageProcesses {
    param([string[]]$ExpectedPaths)
    $normalized = @($ExpectedPaths | ForEach-Object { [IO.Path]::GetFullPath($_) })
    @(Get-CimInstance Win32_Process -Filter "Name = 'jusage.exe'" | Where-Object {
        $_.ExecutablePath -and $normalized -contains [IO.Path]::GetFullPath($_.ExecutablePath)
    })
}

function Stop-KnownJusageProcesses {
    param([string[]]$ExpectedPaths)
    $normalized = @($ExpectedPaths | ForEach-Object { [IO.Path]::GetFullPath($_) })
    foreach ($process in (Get-JusageProcesses -ExpectedPaths $ExpectedPaths)) {
        $id = [int]$process.ProcessId
        $current = Get-CimInstance Win32_Process -Filter "ProcessId = $id" -ErrorAction SilentlyContinue
        if ($current -and $current.ExecutablePath -and $normalized -contains [IO.Path]::GetFullPath($current.ExecutablePath)) {
            Stop-Process -Id $id -Force
            Wait-Process -Id $id -Timeout 10 -ErrorAction SilentlyContinue
        }
    }
}

function Restore-RunValue {
    param([string]$Name)
    if (-not $savedValues.ContainsKey($Name)) { return }
    $registryKey.DeleteValue($Name, $false)
    if ($savedValues.ContainsKey($Name) -and $savedValues[$Name].Exists) {
        $item = $savedValues[$Name]
        $registryKey.SetValue($Name, $item.Value, $item.Kind)
    }
}

try {
    $candidate = [IO.Path]::GetFullPath($CandidateExe)
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "Candidate executable is missing: $candidate"
    }
    $candidateSha = (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash.ToLowerInvariant()

    New-Item -ItemType Directory -Path $installDir, $codexHome -Force | Out-Null
    Invoke-WebRequest -Uri $baselineUrl -OutFile $baselineExe
    $downloadedSha = (Get-FileHash -LiteralPath $baselineExe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($downloadedSha -ne $baselineSha) {
        throw "Public V0.2.0 baseline hash mismatch: $downloadedSha"
    }
    Copy-Item -LiteralPath $baselineExe -Destination $installExe

    $settings = "source=windows`ndistro=`ncodex_home=`nfocus=auto`nautostart=on`ntheme=dark`n"
    $settingsPath = Join-Path $localRoot 'Jusage\settings.txt'
    $dataDir = Join-Path $localRoot 'Jusage\data'
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
    [IO.File]::WriteAllText($settingsPath, $settings, [Text.UTF8Encoding]::new($false))
    $daysPath = Join-Path $dataDir 'days.txt'
    $cyclesPath = Join-Path $dataDir 'cycle.txt'
    [IO.File]::WriteAllText($daysPath, "session 2026-10-27 98 46 46`n", [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText($cyclesPath, "session 0 98 46 46`n", [Text.UTF8Encoding]::new($false))
    $settingsBefore = (Get-FileHash -LiteralPath $settingsPath -Algorithm SHA256).Hash
    $daysBefore = (Get-FileHash -LiteralPath $daysPath -Algorithm SHA256).Hash
    $cyclesBefore = (Get-FileHash -LiteralPath $cyclesPath -Algorithm SHA256).Hash

    $registryKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($runKeyPath)
    foreach ($name in @('Jusage', 'TrayLeft')) {
        $exists = $registryKey.GetValueNames() -contains $name
        $savedValues[$name] = @{
            Exists = $exists
            Value = if ($exists) { $registryKey.GetValue($name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) } else { $null }
            Kind = if ($exists) { $registryKey.GetValueKind($name) } else { [Microsoft.Win32.RegistryValueKind]::String }
        }
    }
    $registryKey.SetValue('Jusage', $installExe, [Microsoft.Win32.RegistryValueKind]::String)
    $registryKey.SetValue('TrayLeft', 'synthetic-legacy-entry', [Microsoft.Win32.RegistryValueKind]::String)

    $env:LOCALAPPDATA = $localRoot
    $env:CODEX_HOME = $codexHome
    $parent = Start-Process -FilePath $candidate -PassThru -WindowStyle Hidden

    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    do {
        Start-Sleep -Milliseconds 250
        $installedSha = if (Test-Path -LiteralPath $installExe -PathType Leaf) {
            (Get-FileHash -LiteralPath $installExe -Algorithm SHA256).Hash.ToLowerInvariant()
        } else { '' }
        $running = Get-JusageProcesses -ExpectedPaths @($installExe)
    } while (($installedSha -ne $candidateSha -or $running.Count -eq 0) -and [DateTime]::UtcNow -lt $deadline)

    if ($installedSha -ne $candidateSha) {
        throw 'V0.3 did not copy to the isolated Jusage install path.'
    }
    if ($running.Count -ne 1) {
        throw "Expected one installed QA process, found $($running.Count)."
    }
    $firstPid = [int]$running[0].ProcessId
    if ($registryKey.GetValueNames() -contains 'TrayLeft') {
        throw 'The stale synthetic TrayLeft startup value was not removed.'
    }
    $startupValue = [string]$registryKey.GetValue('Jusage', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    if (-not [string]::Equals($startupValue.Trim('"'), $installExe, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Jusage startup value changed unexpectedly: $startupValue"
    }

    $second = Start-Process -FilePath $installExe -PassThru -WindowStyle Hidden
    try { Wait-Process -Id $second.Id -Timeout 15 -ErrorAction SilentlyContinue } catch {}
    if (Get-Process -Id $second.Id -ErrorAction SilentlyContinue) {
        throw 'The second Jusage process did not exit after the single-instance guard.'
    }
    $running = Get-JusageProcesses -ExpectedPaths @($installExe)
    if ($running.Count -ne 1 -or [int]$running[0].ProcessId -ne $firstPid) {
        throw 'The original isolated QA process did not remain the sole process.'
    }

    foreach ($pathAndHash in @(
        @{ Path = $settingsPath; Hash = $settingsBefore; Label = 'settings' },
        @{ Path = $daysPath; Hash = $daysBefore; Label = 'daily history' },
        @{ Path = $cyclesPath; Hash = $cyclesBefore; Label = 'cycle history' }
    )) {
        $after = (Get-FileHash -LiteralPath $pathAndHash.Path -Algorithm SHA256).Hash
        if ($after -ne $pathAndHash.Hash) {
            throw "The $($pathAndHash.Label) fixture changed during upgrade."
        }
    }
    Write-Output 'INSTALLATION_FIXTURE=PASS'

    Stop-KnownJusageProcesses -ExpectedPaths @($installExe)
    Copy-Item -LiteralPath $baselineExe -Destination $installExe -Force
    $rollbackSha = (Get-FileHash -LiteralPath $installExe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($rollbackSha -ne $baselineSha) {
        throw 'The stopped-app rollback did not restore the public V0.2.0 file.'
    }
    if ((Get-FileHash $settingsPath -Algorithm SHA256).Hash -ne $settingsBefore -or
        (Get-FileHash $daysPath -Algorithm SHA256).Hash -ne $daysBefore -or
        (Get-FileHash $cyclesPath -Algorithm SHA256).Hash -ne $cyclesBefore) {
        throw 'The settings or history fixture changed during rollback.'
    }
    Write-Output 'ROLLBACK_FIXTURE=PASS'

    $lock = [IO.File]::Open($installExe, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    try {
        $copyFailed = $false
        try { Copy-Item -LiteralPath $candidate -Destination $installExe -Force }
        catch { $copyFailed = $true }
        if (-not $copyFailed) { throw 'A replacement unexpectedly succeeded while the baseline file was locked.' }
        if ((Get-FileHash -LiteralPath $installExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $baselineSha) {
            throw 'The failed replacement changed the baseline executable.'
        }
    }
    finally { $lock.Dispose() }
    Write-Output 'LOCKED_FILE_GUARD=PASS'

    Write-Output "INSTALL_FIXTURE=PASS"
    Write-Output "BASELINE_SHA256=$baselineSha"
    Write-Output "CANDIDATE_SHA256=$candidateSha"
    Write-Output 'CHECKED=isolated self-copy, startup registry, single instance, settings/history preservation, stopped-app rollback, locked-file replacement failure'
    Write-Output 'LIMIT=synthetic filesystem/registry fixture; no production installer, updater rollback handler, live-process replacement, or GUI interaction'
}
finally {
    if ($registryKey) {
        Restore-RunValue -Name 'Jusage'
        Restore-RunValue -Name 'TrayLeft'
        $registryKey.Dispose()
    }
    $env:LOCALAPPDATA = $oldLocalAppData
    $env:CODEX_HOME = $oldCodexHome
    Stop-KnownJusageProcesses -ExpectedPaths @($installExe, $CandidateExe)
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
