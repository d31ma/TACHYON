param(
    [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)] [string[]] $Command
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-HostArchitecture {
    $name = @($env:PROCESSOR_ARCHITEW6432, $env:PROCESSOR_ARCHITECTURE) | Where-Object { $_ } | Select-Object -First 1
    if ($name -eq 'ARM64') { return 'arm64' }
    return 'x64'
}

function Enable-Msvc {
    $architecture = Get-HostArchitecture
    if ((Get-Command cl.exe -ErrorAction SilentlyContinue) -and $env:VSCMD_ARG_TGT_ARCH -eq $architecture) { return }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { throw 'MSVC requires Visual Studio Build Tools' }
    $component = 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64'
    if ($architecture -eq 'arm64') { $component = 'Microsoft.VisualStudio.Component.VC.Tools.ARM64' }
    $installation = & $vswhere -latest -products '*' -requires $component -property installationPath
    if ($LASTEXITCODE -ne 0 -or -not $installation) { throw "MSVC $architecture tools are not installed" }
    $developer = Join-Path $installation 'Common7\Tools\VsDevCmd.bat'
    $environment = & $env:ComSpec /d /s /c "`"$developer`" -no_logo -arch=$architecture -host_arch=$architecture >nul && set"
    if ($LASTEXITCODE -ne 0) { throw 'could not initialize the MSVC developer environment' }
    foreach ($line in $environment) {
        if ($line -match '^([^=]+)=(.*)$') {
            [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
        }
    }
    Remove-Item Env:Platform -ErrorAction SilentlyContinue
    Write-Host "==> MSVC developer environment for $architecture"
}

function Install-WebViewSdk([string] $version, [string] $directory) {
    $archive = "$directory.zip"
    Invoke-WebRequest -TimeoutSec 90 -Uri "https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/$version/microsoft.web.webview2.$version.nupkg" -OutFile $archive
    $expected = 'bc09150b179246ac90189649b13be8e6b11b3ac200e817e18df106e1f3cf489e'
    if ((Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant() -ne $expected) {
        throw 'the WebView2 SDK package checksum did not match'
    }
    Expand-Archive -Path $archive -DestinationPath $directory -Force
    Remove-Item $archive
}

function Enable-WebViewSdk {
    if ($env:TAC_WEBVIEW2_SDK) { return }
    $version = '1.0.3485.44'
    $directory = Join-Path $env:TEMP "tachyon-webview2-$version"
    if (-not (Test-Path (Join-Path $directory 'build\native\include\WebView2.h'))) {
        Install-WebViewSdk $version $directory
    }
    $env:TAC_WEBVIEW2_SDK = $directory
}

Enable-Msvc
Enable-WebViewSdk
$program = $Command[0]
$arguments = @($Command | Select-Object -Skip 1)
& $program @arguments
exit $LASTEXITCODE
