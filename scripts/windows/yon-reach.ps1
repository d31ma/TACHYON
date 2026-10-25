$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:process = $null
$script:window = $null
$script:desktop = $null
$script:deadline = $null
$script:yon = $null
$script:failures = [System.Collections.Generic.List[string]]::new()

function Enable-Msvc {
    if (Get-Command cl.exe -ErrorAction SilentlyContinue) { return }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { throw 'MSVC requires Visual Studio Build Tools' }
    $hostArchitecture = @($env:PROCESSOR_ARCHITEW6432, $env:PROCESSOR_ARCHITECTURE) | Where-Object { $_ } | Select-Object -First 1
    $architecture = if ($hostArchitecture -eq 'ARM64') { 'arm64' } else { 'x64' }
    $component = if ($architecture -eq 'arm64') { 'Microsoft.VisualStudio.Component.VC.Tools.ARM64' } else { 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64' }
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

function Resolve-Ty {
    if ($env:TAC_BIN) { return $env:TAC_BIN }
    cargo build --locked --bin ty
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    return (Resolve-Path (Join-Path $target 'debug\ty.exe')).Path
}

function Get-FreePort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = $listener.LocalEndpoint.Port
    $listener.Stop()
    return $port
}

function Build-ReachApp([string] $ty, [string] $fixture, [string] $origin) {
    python (Join-Path $PSScriptRoot '..\native\yon-reach-fixture.py') $fixture windows $origin
    if ($LASTEXITCODE -ne 0) { throw 'fixture generation failed' }
    & $ty build $fixture --target windows
    if ($LASTEXITCODE -ne 0) { throw 'Windows WebView2/native companion packaging failed' }
}

function Send-YonRequest([string] $method, [string] $url, [string] $body) {
    $request = [System.Net.Http.HttpRequestMessage]::new([System.Net.Http.HttpMethod]::new($method), $url)
    $request.Headers.TryAddWithoutValidation('Origin', 'https://tachyon.local') | Out-Null
    if ($body) {
        $request.Content = [System.Net.Http.StringContent]::new($body, [System.Text.Encoding]::UTF8, 'application/json')
    }
    $handler = [System.Net.Http.HttpClientHandler]::new()
    $handler.UseProxy = $false
    $client = [System.Net.Http.HttpClient]::new($handler)
    try { return $client.SendAsync($request).GetAwaiter().GetResult() }
    finally { $client.Dispose() }
}

function Start-Yon([string] $ty, [string] $fixture, [int] $port, [string] $logs) {
    $arguments = @('serve', "`"$fixture`"", '--no-watch', '--port', "$port")
    $script:yon = Start-Process -FilePath $ty -ArgumentList $arguments -PassThru -NoNewWindow `
        -RedirectStandardOutput (Join-Path $logs 'yon.out.log') -RedirectStandardError (Join-Path $logs 'yon.err.log')
    $until = [DateTime]::UtcNow.AddSeconds(120)
    while ([DateTime]::UtcNow -lt $until) {
        if ($script:yon.HasExited) { throw "Yon exited with $($script:yon.ExitCode) before it was ready" }
        try {
            $answer = Send-YonRequest 'GET' "http://localhost:$port/api/hello" $null
            if ([int]$answer.StatusCode -eq 200) { Write-Host "OK: Yon ready at http://localhost:$port"; return }
        }
        catch { Write-Verbose "Yon is not ready yet: $($_.Exception.Message)" }
        Start-Sleep -Milliseconds 500
    }
    throw 'Yon did not answer GET /api/hello within 120 seconds'
}

function Assert-YonAnswer([string] $method, [string] $url, [string] $body, [int] $status) {
    $answer = Send-YonRequest $method $url $body
    $allowed = $null
    if ($answer.Headers.Contains('Access-Control-Allow-Origin')) {
        $allowed = @($answer.Headers.GetValues('Access-Control-Allow-Origin'))[0]
    }
    if ([int]$answer.StatusCode -ne $status -or $allowed -ne 'https://tachyon.local') {
        throw "Yon answered $method $url with $([int]$answer.StatusCode) and allow-origin '$allowed'; expected $status for https://tachyon.local"
    }
    Write-Host "OK: Yon answers $method $url with $status for https://tachyon.local"
}

function Test-YonAnswers([string] $origin) {
    Assert-YonAnswer 'GET' "$origin/api/hello" $null 200
    Assert-YonAnswer 'POST' "$origin/api/orders" '{"name":"","level":0}' 422
    Assert-YonAnswer 'GET' "$origin/api/account" $null 401
}

function Stop-Yon {
    if ($null -eq $script:yon -or $script:yon.HasExited) { return }
    & taskkill /PID $script:yon.Id /T /F | Out-Null
    if (-not $script:yon.WaitForExit(10000)) { throw 'Yon cleanup timed out' }
}

function Stop-NativeProcesses($process) {
    if ($null -eq $process) { return }
    $browsers = @(Get-CimInstance Win32_Process -Filter (
        "ParentProcessId = $($process.Id) AND Name = 'msedgewebview2.exe'") -OperationTimeoutSec 2 |
        Select-Object -First 16 |
        ForEach-Object { Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue })
    $owned = @($process) + $browsers
    if (-not $process.HasExited) { $process.CloseMainWindow() | Out-Null }
    $grace = [DateTime]::UtcNow.AddSeconds(5)
    foreach ($ownedProcess in $owned) {
        $wait = [Math]::Max(0, [int]($grace - [DateTime]::UtcNow).TotalMilliseconds)
        if (-not $ownedProcess.WaitForExit($wait)) {
            & taskkill /PID $ownedProcess.Id /T /F | Out-Null
        }
    }
}

function Remove-Fixture([string] $fixture) {
    $until = [DateTime]::UtcNow.AddSeconds(5)
    while (Test-Path -LiteralPath $fixture) {
        try { Remove-Item -LiteralPath $fixture -Recurse -Force; return }
        catch [System.IO.IOException] {
            if ([DateTime]::UtcNow -ge $until) { throw }
            Start-Sleep -Milliseconds 200
        }
    }
}

function Find-Control([string] $name) {
    if ($null -eq $script:window) {
        $condition = [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $script:process.Id)
        $script:window = $script:desktop.FindFirst(
            [System.Windows.Automation.TreeScope]::Children, $condition)
    }
    if ($null -eq $script:window) { return $null }
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::NameProperty, $name)
    return $script:window.FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Expect-Controls([string[]] $names) {
    $stageDeadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        if ($script:process.HasExited) { throw "native application exited with $($script:process.ExitCode)" }
        $missing = @($names | Where-Object { $null -eq (Find-Control $_) })
        if ($missing.Count -eq 0) { Write-Host ("OK: " + ($names -join ', ')); return }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $stageDeadline -and [DateTime]::UtcNow -lt $script:deadline)
    throw "rendered controls missing: $($missing -join ', ')"
}

function Invoke-Control([string] $name) {
    Expect-Controls @($name)
    $control = Find-Control $name
    $pattern = $null
    if (-not $control.TryGetCurrentPattern(
        [System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        throw "rendered control '$name' exposes no UI Automation InvokePattern"
    }
    ([System.Windows.Automation.InvokePattern]$pattern).Invoke()
}

function Test-Step([string] $button, [string[]] $labels) {
    $step = if ($button) { $button } else { 'page load' }
    try {
        if ($button) { Invoke-Control $button }
        Expect-Controls $labels
    }
    catch {
        if ($script:process.HasExited) { throw }
        $script:failures.Add("$step -> $($_.Exception.Message)")
        Write-Host "FAIL: $step -> $($_.Exception.Message)"
    }
}

function Test-RepeatedStep([string] $button, [string] $label) {
    $until = [DateTime]::UtcNow.AddSeconds(30)
    do {
        try {
            Invoke-Control $button
            Start-Sleep -Milliseconds 500
            if ($null -ne (Find-Control $label)) { Write-Host "OK: $button -> $label"; return }
        }
        catch { if ($script:process.HasExited) { throw } }
    } while ([DateTime]::UtcNow -lt $until)
    $script:failures.Add("$button -> '$label' never appeared")
    Write-Host "FAIL: $button -> '$label' never appeared"
}

function Test-ReachControls([string] $app) {
    Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
    $script:desktop = [System.Windows.Automation.AutomationElement]::RootElement
    $executable = Join-Path $app 'bin\YonReach.exe'
    if (-not (Test-Path $executable)) { throw "missing artifact $executable" }
    $script:process = Start-Process -FilePath $executable -PassThru
    $script:deadline = [DateTime]::UtcNow.AddSeconds(240)

    Expect-Controls @('Yon reach', 'Check HTTP')
    Test-Step '' @('Stream ok 3', 'Socket open')
    Test-Step 'Check HTTP' @('HTTP ok hello')
    Test-Step 'Check socket' @('Socket ok echo')
    Test-Step 'Check topic' @('Topic sent accepted', 'Topic ok')
    Test-Step 'Check validation' @('Check valid accepted', 'Check invalid refused')
    Test-Step 'Check signed in' @('SignedIn anonymous refused', 'SignedIn ok reach')
    Test-Step 'Check Rust HTTP' @('Rust HTTP ok hello')
    Test-RepeatedStep 'Check Rust stream' 'Rust stream ok 3'
    Test-Step 'Send Rust echo' @('Rust socket sent')
    Test-RepeatedStep 'Check Rust socket' 'Rust socket rust-echo'
    Test-Step 'Check Rust topic' @()
    Test-Step 'Check topic' @('Topic sent accepted', 'Topic ok')
    Test-RepeatedStep 'Check Rust topic' 'Rust topic heard 1'
    Test-Step 'Check CSharp HTTP' @('CSharp HTTP ok hello')
    Test-RepeatedStep 'Check CSharp stream' 'CSharp stream ok 3'
    Test-Step 'Send CSharp echo' @('CSharp socket sent')
    Test-RepeatedStep 'Check CSharp socket' 'CSharp socket csharp-echo'
}

function Write-UiaFailureDiagnostics($window) {
    if ($null -eq $window) {
        Write-Host 'UIA: no top-level window was found for the owned native process'
        return
    }
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
    $queue = [System.Collections.Queue]::new()
    $queue.Enqueue($window)
    $remaining = 96
    while ($queue.Count -gt 0 -and $remaining -gt 0) {
        $element = $queue.Dequeue()
        $current = $element.Current
        Write-Host ("UIA: type={0}; name={1}" -f $current.ControlType.ProgrammaticName, ($current.Name -replace '[\r\n\t]', ' '))
        $remaining--
        $child = $walker.GetFirstChild($element)
        while ($null -ne $child -and $queue.Count -lt $remaining) {
            $queue.Enqueue($child)
            $child = $walker.GetNextSibling($child)
        }
    }
}

function Write-LogTail([string] $title, [string] $log) {
    if (-not (Test-Path $log)) { Write-Host "${title}: absent"; return }
    Write-Host "${title} (last 40 lines):"
    Get-Content -LiteralPath $log -Tail 40 | ForEach-Object { Write-Host $_ }
}

function Write-ReachDiagnostics([string] $logs, [string] $nativeLog) {
    Write-Host 'Yon reach diagnostics (before cleanup):'
    Write-LogTail 'Yon stdout' (Join-Path $logs 'yon.out.log')
    Write-LogTail 'Yon stderr' (Join-Path $logs 'yon.err.log')
    Write-LogTail 'Native lifecycle log' $nativeLog
    try { Write-UiaFailureDiagnostics $script:window }
    catch { Write-Warning "UIA diagnostics were incomplete: $($_.Exception.Message)" }
}

function Invoke-YonReach {
    Add-Type -AssemblyName System.Net.Http
    Set-Location (Join-Path $PSScriptRoot '..\..')
    Enable-Msvc
    Enable-WebViewSdk
    $ty = Resolve-Ty
    $port = Get-FreePort
    $origin = "http://localhost:$port"
    $fixture = Join-Path $env:TEMP ("ty-yon-reach-" + [guid]::NewGuid().ToString('N'))
    $logs = "$fixture-logs"
    New-Item -ItemType Directory -Path $logs | Out-Null
    $app = Join-Path $fixture 'dist\windows\YonReach'
    $nativeLog = Join-Path $env:LOCALAPPDATA 'Tachyon\dev.tachyon.yon-reach.jsonl'
    $failure = $null

    try {
        Build-ReachApp $ty $fixture $origin
        Start-Yon $ty $fixture $port $logs
        Test-YonAnswers $origin
        Test-ReachControls $app
        if ($script:failures.Count -gt 0) { throw "$($script:failures.Count) step(s) failed:`n$($script:failures -join "`n")" }
    }
    catch {
        $failure = $_
        Write-ReachDiagnostics $logs $nativeLog
        throw $failure
    }
    finally {
        try {
            Stop-NativeProcesses $script:process
            Stop-Yon
            Remove-Fixture $fixture
        }
        catch {
            if ($null -eq $failure) { throw }
            Write-Warning "Cleanup also failed: $($_.Exception.Message)"
        }
    }

    Remove-Item -LiteralPath $logs -Recurse -Force
    Write-Host "PASS: the Windows app on https://tachyon.local reached Yon on $origin over HTTP, a stream, a socket, a topic, a Check refusal, a SignedIn route, the linked Rust and C# HTTP clients, linked Rust and C# services' stream and socket clients held by the page, and a topic heard by a linked Rust service"
}

Invoke-YonReach
