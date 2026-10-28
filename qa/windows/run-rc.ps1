$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$toolchain = '1.90.0'
$toolchainArg = "+$toolchain"
$target = 'x86_64-pc-windows-msvc'
$workspace = $env:GITHUB_WORKSPACE
$evidence = Join-Path $env:RUNNER_TEMP 'jusage-rc-evidence'
$buildRoot = Join-Path $env:RUNNER_TEMP 'jusage-rc-builds'
$packageRoot = Join-Path $env:RUNNER_TEMP 'jusage-rc-package'
$packageZip = Join-Path $evidence 'jusage-v0.3.0-rc.zip'
New-Item -ItemType Directory -Path $evidence, $buildRoot -Force | Out-Null
$qaResultPath = Join-Path $evidence 'qa-results.txt'
$qaResults = [ordered]@{
    unit_tests = 'NOT_RUN'
    windows_release_build = 'NOT_RUN'
    synthetic_preview = 'NOT_RUN'
    installation_fixture = 'NOT_RUN'
    rollback_fixture = 'NOT_RUN'
    package_zip = 'NOT_RUN'
    binary_reproducibility = 'NOT_EVALUATED'
}
function Write-QAResults {
    $lines = @($script:qaResults.Keys | ForEach-Object { "$_=$($script:qaResults[$_])" })
    [IO.File]::WriteAllLines($script:qaResultPath, $lines, [Text.UTF8Encoding]::new($false))
}
Write-QAResults

$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path -LiteralPath $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }
if (-not (Get-Command rustup.exe -ErrorAction SilentlyContinue)) {
    $rustupInit = Join-Path $env:RUNNER_TEMP 'rustup-init.exe'
    Invoke-WebRequest 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' -OutFile $rustupInit
    & $rustupInit -y --profile minimal --default-host $target --default-toolchain $toolchain
    if ($LASTEXITCODE -ne 0) { throw 'rustup installation failed.' }
    $env:PATH = "$cargoBin;$env:PATH"
}
& rustup.exe toolchain install $toolchain --profile minimal --target $target
if ($LASTEXITCODE -ne 0) { throw 'Pinned Rust toolchain installation failed.' }
& rustup.exe target add --toolchain $toolchain $target
if ($LASTEXITCODE -ne 0) { throw 'Pinned Windows target installation failed.' }

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) { throw 'vswhere.exe was not found.' }
$vsPath = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1).Trim()
if (-not $vsPath) { throw 'A Visual Studio C++ x64 toolset was not found.' }
$vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path -LiteralPath $vcvars)) { throw 'vcvars64.bat was not found.' }
$vcvarsLines = & $env:ComSpec /d /c "call `"$vcvars`" >nul && set"
if ($LASTEXITCODE -ne 0) { throw 'MSVC environment initialization failed.' }
foreach ($line in $vcvarsLines) {
    $separator = $line.IndexOf('=')
    if ($separator -gt 0) {
        $name = $line.Substring(0, $separator)
        $value = $line.Substring($separator + 1)
        Set-Item -Path "Env:$name" -Value $value
    }
}

$sourceSha = (& git -C $workspace rev-parse HEAD).Trim()
if ($sourceSha -notmatch '^(?:[0-9a-f]{40}|[0-9a-f]{64})$') { throw 'Could not read the full source commit SHA.' }
$rustInfo = (& rustc $toolchainArg -Vv | Out-String).Trim()
$cargoInfo = (& cargo $toolchainArg -V | Out-String).Trim()
$clInfo = (& cl.exe /Bv 2>&1 | Out-String).Trim()
$linkInfo = (& link.exe /? 2>&1 | Select-Object -First 2 | Out-String).Trim()
$buildEvidencePath = Join-Path $evidence 'build-evidence.txt'
$buildEvidence = [ordered]@{
    source_commit = $sourceSha
    target = $target
    windows_version = [System.Environment]::OSVersion.Version.ToString()
    rustc = $rustInfo -replace "`r?`n", ' | '
    cargo = $cargoInfo -replace "`r?`n", ' | '
    msvc_compiler = $clInfo -replace "`r?`n", ' | '
    msvc_linker = $linkInfo -replace "`r?`n", ' | '
    runner_os = $env:RUNNER_OS
    runner_arch = $env:RUNNER_ARCH
    rustflags = '-C link-arg=/Brepro'
    cargo_encoded_rustflags = 'UNSET'
    build_command_a = 'cargo +1.90.0 build --release --locked --target x86_64-pc-windows-msvc --target-dir <RUNNER_TEMP>\jusage-rc-builds\a'
    build_command_b = 'cargo +1.90.0 build --release --locked --target x86_64-pc-windows-msvc --target-dir <RUNNER_TEMP>\jusage-rc-builds\b'
    release_a_sha256 = 'NOT_BUILT'
    release_b_sha256 = 'NOT_BUILT'
    a_b_match = 'NOT_EVALUATED'
}
function Write-BuildEvidence {
    $lines = @($script:buildEvidence.Keys | ForEach-Object { "$_=$($script:buildEvidence[$_])" })
    [IO.File]::WriteAllLines($script:buildEvidencePath, $lines, [Text.UTF8Encoding]::new($false))
}
Write-BuildEvidence

Push-Location $workspace
try {
    $qaResults.unit_tests = 'RUNNING'
    Write-Output 'Running locked Windows unit tests.'
    & cargo $toolchainArg test --locked
    if ($LASTEXITCODE -ne 0) { throw 'Windows unit tests failed.' }
    $qaResults.unit_tests = 'PASS'

    Remove-Item Env:CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
    $env:RUSTFLAGS = '-C link-arg=/Brepro'
    $targetA = Join-Path $buildRoot 'a'
    $targetB = Join-Path $buildRoot 'b'
    $commandA = "cargo +$toolchain build --release --locked --target $target --target-dir <RUNNER_TEMP>\jusage-rc-builds\a"
    $commandB = "cargo +$toolchain build --release --locked --target $target --target-dir <RUNNER_TEMP>\jusage-rc-builds\b"
    $qaResults.windows_release_build = 'RUNNING'
    Write-Output "Building A: $commandA"
    & cargo $toolchainArg build --release --locked --target $target --target-dir $targetA
    if ($LASTEXITCODE -ne 0) { throw 'Windows Release build A failed.' }
    $exeA = Join-Path $targetA "$target\release\jusage.exe"
    $shaA = (Get-FileHash -LiteralPath $exeA -Algorithm SHA256).Hash.ToLowerInvariant()
    $buildEvidence.release_a_sha256 = $shaA
    Write-BuildEvidence
    Write-Output "RELEASE_A_SHA256=$shaA"

    Write-Output "Building B: $commandB"
    & cargo $toolchainArg build --release --locked --target $target --target-dir $targetB
    if ($LASTEXITCODE -ne 0) { throw 'Windows Release build B failed.' }

    $exeB = Join-Path $targetB "$target\release\jusage.exe"
    $shaB = (Get-FileHash -LiteralPath $exeB -Algorithm SHA256).Hash.ToLowerInvariant()
    $buildEvidence.release_b_sha256 = $shaB
    $reproducible = $shaA -eq $shaB
    $buildEvidence.a_b_match = if ($reproducible) { 'PASS' } else { 'FAIL' }
    Write-BuildEvidence
    Write-Output "RELEASE_B_SHA256=$shaB"
    Write-Output "A_B_MATCH=$($buildEvidence.a_b_match)"
    $qaResults.windows_release_build = 'PASS'
    $qaResults.binary_reproducibility = if ($reproducible) { 'PASS' } else { 'FAIL' }

    & (Join-Path $workspace 'qa\windows\compare-pe.ps1') -First $exeA -Second $exeB -OutputPath (Join-Path $evidence 'pe-compare.txt') |
        Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'PE comparison failed.' }

    $previewDir = Join-Path $env:RUNNER_TEMP 'jusage-rc-preview'
    $qaResults.synthetic_preview = 'RUNNING'

    if (Test-Path -LiteralPath $previewDir) {
        Remove-Item -LiteralPath $previewDir -Recurse -Force
    }

    $previewCases = [System.Collections.Generic.List[string]]::new()
    $previewCasesPath = Join-Path $evidence 'preview-failure-cases.txt'
    [IO.File]::WriteAllText($previewCasesPath, '', [Text.UTF8Encoding]::new($false))
    $oldLocalAppData = $env:LOCALAPPDATA
    try {
        $env:LOCALAPPDATA = Join-Path $env:RUNNER_TEMP 'jusage-preview-guard-profile'
        $forbiddenPreview = Join-Path (Join-Path $env:LOCALAPPDATA 'Jusage') 'must-not-be-created'
        & $exeA --preview $forbiddenPreview 2>$null
        $guardExitCode = $LASTEXITCODE
        if ($guardExitCode -ne 22) {
            throw "Installed-profile preview path returned unexpected exit code $guardExitCode."
        }
        if (Test-Path -LiteralPath $forbiddenPreview) {
            throw 'Installed-profile preview guard created the forbidden path.'
        }
        $previewCases.Add("installed_profile_path=PASS; exit_code=$guardExitCode; target_created=false")
        [IO.File]::WriteAllLines($previewCasesPath, $previewCases.ToArray(), [Text.UTF8Encoding]::new($false))
    }
    finally { $env:LOCALAPPDATA = $oldLocalAppData }

    $bitmapFailureDir = Join-Path $env:RUNNER_TEMP 'jusage-preview-bitmap-write-failure'
    New-Item -ItemType Directory -Path $bitmapFailureDir -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $bitmapFailureDir 'dark-ready.bmp') -Force | Out-Null
    try {
        & $exeA --preview $bitmapFailureDir 2>$null
        $bitmapExitCode = $LASTEXITCODE
        if ($bitmapExitCode -ne 36) {
            throw "Preview bitmap write failure returned unexpected exit code $bitmapExitCode."
        }
        if (-not (Test-Path -LiteralPath (Join-Path $bitmapFailureDir 'dark-ready.bmp') -PathType Container)) {
            throw 'Preview bitmap failure changed its directory blocker.'
        }
        $previewCases.Add("bitmap_write_failure=PASS; exit_code=$bitmapExitCode")
        [IO.File]::WriteAllLines($previewCasesPath, $previewCases.ToArray(), [Text.UTF8Encoding]::new($false))
    }
    finally { Remove-Item -LiteralPath $bitmapFailureDir -Recurse -Force -ErrorAction SilentlyContinue }

    $measureFailureDir = Join-Path $env:RUNNER_TEMP 'jusage-preview-measure-write-failure'
    New-Item -ItemType Directory -Path $measureFailureDir -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $measureFailureDir 'measure.txt') -Force | Out-Null
    try {
        & $exeA --preview $measureFailureDir 2>$null
        $measureExitCode = $LASTEXITCODE
        if ($measureExitCode -ne 37) {
            throw "Preview measure write failure returned unexpected exit code $measureExitCode."
        }
        if (-not (Test-Path -LiteralPath (Join-Path $measureFailureDir 'measure.txt') -PathType Container)) {
            throw 'Preview measure failure changed its directory blocker.'
        }
        $previewCases.Add("measure_write_failure=PASS; exit_code=$measureExitCode")
        [IO.File]::WriteAllLines($previewCasesPath, $previewCases.ToArray(), [Text.UTF8Encoding]::new($false))
    }
    finally { Remove-Item -LiteralPath $measureFailureDir -Recurse -Force -ErrorAction SilentlyContinue }
    & $exeA --preview $previewDir 2>$null
    $previewExitCode = $LASTEXITCODE
    $measurePath = Join-Path $previewDir 'measure.txt'
    if (Test-Path -LiteralPath $measurePath) {
        Copy-Item -LiteralPath $measurePath -Destination (Join-Path $evidence 'preview-measure.txt') -Force
    }
    if ($previewExitCode -ne 0) {
        $previewStage = switch ($previewExitCode) {
            20 { 'preview output argument' }
            21 { 'absolute preview path resolution' }
            22 { 'installed-profile path guard' }
            23 { 'preview path reparse resolution' }
            24 { 'isolated profile root resolution' }
            25 { 'preview output directory creation' }
            26 { 'configure isolated preview profile' }
            30 { 'preview bitmap validation' }
            31 { 'screen device context acquisition' }
            32 { 'memory device context creation' }
            33 { 'DIB section creation' }
            34 { 'DIB section selection' }
            36 { 'preview bitmap write' }
            37 { 'preview measure write' }
            38 { 'resolve isolated preview history' }
            39 { 'write isolated preview history' }
            default { 'unknown preview stage' }
        }
        [IO.File]::WriteAllText((Join-Path $evidence 'preview-error.txt'), "exit_code=$previewExitCode`nstage=$previewStage`n", [Text.UTF8Encoding]::new($false))
        throw "Offscreen preview failed at $previewStage (exit code $previewExitCode)."
    }
    if (-not (Test-Path -LiteralPath $measurePath)) {
        [IO.File]::WriteAllText((Join-Path $evidence 'preview-error.txt'), "exit_code=0`nstage=process exited without measure.txt`n", [Text.UTF8Encoding]::new($false))
        throw 'Preview exited successfully without producing measure.txt.'
    }
    $measure = Get-Content -LiteralPath $measurePath -Raw
    if ($measure -notmatch 'mode=preview' -or $measure -notmatch 'copy_home=false' -or $measure -notmatch 'registry=false') {
        throw 'Preview did not report isolated offscreen mode.'
    }
    $renderRows = @($measure -split "`r?`n" | Where-Object { $_ -match '\.bmp ok=' })
    if ($renderRows.Count -lt 15 -or @($renderRows | Where-Object { $_ -notmatch 'ok=1 ' }).Count -gt 0) {
        throw 'One or more synthetic offscreen render cases failed.'
    }
    $testedSha = (Get-FileHash -LiteralPath $exeA -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($testedSha -ne $shaA) { throw 'The preview-tested executable changed after its build hash was recorded.' }
    $qaResults.synthetic_preview = 'PASS'

    $qaResults.installation_fixture = 'RUNNING'
    $qaResults.rollback_fixture = 'RUNNING'
    & (Join-Path $workspace 'qa\windows\verify-install-fixture.ps1') -CandidateExe $exeA |
        Tee-Object -FilePath (Join-Path $evidence 'install-fixture.txt')
    if ($LASTEXITCODE -ne 0) { throw 'The isolated Windows installation fixture failed.' }
    $installLines = Get-Content -LiteralPath (Join-Path $evidence 'install-fixture.txt')
    if ($installLines -contains 'INSTALLATION_FIXTURE=PASS') { $qaResults.installation_fixture = 'PASS' }
    if ($installLines -contains 'ROLLBACK_FIXTURE=PASS') { $qaResults.rollback_fixture = 'PASS' }
    if ($qaResults.installation_fixture -ne 'PASS' -or $qaResults.rollback_fixture -ne 'PASS') {
        throw 'The isolated installation or rollback fixture did not report PASS.'
    }
    if ((Get-FileHash -LiteralPath $exeA -Algorithm SHA256).Hash.ToLowerInvariant() -ne $testedSha) {
        throw 'The package candidate changed during installation fixture testing.'
    }

    if (Test-Path -LiteralPath $packageRoot) { Remove-Item -LiteralPath $packageRoot -Recurse -Force }
    New-Item -ItemType Directory -Path $packageRoot -Force | Out-Null
    Copy-Item -LiteralPath $exeA -Destination (Join-Path $packageRoot 'jusage.exe')
    Copy-Item -LiteralPath (Join-Path $workspace 'LICENSE') -Destination $packageRoot
    Copy-Item -LiteralPath (Join-Path $workspace 'THIRD_PARTY_NOTICES.md') -Destination $packageRoot
    Copy-Item -LiteralPath (Join-Path $workspace 'THIRD_PARTY_LICENSES') -Destination $packageRoot -Recurse
    $licenseCount = @(Get-ChildItem -LiteralPath (Join-Path $packageRoot 'THIRD_PARTY_LICENSES') -File -Recurse).Count
    if ($licenseCount -lt 76) { throw "The package is missing third-party notices ($licenseCount files found)." }
    $qaResults.package_zip = 'RUNNING'

    $reproText = if ($reproducible) { 'MATCH' } else { 'MISMATCH' }
    $manifest = @(
        "source_commit=$sourceSha"
        "rustc=$($rustInfo -replace "`r?`n", ' | ')"
        "cargo=$($cargoInfo -replace "`r?`n", ' | ')"
        "msvc_compiler=$($clInfo -replace "`r?`n", ' | ')"
        "msvc_linker=$($linkInfo -replace "`r?`n", ' | ')"
        'commands=cargo +1.90.0 test --locked; cargo +1.90.0 build --release --locked --target x86_64-pc-windows-msvc --target-dir <RUNNER_TEMP>\jusage-rc-builds\{a,b}; RUSTFLAGS=-C link-arg=/Brepro'
        "release_binary_sha256=$shaA"
        "preview_tested_binary_sha256=$testedSha"
        "independent_build_b_sha256=$shaB"
        "independent_builds=$reproText"
        "unit_tests=$($qaResults.unit_tests)"
        "windows_release_build=$($qaResults.windows_release_build)"
        "synthetic_preview=$($qaResults.synthetic_preview)"
        "installation_fixture=$($qaResults.installation_fixture)"
        "rollback_fixture=$($qaResults.rollback_fixture)"
        "binary_reproducibility=$($qaResults.binary_reproducibility)"
        'package_contents=PASS'
        "offscreen_cases=$($renderRows.Count)"
        "third_party_license_files=$licenseCount"
        'gui_dpi_dual_monitor=NOT_TESTED'
    ) -join "`n"
    [IO.File]::WriteAllText((Join-Path $packageRoot 'BUILD-MANIFEST.txt'), "$manifest`n", [Text.UTF8Encoding]::new($false))
    if (Test-Path -LiteralPath $packageZip) { Remove-Item -LiteralPath $packageZip -Force }
    Compress-Archive -Path (Join-Path $packageRoot '*') -DestinationPath $packageZip -CompressionLevel Optimal
    $zipSha = (Get-FileHash -LiteralPath $packageZip -Algorithm SHA256).Hash.ToLowerInvariant()
    $qaResults.package_zip = 'PASS'
    [IO.File]::WriteAllText((Join-Path $evidence 'package.sha256'), "$zipSha  jusage-v0.3.0-rc.zip`n", [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText((Join-Path $evidence 'build-manifest.txt'), "$manifest`npackage_sha256=$zipSha`n", [Text.UTF8Encoding]::new($false))
    Write-Output "PACKAGE_SHA256=$zipSha"
    Write-Output "BINARY_SHA256=$shaA"
    Write-Output "PREVIEW_TESTED_SHA256=$testedSha"
    Write-Output "REPRODUCIBILITY=$reproText"
}
finally {
    foreach ($key in @($qaResults.Keys)) {
        if ($qaResults[$key] -eq 'RUNNING') { $qaResults[$key] = 'FAIL' }
    }
    $installReport = Join-Path $evidence 'install-fixture.txt'
    if (Test-Path -LiteralPath $installReport) {
        $installLines = Get-Content -LiteralPath $installReport
        if ($installLines -contains 'INSTALLATION_FIXTURE=PASS') { $qaResults.installation_fixture = 'PASS' }
        if ($installLines -contains 'ROLLBACK_FIXTURE=PASS') { $qaResults.rollback_fixture = 'PASS' }
    }
    Write-BuildEvidence
    Write-QAResults
    Pop-Location
}
