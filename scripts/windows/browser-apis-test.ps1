param(
    [ValidateSet('Auto', 'Machine', 'UserCertificate', 'Register')] [string] $InstallRoute = 'Auto',
    [switch] $OpenHostLink,
    [string] $LinkHost = 'sd1-links.example.com',
    [switch] $RequireHostLink,
    [switch] $SkipUnpackaged,
    [switch] $SkipPackage
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:failures = [System.Collections.Generic.List[string]]::new()
$script:process = $null
$script:window = $null
$script:desktop = $null
$script:installed = $null
$script:trustedCertificate = $null
$script:toastHistoryProblem = $null
$script:token = [guid]::NewGuid().ToString('N').Substring(0, 8)

function Write-Ok([string] $text) { Write-Host "OK: $text" }

function Write-Skip([string] $text) { Write-Host "SKIP: $text" }

function Write-Note([string] $text) { Write-Host "NOTE: $text" }

function Add-Failure([string] $text) {
    Write-Host "FAIL: $text"
    $script:failures.Add($text)
}

function Invoke-Check([string] $checkName, [scriptblock] $checkBody) {
    try { & $checkBody }
    catch { Add-Failure "${checkName}: $($_.Exception.Message)" }
}

function Assert-DesktopSession {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $interactive = $identity.Groups -contains [Security.Principal.SecurityIdentifier]'S-1-5-4'
    if (-not $interactive -or (Get-Process -Id $PID).SessionId -eq 0) {
        throw 'this gate needs the signed-in desktop session: run it through scripts\windows\desktop-run.ps1'
    }
}

function Enable-DesktopInput {
    if (-not ('TachyonGate.Desktop' -as [type])) {
        Add-Type -Namespace TachyonGate -Name Desktop -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr window);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, System.UIntPtr extra);
'@
    }
    [TachyonGate.Desktop]::SetProcessDPIAware() | Out-Null
    Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
    $script:desktop = [System.Windows.Automation.AutomationElement]::RootElement
}

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

function Invoke-Tool([string] $description, [string] $line) {
    & $env:ComSpec /d /s /c "$line 2>&1" | ForEach-Object { Write-Host $_ }
    if ($LASTEXITCODE -ne 0) { throw "$description failed with exit code $LASTEXITCODE" }
}

function Resolve-Ty {
    if ($env:TAC_BIN) { return $env:TAC_BIN }
    Invoke-Tool 'cargo build' 'cargo build --locked --bin ty'
    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }
    return (Resolve-Path (Join-Path $target 'debug\ty.exe')).Path
}

function Write-FixtureFile([string] $root, [string] $relative, [string] $text) {
    $file = Join-Path $root $relative
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $file) | Out-Null
    [IO.File]::WriteAllText($file, $text, [Text.UTF8Encoding]::new($false))
}

function Get-FixtureManifest([string] $id, [string[]] $grants, [string] $scheme, [string] $hostName) {
    $application = [ordered]@{ id = $id; version = '0.0.1'; build = 3 }
    if ($scheme) {
        $application.links = [ordered]@{ schemes = @($scheme); hosts = @($hostName) }
    }
    $off = [ordered]@{ check = $false }
    $manifest = [ordered]@{
        name = 'Browser Gate'
        start_url = '/'
        tac = [ordered]@{
            browserApis = @($grants)
            application = $application
            names = $off
            constants = $off
            visibility = $off
            duplication = $off
            cohesion = $off
        }
    }
    return ConvertTo-Json -InputObject $manifest -Depth 8
}

function Get-ProbeService([string] $copyText, [string] $toastTitle) {
    $source = @'
@Service
export default class ProbeService {
  /** @type {TacClipboard} */
  @Needs
  clipboard;
  /** @type {TacShare} */
  @Needs
  share;
  /** @type {TacNotifications} */
  @Needs
  notifications;
  /** @type {TacGeolocation} */
  @Needs
  geolocation;
  copyAnswer = 'waiting';
  pasteAnswer = 'waiting';
  shareAnswer = 'waiting';
  askAnswer = 'waiting';
  notifyAnswer = 'waiting';
  placeAnswer = 'waiting';
  hostAnswer = 'waiting';

  copy() {
    this.copyAnswer = this.#tried(() => this.clipboard.writeText('__COPY_TEXT__'), 'called');
  }

  paste() {
    this.pasteAnswer = this.#tried(() => this.clipboard.readText(), 'called');
  }

  open() {
    this.shareAnswer = this.#tried(() => this.share.open('Browser Gate', 'Shared by Tachyon', 'https://example.com/'), 'called');
  }

  ask() {
    this.askAnswer = this.#tried(() => this.notifications.requestPermission(), 'called');
  }

  notify() {
    this.notifyAnswer = this.#tried(() => this.notifications.show('__TOAST_TITLE__', 'Sent by the Windows browser API gate'), 'notified');
  }

  locate() {
    this.placeAnswer = this.#tried(() => this.geolocation.currentPosition(), 'called');
  }

  async checkGranted() {
    this.hostAnswer = (await BrowserGateUtils.grantedChecks()).join(' | ');
  }

  async checkRefused() {
    this.hostAnswer = (await BrowserGateUtils.refusedChecks()).join(' | ');
  }

  #tried(work, answer) {
    try {
      work();
      return answer;
    } catch (refusal) {
      return 'refused: ' + refusal.message;
    }
  }

  /** @param {ClipboardTextRead} read */
  @OnEvent
  read(read) {
    this.pasteAnswer = 'text: ' + read.text;
  }

  /** @param {ShareFinished} finished */
  @OnEvent
  finished(finished) {
    this.shareAnswer = 'finished';
  }

  /** @param {NotificationPermissionChanged} changed */
  @OnEvent
  changed(changed) {
    this.askAnswer = changed.permission;
  }

  /** @param {PositionFound} found */
  @OnEvent
  found(found) {
    this.placeAnswer = `position ${found.latitude.toFixed(3)}, ${found.longitude.toFixed(3)} within ${Math.round(found.accuracy)} m`;
  }

  /** @param {BrowserCallFailed} failed */
  @OnEvent
  failed(failed) {
    const answer = 'refused: ' + failed.message;
    if (failed.call === 'writeText') this.copyAnswer = answer;
    if (failed.call === 'readText') this.pasteAnswer = answer;
    if (failed.call === 'open') this.shareAnswer = answer;
    if (failed.call === 'requestPermission') this.askAnswer = answer;
    if (failed.call === 'show') this.notifyAnswer = answer;
    if (failed.call === 'currentPosition') this.placeAnswer = answer;
  }
}
'@
    return $source.Replace('__COPY_TEXT__', $copyText).Replace('__TOAST_TITLE__', $toastTitle)
}

function Get-GateUtils {
    return @'
export default class BrowserGateUtils {
  /** @returns {string} */
  static place() {
    return location.pathname + location.search
  }

  /** @returns {Promise<string[]>} */
  static async grantedChecks() {
    return [
      `bridge=${BrowserGateUtils._bridge()}`,
      `ask=${await BrowserGateUtils._host('browserApi.notifications.requestPermission', {})}`,
      `show=${await BrowserGateUtils._host('browserApi.notifications.show', { title: 'Host check', body: 'Sent past the page check' })}`,
    ]
  }

  /** @returns {Promise<string[]>} */
  static async refusedChecks() {
    const granted = await BrowserGateUtils.grantedChecks()
    return granted.concat([
      `read=${await BrowserGateUtils._host('browserApi.clipboard.readText', {})}`,
      `write=${await BrowserGateUtils._host('browserApi.clipboard.writeText', { text: 'Written by the refused page' })}`,
      `share=${await BrowserGateUtils._host('browserApi.share.open', { title: 'Refused', text: 'Refused share' })}`,
      `webClipboard=${await BrowserGateUtils._webClipboard()}`,
      `webLocation=${await BrowserGateUtils._webLocation()}`,
    ])
  }

  /** @returns {string} */
  static _bridge() {
    return Array.isArray(globalThis.__tachyonBrowserApiBridge) ? globalThis.__tachyonBrowserApiBridge.join(',') : 'none'
  }

  /**
   * @param {string} capability
   * @param {Object} values
   * @returns {Promise<string>}
   */
  static async _host(capability, values) {
    try {
      const reply = JSON.parse(await globalThis.__tachyonNativeHostCall(capability, JSON.stringify(values)))
      return 'error' in reply ? `error ${reply.error}` : `value ${JSON.stringify(reply.value)}`
    } catch (problem) {
      return `failed ${problem.message}`
    }
  }

  /** @returns {Promise<string>} */
  static async _webClipboard() {
    try {
      await navigator.clipboard.readText()
      return 'read'
    } catch (problem) {
      return `refused ${problem.name}`
    }
  }

  /** @returns {Promise<string>} */
  static _webLocation() {
    return new Promise((resolve) => navigator.geolocation.getCurrentPosition(
      () => resolve('position'),
      (problem) => resolve(`refused ${problem.code} ${problem.message}`),
      { timeout: 5000 }))
  }
}
'@
}

function Get-ProbeView {
    return @'
<main aria-label="Browser gate"><h1>Browser gate</h1>
<button type="button" on:click="probe.copy()">Copy</button>
<button type="button" on:click="probe.paste()">Paste</button>
<button type="button" on:click="probe.open()">Share</button>
<button type="button" on:click="probe.ask()">Ask</button>
<button type="button" on:click="probe.notify()">Notify</button>
<button type="button" on:click="probe.locate()">Locate</button>
<button type="button" on:click="probe.checkGranted()">Check granted host</button>
<button type="button" on:click="probe.checkRefused()">Check refused host</button>
<p>Copy answer: {probe.copyAnswer}</p>
<p>Paste answer: {probe.pasteAnswer}</p>
<p>Share answer: {probe.shareAnswer}</p>
<p>Ask answer: {probe.askAnswer}</p>
<p>Notify answer: {probe.notifyAnswer}</p>
<p>Place answer: {probe.placeAnswer}</p>
<p>Host answer: {probe.hostAnswer}</p>
</main>
'@
}

function Write-BrowserFixture([string] $root, [string] $id, [string[]] $grants, [string] $scheme, [string] $hostName,
    [string] $copyText, [string] $toastTitle) {
    if (Test-Path -LiteralPath $root) { throw "the fixture folder $root already exists" }
    Write-FixtureFile $root 'client/manifest.json' (Get-FixtureManifest $id $grants $scheme $hostName)
    Write-FixtureFile $root 'client/shared/utils/BrowserGateUtils.js' (Get-GateUtils)
    Write-FixtureFile $root 'client/services/ProbeService.js' (Get-ProbeService $copyText $toastTitle)
    Write-FixtureFile $root 'client/components/browser/probe/tac.html' (Get-ProbeView)
    Write-FixtureFile $root 'client/pages/tac.html' "<browser-probe hydrate=`"load`" />`n"
    if (-not $scheme) { return }
    Write-FixtureFile $root 'client/services/LinkPlaceService.js' (
        "@Service`nexport default class LinkPlaceService {`n  path = BrowserGateUtils.place();`n}`n")
    Write-FixtureFile $root 'client/components/link/place/tac.html' (
        "<main aria-label=`"Link place`"><p>Link route {linkPlace.path}</p></main>`n")
    Write-FixtureFile $root 'client/pages/alpha/tac.html' "<link-place hydrate=`"load`" />`n"
    Write-FixtureFile $root 'client/pages/beta/tac.html' "<link-place hydrate=`"load`" />`n"
}

function Connect-AppWindow([System.Diagnostics.Process] $process) {
    $script:process = $process
    $script:window = $null
}

function Get-AppWindow {
    if ($null -eq $script:window -and $null -ne $script:process) {
        $condition = [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $script:process.Id)
        $script:window = $script:desktop.FindFirst([System.Windows.Automation.TreeScope]::Children, $condition)
    }
    return $script:window
}

function Find-Control([string] $name) {
    $window = Get-AppWindow
    if ($null -eq $window) { return $null }
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::NameProperty, $name)
    return $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Wait-Control([string] $name) {
    $until = [DateTime]::UtcNow.AddSeconds(30)
    do {
        if ($script:process.HasExited) { throw "the app exited with $($script:process.ExitCode)" }
        $control = Find-Control $name
        if ($null -ne $control) { return $control }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    throw "the page shows no '$name' control"
}

function Read-PageText([string] $prefix) {
    $window = Get-AppWindow
    if ($null -eq $window) { return $null }
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)
    foreach ($text in $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)) {
        $name = $text.Current.Name
        if ($name -and $name.StartsWith($prefix)) { return $name.Substring($prefix.Length).Trim() }
    }
    return $null
}

function Read-Answer([string] $label) {
    return Read-PageText "$label answer:"
}

function Wait-Answer([string] $label, [int] $seconds = 20) {
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        $answer = Read-Answer $label
        if ($null -ne $answer -and @('waiting', 'called') -notcontains $answer) { return $answer }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    return $answer
}

function Wait-PageReady {
    $until = [DateTime]::UtcNow.AddSeconds(120)
    do {
        if ($script:process.HasExited) { throw "the app exited with $($script:process.ExitCode)" }
        if ($null -ne (Read-Answer 'Copy')) { return }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    throw 'the browser API page did not render within 120 seconds'
}

function Invoke-Button([string] $name) {
    $control = Wait-Control $name
    $pattern = $null
    if (-not $control.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        throw "the '$name' button has no UI Automation InvokePattern"
    }
    ([System.Windows.Automation.InvokePattern]$pattern).Invoke()
}

function Invoke-ButtonClick([string] $name) {
    $control = Wait-Control $name
    $window = Get-AppWindow
    [TachyonGate.Desktop]::SetForegroundWindow([IntPtr]$window.Current.NativeWindowHandle) | Out-Null
    $bounds = $control.Current.BoundingRectangle
    if ($bounds.IsEmpty) { throw "the '$name' button has no place on the screen" }
    [TachyonGate.Desktop]::SetCursorPos([int]($bounds.X + $bounds.Width / 2), [int]($bounds.Y + $bounds.Height / 2)) | Out-Null
    [TachyonGate.Desktop]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    [TachyonGate.Desktop]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
}

function Use-Button([string] $name, [string] $label) {
    Invoke-Button $name
    Start-Sleep -Milliseconds 300
    $answer = Read-Answer $label
    if ($null -ne $answer -and $answer -like '*needs a user action*') {
        Write-Note "UI Automation Invoke on '$name' gave the page no user activation; clicking it with the mouse"
        Invoke-ButtonClick $name
        Start-Sleep -Milliseconds 300
    }
}

function Write-PageTexts {
    $window = Get-AppWindow
    if ($null -eq $window) { Write-Host 'UIA: no top-level window was found for the app process'; return }
    $count = 0
    foreach ($element in $window.FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)) {
        $current = $element.Current
        if (-not $current.Name) { continue }
        Write-Host ("UIA: {0} '{1}'" -f $current.ControlType.ProgrammaticName, $current.Name)
        $count++
        if ($count -ge 60) { return }
    }
}

function Get-TopWindows {
    $windows = @{}
    foreach ($child in $script:desktop.FindAll([System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.Condition]::TrueCondition)) {
        try { $windows[[string]$child.Current.NativeWindowHandle] = $child }
        catch { Write-Verbose "a top-level window closed while it was listed: $($_.Exception.Message)" }
    }
    return $windows
}

function Get-WindowText($element) {
    try {
        $current = $element.Current
        $owner = Get-Process -Id $current.ProcessId -ErrorAction SilentlyContinue
        $processName = if ($null -ne $owner) { $owner.ProcessName } else { "pid $($current.ProcessId)" }
        return "'$($current.Name)' (class $($current.ClassName), process $processName)"
    }
    catch { return 'a window that has closed' }
}

function Get-NewWindows($before) {
    foreach ($entry in (Get-TopWindows).GetEnumerator()) {
        if ($before.ContainsKey($entry.Key)) { continue }
        try { $owner = $entry.Value.Current.ProcessId }
        catch { continue }
        $ownerProcess = Get-Process -Id $owner -ErrorAction SilentlyContinue
        $ownerName = if ($null -ne $ownerProcess) { $ownerProcess.ProcessName } else { '' }
        if ($owner -eq $script:process.Id -or $ownerName -eq 'msedgewebview2') { continue }
        $entry.Value
    }
}

function Wait-NewWindows($before, [int] $seconds) {
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        $found = @(Get-NewWindows $before)
        if ($found.Count -gt 0) { return $found }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    return @()
}

function Wait-WindowsClosed($windows, [int] $seconds) {
    $handles = @($windows | ForEach-Object { [string]$_.Current.NativeWindowHandle })
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        $open = Get-TopWindows
        $left = @($handles | Where-Object { $open.ContainsKey($_) })
        if ($left.Count -eq 0) { return $true }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    return $false
}

function Close-Windows($windows) {
    foreach ($window in $windows) {
        $pattern = $null
        try {
            if ($window.TryGetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern, [ref]$pattern)) {
                ([System.Windows.Automation.WindowPattern]$pattern).Close()
            }
        }
        catch { Write-Verbose "the window had already closed: $($_.Exception.Message)" }
    }
}

function Find-PermissionPrompt {
    foreach ($name in @('Allow', 'Block', 'Deny')) {
        $control = Find-Control $name
        if ($null -ne $control) { return "a '$name' $($control.Current.ControlType.ProgrammaticName) inside the app window" }
    }
    return $null
}

function ConvertFrom-HostAnswer([string] $answer) {
    $values = @{}
    foreach ($part in ($answer -split ' \| ')) {
        $index = $part.IndexOf('=')
        if ($index -gt 0) { $values[$part.Substring(0, $index)] = $part.Substring($index + 1) }
    }
    return $values
}

function Read-WindowsClipboard {
    for ($attempt = 0; $attempt -lt 5; $attempt++) {
        try {
            $text = Get-Clipboard -Raw
            if ($null -eq $text) { return '' }
            return ([string]$text).TrimEnd("`r", "`n")
        }
        catch { Start-Sleep -Milliseconds 200 }
    }
    throw 'Get-Clipboard could not open the Windows clipboard'
}

function Write-WindowsClipboard([string] $text) {
    for ($attempt = 0; $attempt -lt 5; $attempt++) {
        try { Set-Clipboard -Value $text; return }
        catch { Start-Sleep -Milliseconds 200 }
    }
    throw 'Set-Clipboard could not open the Windows clipboard'
}

function Get-LocationConsent([string] $family) {
    $path = 'SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location'
    $places = @(@('system', "HKLM:\$path"), @('apps', "HKCU:\$path"), @('desktop apps', "HKCU:\$path\NonPackaged"))
    if ($family) { $places += , @('this package', "HKCU:\$path\$family") }
    $parts = foreach ($place in $places) {
        $item = Get-ItemProperty -LiteralPath $place[1] -Name Value -ErrorAction SilentlyContinue
        $value = if ($null -ne $item) { $item.Value } else { 'unset' }
        "$($place[0])=$value"
    }
    $service = Get-Service -Name lfsvc -ErrorAction SilentlyContinue
    $state = if ($null -ne $service) { $service.Status } else { 'missing' }
    return (@($parts) + "geolocation service=$state") -join ', '
}

function Test-GrantedClipboard([string] $copyText) {
    $fromWindows = "Set by Windows $($script:token)"
    Write-WindowsClipboard $fromWindows
    Use-Button 'Paste' 'Paste'
    $answer = Wait-Answer 'Paste'
    if ($answer -ne "text: $fromWindows") { throw "readText answered '$answer', not the text Set-Clipboard wrote" }
    Write-Ok "readText answered '$fromWindows', which Set-Clipboard put on the Windows clipboard"
    Use-Button 'Copy' 'Copy'
    $until = [DateTime]::UtcNow.AddSeconds(10)
    do {
        $text = Read-WindowsClipboard
        if ($text -eq $copyText) { break }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    if ($text -ne $copyText) { throw "after writeText, Get-Clipboard read '$text' (page answer: $(Read-Answer 'Copy'))" }
    Write-Ok "writeText put '$copyText' on the Windows clipboard, and Get-Clipboard read it"
}

function Test-GrantedShare {
    $before = Get-TopWindows
    Use-Button 'Share' 'Share'
    $answer = Wait-Answer 'Share'
    $sheets = @(Wait-NewWindows $before 10)
    if ($answer -ne 'finished') {
        Close-Windows $sheets
        throw "share.open answered '$answer'"
    }
    if ($sheets.Count -eq 0) { throw 'share.open answered, but no share window appeared on the desktop' }
    Write-Ok ("share.open opened a Windows share window: " + (($sheets | ForEach-Object { Get-WindowText $_ }) -join '; '))
    [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    if (Wait-WindowsClosed $sheets 5) { Write-Ok 'Escape dismissed the share window'; return }
    Close-Windows $sheets
    Write-Note 'Escape did not dismiss the share window, so the gate closed it through UI Automation'
}

function Test-UnpackagedNotifications {
    Use-Button 'Ask' 'Ask'
    $answer = Wait-Answer 'Ask'
    if ($answer -ne 'denied') { throw "requestPermission answered '$answer' in the unpackaged app, not denied" }
    Write-Ok 'requestPermission answers denied in the unpackaged app'
    Use-Button 'Notify' 'Notify'
    $answer = Wait-Answer 'Notify'
    if ($answer -notlike 'refused:*requestPermission*') { throw "show answered '$answer' after denied" }
    Write-Ok "show is refused in the unpackaged app: $answer"
}

function Test-GrantedHost {
    Invoke-Button 'Check granted host'
    $values = ConvertFrom-HostAnswer (Wait-Answer 'Host' 30)
    $expected = @(
        @('bridge', 'clipboard,share,notifications', 'the page bridges clipboard, share and notifications to the host'),
        @('ask', 'value "denied"', 'the host answers denied to requestPermission without package identity'),
        @('show', 'error Windows shows notifications only for the installed MSIX package*', 'the host refuses a toast without package identity')
    )
    foreach ($entry in $expected) {
        $value = if ($values.ContainsKey($entry[0])) { $values[$entry[0]] } else { 'missing' }
        if ($value -like $entry[1]) { Write-Ok "$($entry[2]): $value" }
        else { Add-Failure "granted host $($entry[0]): '$value' does not match '$($entry[1])'" }
    }
}

function Test-GrantedGeolocation([string] $family) {
    $consent = Get-LocationConsent $family
    $before = Get-TopWindows
    Use-Button 'Locate' 'Place'
    $answer = Wait-Answer 'Place' 25
    $prompt = Find-PermissionPrompt
    $appeared = @(Get-NewWindows $before)
    if ($appeared.Count -gt 0) {
        Write-Note ('windows that appeared while the page asked for its position: ' +
            (($appeared | ForEach-Object { Get-WindowText $_ }) -join '; ') + '; pressing Escape')
        [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    }
    if ($null -ne $prompt) { throw "WebView2 showed a permission prompt: $prompt" }
    if ($answer -like 'position *') {
        Write-Ok "geolocation is granted with no WebView2 prompt and answered a $answer (Windows: $consent)"
        return
    }
    if ($answer -notlike 'refused:*') { throw "currentPosition did not answer within 25 seconds: '$answer' (Windows: $consent)" }
    if ($answer -like '*denied*' -and $consent -notmatch 'Deny') {
        throw "currentPosition was refused as denied although Windows allows location ($consent): $answer"
    }
    Write-Ok "geolocation asked no WebView2 prompt and answered a clean OS-level error: $answer (Windows: $consent)"
}

function Test-RefusedPage {
    $sentinel = "Kept by Windows $($script:token)"
    Write-WindowsClipboard $sentinel
    $calls = @(
        @('Copy', 'Copy', 'clipboard'), @('Paste', 'Paste', 'clipboard'), @('Share', 'Share', 'share'),
        @('Ask', 'Ask', 'notifications'), @('Notify', 'Notify', 'notifications'), @('Locate', 'Place', 'geolocation')
    )
    foreach ($call in $calls) {
        Invoke-Check "refused $($call[0])" {
            Invoke-Button $call[0]
            $answer = Wait-Answer $call[1] 10
            if ($answer -notlike "refused: The $($call[2]) browser API is not granted*") { throw "the page answered '$answer'" }
            Write-Ok "$($call[0]) is refused without a grant: $answer"
        }
    }
}

function Test-RefusedHost {
    $sentinel = "Kept by Windows $($script:token)"
    Write-WindowsClipboard $sentinel
    Invoke-Button 'Check refused host'
    $values = ConvertFrom-HostAnswer (Wait-Answer 'Host' 40)
    $expected = @(
        @('ask', 'value "denied"', 'the host answers denied to requestPermission'),
        @('show', 'error tac.browserApis does not grant notifications*', 'the host refuses a toast'),
        @('read', 'error tac.browserApis does not grant clipboard*', 'the host refuses a clipboard read'),
        @('write', 'error tac.browserApis does not grant clipboard*', 'the host refuses a clipboard write'),
        @('share', 'error tac.browserApis does not grant share*', 'the host refuses the share sheet'),
        @('webClipboard', 'refused NotAllowedError*', 'WebView2 refuses navigator.clipboard.readText'),
        @('webLocation', 'refused 1 *', 'the PermissionRequested decision refuses navigator.geolocation (PERMISSION_DENIED)')
    )
    if ($values.ContainsKey('bridge')) { Write-Note "the refused page still lists the bridged APIs: $($values['bridge'])" }
    foreach ($entry in $expected) {
        $value = if ($values.ContainsKey($entry[0])) { $values[$entry[0]] } else { 'missing' }
        if ($value -like $entry[1]) { Write-Ok "without a grant, $($entry[2]): $value" }
        else { Add-Failure "refused host $($entry[0]): '$value' does not match '$($entry[1])'" }
    }
    $prompt = Find-PermissionPrompt
    if ($null -ne $prompt) { Add-Failure "WebView2 showed a permission prompt in the refused app: $prompt" }
    else { Write-Ok 'the refused app showed no WebView2 permission prompt' }
    $text = Read-WindowsClipboard
    if ($text -ne $sentinel) { Add-Failure "the refused app changed the Windows clipboard to '$text'" }
    else { Write-Ok 'the refused app left the Windows clipboard unchanged' }
}

function Stop-AppProcesses {
    $owned = @()
    if ($null -ne $script:process) {
        $owned += @(Get-CimInstance Win32_Process -Filter (
            "ParentProcessId = $($script:process.Id) AND Name = 'msedgewebview2.exe'") -OperationTimeoutSec 2 |
            Select-Object -First 16 |
            ForEach-Object { Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue })
        $owned = @($script:process) + $owned
    }
    $owned += @(Get-Process -Name BrowserGate -ErrorAction SilentlyContinue)
    foreach ($process in $owned) {
        if (-not $process.HasExited) { $process.CloseMainWindow() | Out-Null }
    }
    $grace = [DateTime]::UtcNow.AddSeconds(5)
    foreach ($process in $owned) {
        $wait = [Math]::Max(0, [int]($grace - [DateTime]::UtcNow).TotalMilliseconds)
        if (-not $process.WaitForExit($wait)) { & $env:ComSpec /d /c "taskkill /PID $($process.Id) /T /F >nul 2>&1" }
    }
    $script:process = $null
    $script:window = $null
}

function Remove-Fixture([string] $fixture) {
    $until = [DateTime]::UtcNow.AddSeconds(10)
    while (Test-Path -LiteralPath $fixture) {
        try { Remove-Item -LiteralPath $fixture -Recurse -Force; return }
        catch [System.IO.IOException] {
            if ([DateTime]::UtcNow -ge $until) { throw }
            Start-Sleep -Milliseconds 250
        }
        catch [System.UnauthorizedAccessException] {
            if ([DateTime]::UtcNow -ge $until) { throw }
            Start-Sleep -Milliseconds 250
        }
    }
}

function Start-UnpackagedApp([string] $app) {
    $executable = Join-Path $app 'bin\BrowserGate.exe'
    if (-not (Test-Path -LiteralPath $executable)) { throw "the build made no $executable" }
    Connect-AppWindow (Start-Process -FilePath $executable -PassThru)
    Wait-PageReady
}

function Test-GrantedApp([string] $ty, [string] $fixture, [string] $copyText) {
    Write-Host '==> unpackaged app with clipboard, share, notifications and geolocation granted'
    Write-BrowserFixture $fixture 'dev.tachyon.browser-gate' @('clipboard', 'share', 'notifications', 'geolocation') '' '' $copyText 'Unused toast'
    Invoke-Tool 'ty build (granted)' "`"$ty`" build `"$fixture`" --target windows"
    try {
        Start-UnpackagedApp (Join-Path $fixture 'dist\windows\BrowserGate')
        Write-Ok 'the granted unpackaged app rendered its page in WebView2'
        Invoke-Check 'granted host' { Test-GrantedHost }
        Invoke-Check 'granted clipboard' { Test-GrantedClipboard $copyText }
        Invoke-Check 'granted share' { Test-GrantedShare }
        Invoke-Check 'unpackaged notifications' { Test-UnpackagedNotifications }
        Invoke-Check 'granted geolocation' { Test-GrantedGeolocation '' }
    }
    catch {
        Add-Failure "granted app: $($_.Exception.Message)"
        Write-PageTexts
    }
    finally { Stop-AppProcesses }
}

function Test-RefusedApp([string] $ty, [string] $fixture) {
    Write-Host '==> unpackaged app with no browser API granted'
    Write-BrowserFixture $fixture 'dev.tachyon.browser-gate-refused' @() '' '' 'Refused copy' 'Refused toast'
    Invoke-Tool 'ty build (refused)' "`"$ty`" build `"$fixture`" --target windows"
    try {
        Start-UnpackagedApp (Join-Path $fixture 'dist\windows\BrowserGate')
        Write-Ok 'the refused unpackaged app rendered its page in WebView2'
        Test-RefusedPage
        Invoke-Check 'refused host' { Test-RefusedHost }
    }
    catch {
        Add-Failure "refused app: $($_.Exception.Message)"
        Write-PageTexts
    }
    finally { Stop-AppProcesses }
}

function New-PackageCertificate([string] $subject, [string] $pfx) {
    $certificate = New-SelfSignedCertificate -Type CodeSigningCert -Subject $subject `
        -CertStoreLocation Cert:\CurrentUser\My -NotAfter (Get-Date).AddDays(2)
    $password = [guid]::NewGuid().ToString('N')
    $secret = ConvertTo-SecureString $password -AsPlainText -Force
    Export-PfxCertificate -Cert $certificate -FilePath $pfx -Password $secret | Out-Null
    $env:TAC_WINDOWS_CERTIFICATE = $pfx
    $env:TAC_WINDOWS_CERTIFICATE_PASSWORD = $password
    $env:TAC_WINDOWS_CERTIFICATE_PUBLISHER = $subject
    return $certificate
}

function Remove-PackageCertificate($certificate, [string] $pfx) {
    if ($null -ne $certificate) {
        Remove-Item -LiteralPath "Cert:\CurrentUser\My\$($certificate.Thumbprint)" -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $pfx) { Remove-Item -LiteralPath $pfx }
    foreach ($name in @('TAC_WINDOWS_CERTIFICATE', 'TAC_WINDOWS_CERTIFICATE_PASSWORD', 'TAC_WINDOWS_CERTIFICATE_PUBLISHER')) {
        [Environment]::SetEnvironmentVariable($name, $null, 'Process')
    }
}

function Assert-PackageSignature([string] $package, $certificate) {
    if (-not (Test-Path -LiteralPath $package)) { throw "the build made no $package" }
    $signature = Get-AuthenticodeSignature -FilePath $package
    if ($null -eq $signature.SignerCertificate -or $signature.SignerCertificate.Thumbprint -ne $certificate.Thumbprint) {
        throw "the package is not signed with the gate certificate: $($signature.Status)"
    }
    Write-Ok "BrowserGate.msix is signed with the gate certificate (Windows calls the signature $($signature.Status))"
}

function Test-LinkManifest([xml] $manifest, [string] $where, [string] $scheme, [string] $hostName) {
    $names = [System.Xml.XmlNamespaceManager]::new($manifest.NameTable)
    $names.AddNamespace('m', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
    $names.AddNamespace('uap', 'http://schemas.microsoft.com/appx/manifest/uap/windows10')
    $names.AddNamespace('uap3', 'http://schemas.microsoft.com/appx/manifest/uap/windows10/3')
    $extensions = '//m:Applications/m:Application/m:Extensions'
    $protocol = $manifest.SelectSingleNode(
        "$extensions/uap:Extension[@Category='windows.protocol']/uap:Protocol[@Name='$scheme']", $names)
    $handler = $manifest.SelectSingleNode(
        "$extensions/uap3:Extension[@Category='windows.appUriHandler']/uap3:AppUriHandler/uap3:Host[@Name='$hostName']", $names)
    if ($null -eq $protocol) { throw "$where declares no windows.protocol extension for $scheme" }
    if ($null -eq $handler) { throw "$where declares no uap3:AppUriHandler host $hostName" }
    Write-Ok "$where declares the $scheme protocol and the uap3:AppUriHandler host $hostName"
}

function Test-Elevated {
    $principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Test-DeveloperMode {
    $item = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock' `
        -Name AllowDevelopmentWithoutDevLicense -ErrorAction SilentlyContinue
    return $null -ne $item -and $item.AllowDevelopmentWithoutDevLicense -eq 1
}

function Get-RouteProblem([string] $route) {
    if ($route -eq 'Machine' -and -not (Test-Elevated)) {
        return 'trusting the certificate in Cert:\LocalMachine\TrustedPeople needs an elevated prompt, and this one is not elevated'
    }
    if ($route -eq 'Register' -and -not (Test-DeveloperMode)) {
        return 'Developer Mode is off (HKLM AppModelUnlock AllowDevelopmentWithoutDevLicense is not 1), and turning it on needs an administrator'
    }
    return $null
}

function Add-TrustedCertificate($certificate, [string] $store) {
    $public = Join-Path $env:TEMP ('tachyon-browser-gate-' + [guid]::NewGuid().ToString('N') + '.cer')
    Export-Certificate -Cert $certificate -FilePath $public | Out-Null
    try { Import-Certificate -FilePath $public -CertStoreLocation $store | Out-Null }
    finally { Remove-Item -LiteralPath $public }
    $script:trustedCertificate = Join-Path $store $certificate.Thumbprint
}

function Remove-TrustedCertificate {
    if (-not $script:trustedCertificate) { return }
    try { Remove-Item -LiteralPath $script:trustedCertificate }
    catch { Add-Failure "could not remove the trusted gate certificate $($script:trustedCertificate): $($_.Exception.Message)" }
    $script:trustedCertificate = $null
}

function Install-ByRoute([string] $route, [string] $package, [string] $layout, $certificate) {
    if ($route -eq 'Machine') {
        Add-TrustedCertificate $certificate 'Cert:\LocalMachine\TrustedPeople'
        Add-AppxPackage -Path $package
        return 'the signed .msix, with the gate certificate trusted in Cert:\LocalMachine\TrustedPeople'
    }
    if ($route -eq 'UserCertificate') {
        Add-TrustedCertificate $certificate 'Cert:\CurrentUser\TrustedPeople'
        Add-AppxPackage -Path $package
        return 'the signed .msix, with the gate certificate trusted only in Cert:\CurrentUser\TrustedPeople'
    }
    Add-AppxPackage -Register (Join-Path $layout 'AppxManifest.xml')
    return 'Developer Mode registration of the unpacked layout that ty bundle packed (the installer did not check the .msix signature)'
}

function Install-GatePackage([string] $package, [string] $layout, $certificate, [string] $identity) {
    Get-AppxPackage -Name $identity | Remove-AppxPackage
    $routes = if ($InstallRoute -eq 'Auto') { @('Machine', 'UserCertificate', 'Register') } else { @($InstallRoute) }
    foreach ($route in $routes) {
        $problem = Get-RouteProblem $route
        if ($null -ne $problem) { Write-Note "install route ${route}: not available, because $problem"; continue }
        try {
            $how = Install-ByRoute $route $package $layout $certificate
            $script:installed = Get-AppxPackage -Name $identity
            if ($null -eq $script:installed) { throw 'Get-AppxPackage does not list the package after the install' }
            Write-Ok "MSIX install route ${route}: $($script:installed.PackageFullName) is installed from $how"
            return $route
        }
        catch {
            Write-Note "install route $route failed: $($_.Exception.Message)"
            Remove-TrustedCertificate
        }
    }
    return $null
}

function Remove-GatePackage {
    if ($null -eq $script:installed) { return }
    try { Remove-AppxPackage -Package $script:installed.PackageFullName }
    catch { Add-Failure "could not remove $($script:installed.PackageFullName): $($_.Exception.Message)" }
    $script:installed = $null
}

function Get-PackagedProcesses {
    $location = $script:installed.InstallLocation
    return @(Get-Process -Name BrowserGate -ErrorAction SilentlyContinue | Where-Object {
        $null -ne $_.Path -and $_.Path.StartsWith($location, [StringComparison]::OrdinalIgnoreCase)
    })
}

function Start-PackagedApp {
    Start-Process "shell:AppsFolder\$($script:installed.PackageFamilyName)!App"
    $until = [DateTime]::UtcNow.AddSeconds(60)
    do {
        $running = @(Get-PackagedProcesses)
        if ($running.Count -gt 0) { break }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    if ($running.Count -eq 0) { throw 'the installed package did not start' }
    Connect-AppWindow $running[0]
    Wait-PageReady
    Write-Ok "the installed package runs from $($script:installed.InstallLocation)"
}

function Find-ToastBanner([string] $title) {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)
    foreach ($window in (Get-TopWindows).Values) {
        try {
            $current = $window.Current
            $ownerProcess = Get-Process -Id $current.ProcessId -ErrorAction SilentlyContinue
            $owner = if ($null -ne $ownerProcess) { $ownerProcess.ProcessName } else { '' }
            $candidate = $owner -eq 'ShellExperienceHost' -or $current.ClassName -eq 'Windows.UI.Core.CoreWindow' -or
                $current.Name -like '*notification*'
            if (-not $candidate) { continue }
            foreach ($text in $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)) {
                if ($text.Current.Name -like "*$title*") { return "an on-screen banner in $(Get-WindowText $window)" }
            }
        }
        catch { Write-Verbose "a window closed while the gate looked for the toast: $($_.Exception.Message)" }
    }
    return $null
}

function Find-ToastHistory([string] $appId, [string] $title) {
    try {
        $manager = [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime]
        foreach ($toast in $manager::History.GetHistory($appId)) {
            if ($toast.Content.GetXml() -like "*$title*") { return "the notification history of $appId" }
        }
    }
    catch { $script:toastHistoryProblem = $_.Exception.Message }
    return $null
}

function Find-Toast([string] $appId, [string] $title) {
    $until = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $banner = Find-ToastBanner $title
        if ($null -ne $banner) { return $banner }
        $history = Find-ToastHistory $appId $title
        if ($null -ne $history) { return $history }
        Start-Sleep -Milliseconds 500
    } while ([DateTime]::UtcNow -lt $until)
    return $null
}

function Get-ToastsTurnedOff([string] $appId) {
    $global = Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\PushNotifications' `
        -Name ToastEnabled -ErrorAction SilentlyContinue
    if ($null -ne $global -and $global.ToastEnabled -eq 0) { return 'notifications are turned off for this user (PushNotifications ToastEnabled=0)' }
    $app = Get-ItemProperty -LiteralPath "HKCU:\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\$appId" `
        -Name Enabled -ErrorAction SilentlyContinue
    if ($null -ne $app -and $app.Enabled -eq 0) { return "notifications are turned off for $appId in Settings" }
    return $null
}

function Test-PackagedToast([string] $title) {
    $appId = "$($script:installed.PackageFamilyName)!App"
    Use-Button 'Ask' 'Ask'
    $answer = Wait-Answer 'Ask'
    if ($answer -ne 'granted') {
        $off = Get-ToastsTurnedOff $appId
        if ($null -ne $off) { Write-Skip "the toast from the installed package: requestPermission answered '$answer' because $off"; return }
        throw "requestPermission answered '$answer' in the installed package, not granted"
    }
    Write-Ok 'requestPermission answers granted in the installed package'
    Use-Button 'Notify' 'Notify'
    Start-Sleep -Seconds 2
    $answer = Read-Answer 'Notify'
    if ($answer -ne 'notified') { throw "show answered '$answer'" }
    $seen = Find-Toast $appId $title
    if ($null -eq $seen) {
        $history = if ($script:toastHistoryProblem) { "; reading the history failed: $($script:toastHistoryProblem)" } else { '' }
        throw "show answered without an error, but no toast titled '$title' was found on screen or in the history$history"
    }
    Write-Ok "the installed package showed the toast '$title', found in $seen"
}

function Wait-OnePackagedProcess([int] $firstId) {
    $until = [DateTime]::UtcNow.AddSeconds(10)
    do {
        $running = @(Get-PackagedProcesses)
        if ($running.Count -eq 1) { break }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    if ($running.Count -ne 1) { throw "$($running.Count) packaged app processes run after the link, not one" }
    if ($running[0].Id -ne $firstId) { throw "the running app is process $($running[0].Id), not the first instance $firstId" }
}

function Wait-LinkRoute([string] $path, [string] $query, [int] $seconds) {
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        $route = Read-PageText 'Link route'
        if ($null -ne $route -and $route.StartsWith($path) -and $route.Contains($query)) { return $route }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    return $null
}

function Get-LinkRecords {
    $id = 'dev.tachyon.browser-gate-package'
    $logs = @(
        (Join-Path $env:LOCALAPPDATA "Tachyon\$id.jsonl"),
        (Join-Path $env:LOCALAPPDATA "Packages\$($script:installed.PackageFamilyName)\LocalCache\Local\Tachyon\$id.jsonl")
    )
    $count = 0
    foreach ($log in $logs) {
        if (Test-Path -LiteralPath $log) { $count += @(Get-Content -LiteralPath $log | Where-Object { $_ -like '*"link.opened"*' }).Count }
    }
    return $count
}

function Test-PackagedDeepLinks([string] $scheme) {
    $firstId = $script:process.Id
    $links = @(@("${scheme}://alpha?x=1", '/alpha', 'x=1'), @("${scheme}:/beta?x=2", '/beta', 'x=2'))
    foreach ($link in $links) {
        Start-Process $link[0]
        $route = Wait-LinkRoute $link[1] $link[2] 20
        if ($null -eq $route) { throw "the running package did not open $($link[0]) (page shows '$(Read-PageText 'Link route')')" }
        Wait-OnePackagedProcess $firstId
        Write-Ok "the running package opened $($link[0]) at $route, and one app process ($firstId) runs"
    }
    Write-Note "the host log lists $(Get-LinkRecords) link.opened records"
}

function Test-WebLinkFile([string] $fixture) {
    $files = @(Get-ChildItem -LiteralPath (Join-Path $fixture 'dist') -Recurse -Force -Filter 'windows-app-web-link')
    if ($files.Count -eq 0) { throw 'the build wrote no .well-known/windows-app-web-link' }
    $entries = Get-Content -Raw -LiteralPath $files[0].FullName | ConvertFrom-Json
    $entry = @($entries)[0]
    if ($entry.packageFamilyName -ne $script:installed.PackageFamilyName) {
        throw "windows-app-web-link names $($entry.packageFamilyName), but Windows installed $($script:installed.PackageFamilyName)"
    }
    Write-Ok "windows-app-web-link names $($entry.packageFamilyName), the family name Windows gave the installed package"
}

function Wait-WinRtOperation($operation, [Type] $resultType) {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.IsGenericMethodDefinition -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
    } | Select-Object -First 1
    $task = $asTask.MakeGenericMethod($resultType).Invoke($null, @($operation))
    if (-not $task.Wait(15000)) { throw 'a Windows Runtime call did not finish within 15 seconds' }
    return $task.Result
}

function Get-HostLinkSupport([string] $link, [string] $family) {
    $launcher = [Windows.System.Launcher, Windows.System, ContentType = WindowsRuntime]
    $status = [Windows.System.LaunchQuerySupportStatus, Windows.System, ContentType = WindowsRuntime]
    return [string](Wait-WinRtOperation ($launcher::QueryAppUriSupportAsync([Uri]$link, $family)) $status)
}

function Get-HostLinkHandlers([string] $link) {
    $launcher = [Windows.System.Launcher, Windows.System, ContentType = WindowsRuntime]
    $information = [Windows.ApplicationModel.AppInfo, Windows.ApplicationModel, ContentType = WindowsRuntime]
    $list = [Type]::GetType('System.Collections.Generic.IReadOnlyList`1').MakeGenericType($information)
    $handlers = Wait-WinRtOperation ($launcher::FindAppUriHandlersAsync([Uri]$link)) $list
    $families = @($handlers | ForEach-Object { $_.PackageFamilyName })
    if ($families.Count -eq 0) { return 'none' }
    return $families -join ', '
}

function Test-PackagedHostLink([string] $hostName) {
    $family = $script:installed.PackageFamilyName
    $link = "https://$hostName/alpha?x=3"
    $support = 'unknown'
    $handlers = 'unknown'
    try { $support = Get-HostLinkSupport $link $family }
    catch { Write-Note "QueryAppUriSupportAsync failed: $($_.Exception.Message)" }
    try { $handlers = Get-HostLinkHandlers $link }
    catch { Write-Note "FindAppUriHandlersAsync failed: $($_.Exception.Message)" }
    Write-Note "Windows answers '$support' for $link in $family, and lists these app URI handlers for it: $handlers"
    if ($RequireHostLink -and $support -ne 'Available') {
        throw "Windows has not verified $hostName for $family ($support): https://$hostName/.well-known/windows-app-web-link must name the package"
    }
    if ($support -ne 'Available' -and -not $OpenHostLink) {
        Write-Skip ("opening $link in the installed package: Windows has not verified $hostName for it ($support), " +
            "because no https://$hostName/.well-known/windows-app-web-link names the package; an unverified host link opens in the browser. Pass -OpenHostLink to open it anyway")
        return
    }
    $firstId = $script:process.Id
    Start-Process $link
    $route = Wait-LinkRoute '/alpha' 'x=3' 15
    if ($null -ne $route) {
        Wait-OnePackagedProcess $firstId
        Write-Ok "the shell opened $link in the running package at $route"
        return
    }
    if ($RequireHostLink) { throw "the shell did not open $link in the running package ($support)" }
    if ($support -eq 'Available') {
        Write-Skip ("the shell sent $link to the browser although Windows lists the package as its handler: $hostName serves no " +
            "windows-app-web-link, so Windows never verified it; opening a host link needs a verified domain")
        return
    }
    Write-Skip "the shell did not open $link in the package ($support): it went to the default browser, as it does for an unverified host"
}

function Test-PackagedApp([string] $ty, [string] $fixture, [string] $scheme, [string] $hostName) {
    Write-Host '==> MSIX package with every browser API granted, a scheme and a host'
    $identity = 'dev.tachyon.browser-gate-package'
    $toastTitle = "Tachyon toast $($script:token)"
    Write-BrowserFixture $fixture $identity @('clipboard', 'share', 'notifications', 'geolocation') $scheme $hostName 'Packaged copy' $toastTitle
    $pfx = Join-Path $env:TEMP ('tachyon-browser-gate-' + [guid]::NewGuid().ToString('N') + '.pfx')
    $certificate = $null
    try {
        $certificate = New-PackageCertificate 'CN=Tachyon Browser Gate' $pfx
        Invoke-Tool 'ty bundle --release' "`"$ty`" bundle `"$fixture`" --target windows --release"
        $layout = Join-Path $fixture 'dist\windows\BrowserGate'
        $package = Join-Path $layout 'BrowserGate.msix'
        Invoke-Check 'package signature' { Assert-PackageSignature $package $certificate }
        Invoke-Check 'layout manifest' {
            Test-LinkManifest ([xml](Get-Content -Raw -LiteralPath (Join-Path $layout 'AppxManifest.xml'))) 'the packed layout' $scheme $hostName
        }
        $route = Install-GatePackage $package $layout $certificate $identity
        if ($null -eq $route) {
            Write-Skip 'the toast, the deep links and the host link from the installed package: no install route worked (see the NOTE lines)'
            return
        }
        Invoke-Check 'installed manifest' {
            Test-LinkManifest (Get-AppxPackageManifest -Package $script:installed.PackageFullName) 'the installed package' $scheme $hostName
        }
        Invoke-Check 'windows-app-web-link' { Test-WebLinkFile $fixture }
        try { Start-PackagedApp }
        catch {
            Add-Failure "installed package: $($_.Exception.Message)"
            Write-PageTexts
            return
        }
        Invoke-Check 'packaged toast' { Test-PackagedToast $toastTitle }
        Invoke-Check 'packaged deep links' { Test-PackagedDeepLinks $scheme }
        Invoke-Check 'packaged host link' { Test-PackagedHostLink $hostName }
    }
    finally {
        Stop-AppProcesses
        Remove-GatePackage
        Remove-TrustedCertificate
        Remove-PackageCertificate $certificate $pfx
    }
}

function Write-Summary {
    if ($script:failures.Count -eq 0) {
        Write-Host 'PASS: Windows WebView2 browser APIs, MSIX toast, deep links and host link checks (see SKIP lines for what did not run)'
        return
    }
    Write-Host "FAILED: $($script:failures.Count) check(s)"
    foreach ($failure in $script:failures) { Write-Host " - $failure" }
    exit 1
}

function Invoke-BrowserApisGate {
    $root = Join-Path $env:TEMP ('ty-windows-browser-apis-' + [guid]::NewGuid().ToString('N'))
    $saved = $null
    try {
        Assert-DesktopSession
        Enable-DesktopInput
        $saved = Read-WindowsClipboard
        Enable-Msvc
        Enable-WebViewSdk
        $ty = Resolve-Ty
        if ($SkipUnpackaged) { Write-Skip 'the unpackaged apps (-SkipUnpackaged)' }
        else {
            Test-GrantedApp $ty (Join-Path $root 'granted') "Copied by Tachyon $($script:token)"
            Test-RefusedApp $ty (Join-Path $root 'refused')
        }
        if ($SkipPackage) { Write-Skip 'the MSIX package (-SkipPackage)' }
        else { Test-PackagedApp $ty (Join-Path $root 'package') 'sd1check' $LinkHost }
    }
    catch { Add-Failure "the gate stopped: $($_.Exception.Message)" }
    finally {
        if ($saved) { Write-WindowsClipboard $saved }
        Remove-Fixture $root
    }
    Write-Summary
}

Invoke-BrowserApisGate
