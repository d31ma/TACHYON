param(
    [Parameter(Mandatory = $true)] [string] $Command,
    [string] $Directory = (Get-Location).Path,
    [int] $TimeoutMinutes = 40
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-DesktopUser {
    $owner = (Get-CimInstance Win32_ComputerSystem).UserName
    if (-not $owner) { throw 'nobody is signed in at the desktop of this machine' }
    $current = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    if ($owner -ne $current) { throw "the desktop belongs to $owner, not to $current" }
    $locked = @(Get-Process -Name LogonUI -ErrorAction SilentlyContinue |
        Where-Object { $_.SessionId -ne 0 })
    if ($locked.Count -gt 0) { Write-Host 'NOTE: the desktop is locked; pages may not render until it is unlocked' }
    return $owner
}

function Write-RunState([string] $run) {
    $environment = [ordered]@{}
    foreach ($entry in [Environment]::GetEnvironmentVariables('Process').GetEnumerator()) {
        if ($entry.Key -notlike 'SSH_*') { $environment[$entry.Key] = $entry.Value }
    }
    $state = [ordered]@{
        command = $Command
        directory = (Resolve-Path -LiteralPath $Directory).Path
        log = Join-Path $run 'output.log'
        exit = Join-Path $run 'exit.txt'
        environment = $environment
    }
    $file = Join-Path $run 'state.json'
    $state | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $file -Encoding UTF8
    return $file
}

function Write-RunWrapper([string] $run) {
    $file = Join-Path $run 'run.ps1'
    Set-Content -LiteralPath $file -Encoding UTF8 -Value @'
param([string] $StateFile)
$state = Get-Content -Raw -LiteralPath $StateFile | ConvertFrom-Json
foreach ($entry in $state.environment.PSObject.Properties) {
    [Environment]::SetEnvironmentVariable($entry.Name, $entry.Value, 'Process')
}
Set-Location -LiteralPath $state.directory
$code = 1
try {
    $line = "/d /s /c `"$($state.command) 1>`"$($state.log)`" 2>&1`""
    $process = Start-Process -FilePath $env:ComSpec -ArgumentList $line -NoNewWindow -Wait -PassThru
    $code = $process.ExitCode
}
catch { $_ | Out-String | Add-Content -LiteralPath $state.log }
Set-Content -LiteralPath $state.exit -Value $code
'@
    return $file
}

function Read-NewOutput([string] $log, [long] $offset) {
    if (-not (Test-Path -LiteralPath $log)) { return $offset }
    $stream = [IO.File]::Open($log, 'Open', 'Read', 'ReadWrite')
    try {
        if ($stream.Length -le $offset) { return $offset }
        $stream.Seek($offset, 'Begin') | Out-Null
        $reader = [IO.StreamReader]::new($stream)
        [Console]::Out.Write($reader.ReadToEnd())
        return $stream.Length
    }
    finally { $stream.Dispose() }
}

function Wait-DesktopRun([string] $folder, [string] $name, [string] $run) {
    $log = Join-Path $run 'output.log'
    $exit = Join-Path $run 'exit.txt'
    $offset = 0
    $started = [DateTime]::UtcNow.AddSeconds(30)
    $deadline = [DateTime]::UtcNow.AddMinutes($TimeoutMinutes)
    while (-not (Test-Path -LiteralPath $exit)) {
        $offset = Read-NewOutput $log $offset
        $task = Get-ScheduledTask -TaskPath $folder -TaskName $name
        if ($task.State -ne 'Running' -and [DateTime]::UtcNow -gt $started -and -not (Test-Path -LiteralPath $exit)) {
            $result = (Get-ScheduledTaskInfo -TaskPath $folder -TaskName $name).LastTaskResult
            throw "the desktop task is $($task.State) without finishing (last result $result)"
        }
        if ([DateTime]::UtcNow -gt $deadline) {
            Stop-ScheduledTask -TaskPath $folder -TaskName $name
            throw "the desktop run took longer than $TimeoutMinutes minutes"
        }
        Start-Sleep -Milliseconds 500
    }
    Start-Sleep -Milliseconds 200
    Read-NewOutput $log $offset | Out-Null
    return [int](Get-Content -LiteralPath $exit -TotalCount 1)
}

function Invoke-DesktopRun {
    $user = Get-DesktopUser
    $run = Join-Path $env:TEMP ('tachyon-desktop-run-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $run | Out-Null
    $state = Write-RunState $run
    $wrapper = Write-RunWrapper $run
    $folder = '\Tachyon\'
    $name = 'desktop-run-' + [IO.Path]::GetFileName($run).Substring(21)
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -WorkingDirectory $run -Argument (
        "-NoProfile -ExecutionPolicy Bypass -WindowStyle Minimized -File `"$wrapper`" `"$state`"")
    $principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
        -ExecutionTimeLimit (New-TimeSpan -Minutes ($TimeoutMinutes + 5)) -MultipleInstances IgnoreNew
    Register-ScheduledTask -TaskPath $folder -TaskName $name -Action $action -Principal $principal `
        -Settings $settings | Out-Null
    try {
        Write-Host "Running in the desktop session of ${user}: $Command"
        Start-ScheduledTask -TaskPath $folder -TaskName $name
        $code = Wait-DesktopRun $folder $name $run
    }
    finally {
        Unregister-ScheduledTask -TaskPath $folder -TaskName $name -Confirm:$false
    }
    Remove-Item -LiteralPath $run -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "Desktop run exit code: $code"
    exit $code
}

Invoke-DesktopRun
