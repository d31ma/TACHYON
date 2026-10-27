param(
    [string] $OutputDirectory = (Join-Path $env:TEMP 'tachyon-wm1-shots'),
    [int] $ScaleChangeSeconds = 0,
    [switch] $AllowStandardDpi
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot 'desktop-gate-common.ps1')

function Add-DpiTypes {
    if ('TachyonDpi' -as [type]) { return }
    Add-Type -Language CSharp -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class TachyonDpi
{
    [StructLayout(LayoutKind.Sequential)]
    public struct Box { public int Left; public int Top; public int Right; public int Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    struct MonitorInfo { public int Size; public Box Monitor; public Box Work; public uint Flags; }

    public struct Screen { public IntPtr Monitor; public Box Work; public uint Dpi; }

    delegate bool MonitorVisitor(IntPtr monitor, IntPtr context, IntPtr box, IntPtr data);

    [DllImport("user32.dll")] static extern bool EnumDisplayMonitors(IntPtr context, IntPtr clip, MonitorVisitor visitor, IntPtr data);
    [DllImport("user32.dll")] static extern bool GetMonitorInfoW(IntPtr monitor, ref MonitorInfo info);
    [DllImport("user32.dll")] static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll")] static extern IntPtr GetWindowDpiAwarenessContext(IntPtr window);
    [DllImport("user32.dll")] static extern bool AreDpiAwarenessContextsEqual(IntPtr first, IntPtr second);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("shcore.dll")] static extern int GetDpiForMonitor(IntPtr monitor, int kind, out uint x, out uint y);

    public static bool PerMonitorV2(IntPtr window)
    {
        return AreDpiAwarenessContextsEqual(GetWindowDpiAwarenessContext(window), new IntPtr(-4));
    }

    public static IntPtr MonitorOf(IntPtr window) { return MonitorFromWindow(window, 2); }

    public static Screen[] Screens()
    {
        List<Screen> found = new List<Screen>();
        MonitorVisitor visitor = delegate (IntPtr monitor, IntPtr context, IntPtr box, IntPtr data)
        {
            MonitorInfo info = new MonitorInfo();
            info.Size = Marshal.SizeOf(typeof(MonitorInfo));
            uint x, y;
            if (GetMonitorInfoW(monitor, ref info) && GetDpiForMonitor(monitor, 0, out x, out y) == 0)
            {
                Screen screen = new Screen();
                screen.Monitor = monitor; screen.Work = info.Work; screen.Dpi = x;
                found.Add(screen);
            }
            return true;
        };
        EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, visitor, IntPtr.Zero);
        GC.KeepAlive(visitor);
        return found.ToArray();
    }

    public static void Size(IntPtr window, int width, int height)
    {
        SetWindowPos(window, IntPtr.Zero, 0, 0, width, height, 0x0002 | 0x0004 | 0x0010);
    }
}
'@
}

function Write-SetupFixture([string] $root) {
    Write-FixtureFile $root 'client/manifest.json' @'
{
  "name": "Dpi Setup",
  "start_url": "/",
  "tac": {
    "application": {
      "id": "dev.tachyon.dpi-setup",
      "version": "0.0.1",
      "window": { "width": 720, "height": 480, "minWidth": 600, "minHeight": 400, "controls": ["resize"] }
    },
    __CHECKS__
  }
}
'@
    Write-FixtureFile $root 'client/pages/tac.html' "<dpi-setup hydrate=`"load`" />`n"
    Write-FixtureFile $root 'client/components/dpi/setup/tac.html' @'
<main aria-label="Dpi setup">
  <h1>Setup route</h1>
  <p>Ratio {dpiWindow.ratio}</p>
  <p>Viewport {dpiWindow.viewport}</p>
  <p>Problem {dpiWindow.problem}</p>
  <p>Reads {dpiWindow.reads}</p>
  <button on:click="dpiWindow.read()">Read size</button>
  <button on:click="dpiWindow.resizeWindow()">Resize window</button>
</main>
'@
    Write-FixtureFile $root 'client/services/DpiWindowService.js' @'
@Service
export default class DpiWindowService {
  /** @type {TacWindowControls} */
  @Needs
  windowControls;
  ratio = 'unread';
  viewport = 'unread';
  problem = 'none';
  reads = 0;

  read() {
    const page = document.documentElement;
    this.ratio = String(devicePixelRatio);
    this.viewport = `${page.clientWidth}x${page.clientHeight}`;
    this.reads += 1;
  }

  resizeWindow() {
    try {
      this.windowControls.resize(640, 480);
    } catch (refusal) {
      this.problem = /** @type {Error} */ (refusal).message;
    }
  }
}
'@
}

function Get-Scaled([int] $points, [double] $dpi) {
    return [int][Math]::Round($points * $dpi / 96, [MidpointRounding]::AwayFromZero)
}

function Start-UnelevatedApp([string] $executable, [string] $applicationId) {
    if (-not (Test-Path $executable)) { throw "missing artifact $executable" }
    $script:log = Join-Path $env:LOCALAPPDATA "Tachyon\$applicationId.jsonl"
    if (Test-Path $script:log) { Remove-Item $script:log }
    $start = [Diagnostics.ProcessStartInfo]::new($executable)
    $start.UseShellExecute = $false
    try { $script:process = [Diagnostics.Process]::Start($start) }
    catch {
        $failure = $_.Exception
        while ($null -ne $failure -and -not ($failure -is [ComponentModel.Win32Exception])) { $failure = $failure.InnerException }
        if ($null -ne $failure -and $failure.NativeErrorCode -eq 740) { throw 'Windows asks for administrator rights to start the app (error 740)' }
        throw
    }
    $script:window = [IntPtr]::Zero
    Wait-Until {
        Assert-Running
        $found = @([TachyonDesktop]::Windows($script:process.Id, 'TachyonNativeHost', $true))
        if ($found.Count -gt 0) { $script:window = $found[0] }
        $script:window -ne [IntPtr]::Zero
    } 30 'the app window'
    Write-Host ("Started {0} without the shell (pid {1}, window 0x{2:X})" -f $executable, $script:process.Id, $script:window.ToInt64())
}

function Read-PageSize {
    $reads = [int]((Wait-Matching '^Reads \d+$' 10) -replace '^Reads ', '')
    Invoke-Named 'Read size'
    Wait-Named @("Reads $($reads + 1)") 10 | Out-Null
    $ratio = Wait-Matching '^Ratio [\d.]+$' 10
    $viewport = Wait-Matching '^Viewport \d+x\d+$' 10
    $size = ($viewport -replace '^Viewport ', '') -split 'x'
    return [pscustomobject]@{ Ratio = [double]($ratio -replace '^Ratio ', ''); Width = [int]$size[0]; Height = [int]$size[1] }
}

function Assert-ClientPoints([int] $width, [int] $height, [int] $tolerance, [string] $what) {
    $dpi = [double][TachyonDesktop]::Dpi($script:window)
    $client = [TachyonDesktop]::ClientBox($script:window)
    $pixels = @(($client.Right - $client.Left), ($client.Bottom - $client.Top))
    $wanted = @((Get-Scaled $width $dpi), (Get-Scaled $height $dpi))
    Assert-That ([Math]::Abs($pixels[0] - $wanted[0]) -le $tolerance -and [Math]::Abs($pixels[1] - $wanted[1]) -le $tolerance) (
        "$what is $($pixels[0]) by $($pixels[1]) pixels at $dpi DPI; $width by $height DIP is $($wanted[0]) by $($wanted[1])")
    $page = Read-PageSize
    Assert-That ([Math]::Abs($page.Ratio - $dpi / 96) -le 0.01) "the page reads devicePixelRatio $($page.Ratio) at $dpi DPI"
    Assert-That ([Math]::Abs($page.Width - $width) -le $tolerance -and [Math]::Abs($page.Height - $height) -le $tolerance) (
        "the page reads a $($page.Width) by $($page.Height) viewport, not $width by $height")
    Write-Host "OK: $what is $($pixels[0]) by $($pixels[1]) pixels at $dpi DPI, and the page reads a $($page.Width) by $($page.Height) viewport at devicePixelRatio $($page.Ratio)"
}

function Test-ManifestEmbedded([string] $fixture) {
    $executable = Get-AppExecutable $fixture 'DpiSetup'
    Test-Check 'the executable embeds the application manifest' {
        $text = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($executable))
        foreach ($embedded in @('<requestedExecutionLevel level="asInvoker"', 'PerMonitorV2', 'Microsoft.Windows.Common-Controls')) {
            Assert-That ($text.Contains($embedded)) "DpiSetup.exe holds no '$embedded'"
        }
        $loose = @(Get-ChildItem -LiteralPath (Join-Path $fixture 'dist\windows\DpiSetup') -Recurse -Filter '*.manifest' | ForEach-Object { $_.FullName })
        Assert-That ($loose.Count -eq 0) "the build still writes a loose manifest: $($loose -join ', ')"
        Write-Host 'OK: DpiSetup.exe embeds the manifest (asInvoker, PerMonitorV2, Common Controls 6) and no loose .manifest file is written'
    }
}

function Test-ScaledWindow {
    Test-Check 'the window is per-monitor DPI aware (v2)' {
        Assert-That ([TachyonDpi]::PerMonitorV2($script:window)) 'the window does not have the per-monitor v2 DPI awareness context'
        Write-Host 'OK: GetWindowDpiAwarenessContext of the app window equals DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2'
    }
    Test-Check 'the Common Controls version the process loaded' {
        try {
            $script:process.Refresh()
            $controls = @($script:process.Modules | Where-Object { $_.ModuleName -ieq 'comctl32.dll' })
            if ($controls.Count -eq 0) { Write-Host 'NOTE: the app has not loaded comctl32.dll' }
            foreach ($module in $controls) { Write-Host "NOTE: the app loaded $($module.FileName) ($($module.FileVersionInfo.FileVersion))" }
        }
        catch { Write-Host "NOTE: the modules of the app could not be read: $($_.Exception.Message)" }
    }
    Test-Check 'the desktop is scaled above 100%' {
        $dpi = [TachyonDesktop]::Dpi($script:window)
        if ($dpi -eq 96 -and $AllowStandardDpi) { Write-Host 'NOTE: the desktop is at 96 DPI (100%), so the size checks below do not prove scaling' }
        else { Assert-That ($dpi -gt 96) 'the desktop is at 96 DPI (100%); set 150% or 200% in Settings > Display, or pass -AllowStandardDpi' }
        Write-Host ("OK: the app window is at {0} DPI ({1:P0})" -f $dpi, ($dpi / 96))
    }
    Test-Check 'the client area opens at 720 by 480 DIP' {
        Wait-Named @('Setup route', 'Read size') | Out-Null
        Assert-ClientPoints 720 480 1 'the first client area'
    }
    Test-Check 'the minimum size holds at this DPI' {
        Show-AppWindow
        [TachyonDpi]::Size($script:window, 100, 100)
        Start-Sleep -Milliseconds 400
        $dpi = [double][TachyonDesktop]::Dpi($script:window)
        $client = [TachyonDesktop]::ClientBox($script:window)
        $wanted = @((Get-Scaled 600 $dpi), (Get-Scaled 400 $dpi))
        $pixels = @(($client.Right - $client.Left), ($client.Bottom - $client.Top))
        Assert-That ($pixels[0] -ge $wanted[0] - 1 -and $pixels[1] -ge $wanted[1] - 1) (
            "SetWindowPos(100, 100) left a $($pixels[0]) by $($pixels[1]) client area; the minimum is $($wanted[0]) by $($wanted[1]) pixels")
        Write-Host "OK: SetWindowPos(100, 100) stopped at a $($pixels[0]) by $($pixels[1]) client area (minimum 600 by 400 DIP = $($wanted[0]) by $($wanted[1]) pixels)"
        $frame = [TachyonDesktop]::WindowBox($script:window)
        $corner = @(($frame.Right - 4), ($frame.Bottom - 4))
        $hit = [TachyonDesktop]::HitTest($script:window, $corner[0], $corner[1])
        if ($hit -ne 17) { Write-Host "NOTE: the bottom-right corner ($($corner[0]),$($corner[1])) answers $hit, not HTBOTTOMRIGHT; no drag" }
        else {
            [TachyonDesktop]::Drag($corner[0], $corner[1], -300, -300, 12)
            Start-Sleep -Milliseconds 400
            $client = [TachyonDesktop]::ClientBox($script:window)
            $pixels = @(($client.Right - $client.Left), ($client.Bottom - $client.Top))
            Assert-That ($pixels[0] -ge $wanted[0] - 1 -and $pixels[1] -ge $wanted[1] - 1) (
                "a corner drag left a $($pixels[0]) by $($pixels[1]) client area; the minimum is $($wanted[0]) by $($wanted[1]) pixels")
            Write-Host "OK: a SendInput drag of the bottom-right corner stopped at a $($pixels[0]) by $($pixels[1]) client area"
        }
    }
    Test-Check 'resize(640, 480) from the page gives 640 by 480 DIP' {
        $before = [TachyonDesktop]::WindowBox($script:window)
        Invoke-Named 'Resize window'
        Wait-Until { ([TachyonDesktop]::WindowBox($script:window)).Right -ne $before.Right } 5 'the window to resize'
        Start-Sleep -Milliseconds 300
        Assert-That ((Wait-Matching '^Problem ' 5) -eq 'Problem none') 'the page call failed'
        Assert-ClientPoints 640 480 1 'the resized client area'
    }
}

function Wait-DpiChange([uint32] $from, [int] $seconds) {
    Wait-Until { [TachyonDesktop]::Dpi($script:window) -ne $from } $seconds 'the window DPI to change'
    Start-Sleep -Milliseconds 800
    $now = [TachyonDesktop]::Dpi($script:window)
    $screen = @([TachyonDpi]::Screens() | Where-Object { $_.Monitor -eq [TachyonDpi]::MonitorOf($script:window) })
    if ($screen.Count -gt 0) { Assert-That ($screen[0].Dpi -eq $now) "the window is at $now DPI on a $($screen[0].Dpi) DPI monitor" }
    Write-Host "OK: the window DPI changed from $from to $now"
}

function Test-DpiChange {
    $script:dpiChange = 'not run'
    $from = [TachyonDesktop]::Dpi($script:window)
    $other = @([TachyonDpi]::Screens() | Where-Object { $_.Dpi -ne $from } | Select-Object -First 1)
    if ($other.Count -gt 0) {
        Test-Check 'a move to a monitor with another scale (WM_DPICHANGED)' {
            $first = [TachyonDesktop]::WindowBox($script:window)
            [TachyonDesktop]::Place($script:window, $other[0].Work.Left + 40, $other[0].Work.Top + 40)
            Wait-DpiChange $from 10
            Assert-ClientPoints 640 480 3 "the client area on the $($other[0].Dpi) DPI monitor"
            [TachyonDesktop]::Place($script:window, $first.Left, $first.Top)
            Wait-DpiChange $other[0].Dpi 10
            Assert-ClientPoints 640 480 3 'the client area back on the first monitor'
            $script:dpiChange = 'monitor move'
        }
        return
    }
    if ($ScaleChangeSeconds -le 0) {
        Write-Host "NOT RUN: every monitor is at $from DPI; attach one with another scale, or pass -ScaleChangeSeconds and change the scale in Settings > Display during the run"
        return
    }
    Test-Check 'a scale change while the app runs (WM_DPICHANGED)' {
        Write-Host "ACTION: change the display scale in Settings > Display now (waiting $ScaleChangeSeconds s)"
        Wait-DpiChange $from $ScaleChangeSeconds
        Assert-ClientPoints 640 480 3 'the client area after the scale change'
        $script:dpiChange = 'scale change'
    }
}

function Test-SetupApp([string] $fixture) {
    Test-ManifestEmbedded $fixture
    Test-Check 'an app named DpiSetup starts without asking for administrator rights' {
        Start-UnelevatedApp (Get-AppExecutable $fixture 'DpiSetup') 'dev.tachyon.dpi-setup'
        Write-Host 'OK: CreateProcess started DpiSetup.exe from an unelevated process (no error 740)'
    }
    if ($null -eq $script:process) { return }
    Test-ScaledWindow
    Test-DpiChange
}

function Invoke-HighDpiGate {
    Initialize-DesktopGate $OutputDirectory
    Add-DpiTypes
    Enable-Msvc (Get-HostArchitecture)
    Enable-WebViewSdk
    $ty = Resolve-Ty
    $script:dpiChange = 'not run'
    Invoke-GateApp 'Dpi Setup' ${function:Write-SetupFixture} $ty 'native' ${function:Test-SetupApp}
    Write-Host "DPI change check: $($script:dpiChange)"
    Complete-Gate 'the embedded manifest (asInvoker, PerMonitorV2), an app named Setup starting without elevation, and the first size, minimum size and page resize in DIP on a scaled desktop'
}

Invoke-HighDpiGate
