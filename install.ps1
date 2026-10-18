$ErrorActionPreference = 'Stop'

$script:TachyonSteps = 5
$script:TachyonStep = 0

function Write-TachyonStep {
    param([string]$Message)
    $script:TachyonStep += 1
    $percent = [Math]::Floor(($script:TachyonStep * 100) / $script:TachyonSteps)
    $filled = [Math]::Floor(($script:TachyonStep * 24) / $script:TachyonSteps)
    $empty = 24 - $filled
    $bar = ('#' * $filled) + ('-' * $empty)
    Write-Host ("TACHYON [{0}] {1,3}%  {2}" -f $bar, $percent, $Message)
    Write-Progress -Activity 'Tachyon install' -Status $Message -PercentComplete $percent
}

function Write-TachyonBanner {
    Write-Host 'TACHYON installer'
    Write-Host 'Bringing the ty binary online...'
    Write-Host ''
}

function Get-TachyonBaseUrl {
    if ($env:TAC_BASE_URL) {
        return $env:TAC_BASE_URL
    }
    $repo = 'd31ma/Tachyon'
    return "https://github.com/$repo/releases/latest/download"
}

function Get-TachyonInstallDirectory {
    if ($env:TAC_INSTALL_DIR) {
        return $env:TAC_INSTALL_DIR
    }
    $directory = Join-Path $env:LOCALAPPDATA 'Tachyon'
    return $directory
}

function Get-TachyonArchitecture {
    $architecture = ''
    try {
        $architecture = [string][System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    } catch {
        $architecture = ''
    }
    if (-not $architecture) {
        $architecture = $env:PROCESSOR_ARCHITECTURE
        if ($env:PROCESSOR_ARCHITEW6432) {
            $architecture = $env:PROCESSOR_ARCHITEW6432
        }
    }
    if ($architecture -eq 'Arm64') {
        return 'arm64'
    }
    return 'x64'
}

function Get-TachyonAsset {
    param([string]$Architecture)
    return "ty-windows-$Architecture.exe"
}

function Get-TachyonChecksums {
    param([string]$BaseUrl)
    $sumsContent = (Invoke-WebRequest -Uri "$BaseUrl/SHA256SUMS" -UseBasicParsing).Content
    if ($sumsContent -is [byte[]]) {
        return [Text.Encoding]::UTF8.GetString($sumsContent)
    }
    return [string]$sumsContent
}

function Assert-TachyonChecksum {
    param([string]$BaseUrl, [string]$asset, [string]$download)
    $sums = Get-TachyonChecksums $BaseUrl
    $pattern = "^[0-9a-fA-F]{64}\s+$([regex]::Escape($asset))$"
    $line = $sums -split "`n" | Where-Object { $_.Trim() -match $pattern } | Select-Object -First 1
    if (-not $line) {
        throw "No checksum published for $asset. Aborting."
    }
    $expected = ($line.Trim() -split '\s+')[0].ToLower()
    $actual = (Get-FileHash -Algorithm SHA256 $download).Hash.ToLower()
    if ($expected -ne $actual) {
        throw "Checksum mismatch for $asset. Aborting."
    }
}

function Install-TachyonBinary {
    param([string]$download, [string]$exe, [string]$backup)
    if (Test-Path $exe) {
        [IO.File]::Replace($download, $exe, $backup)
        Remove-Item $backup -Force
        return
    }
    Move-Item -Path $download -Destination $exe
}

function Add-TachyonToUserPath {
    param([string]$dest)
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath -like "*$dest*") {
        return
    }
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$dest", 'User')
    Write-Host "Added $dest to your user PATH (restart your terminal to pick it up)."
}

function Invoke-TachyonInstall {
    $base = Get-TachyonBaseUrl
    $architecture = Get-TachyonArchitecture
    $asset = Get-TachyonAsset $architecture

    Write-TachyonBanner
    Write-TachyonStep "Detected windows/$architecture"

    $dest = Get-TachyonInstallDirectory
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    $exe = Join-Path $dest 'ty.exe'
    $download = Join-Path $dest ("ty-{0}.download" -f [Guid]::NewGuid().ToString('N'))
    $backup = Join-Path $dest ("ty-{0}.backup" -f [Guid]::NewGuid().ToString('N'))
    Write-TachyonStep "Selected install directory: $dest"

    try {
        Write-TachyonStep "Downloading $asset"
        Invoke-WebRequest -Uri "$base/$asset" -OutFile $download

        Write-TachyonStep 'Verifying release checksum'
        Assert-TachyonChecksum $base $asset $download

        Write-TachyonStep 'Installing ty'
        Install-TachyonBinary $download $exe $backup
    } finally {
        Remove-Item $download -Force -ErrorAction SilentlyContinue
        Remove-Item $backup -Force -ErrorAction SilentlyContinue
    }

    Add-TachyonToUserPath $dest
    Write-Host "Installed ty to $exe"

    Write-Progress -Activity 'Tachyon install' -Completed
    Write-Host "Run 'ty --help' to get started."
}

Invoke-TachyonInstall
