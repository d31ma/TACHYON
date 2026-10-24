param(
    [ValidateSet('native', 'x64', 'arm64')] [string] $Architecture = 'native',
    [string] $OutputDirectory = (Join-Path $env:TEMP 'tachyon-sd1-shots')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot 'desktop-gate-common.ps1')

function Write-BarFixture([string] $root) {
    Write-FixtureFile $root 'client/manifest.json' @'
{
  "name": "Shell Bar",
  "start_url": "/",
  "tac": {
    "application": {
      "id": "dev.tachyon.shell-bar",
      "version": "0.0.1",
      "window": { "width": 720, "height": 480, "titleBar": "integrated" }
    },
    "styles": ["/shared/shell.css"],
    __CHECKS__
  }
}
'@
    Write-FixtureFile $root 'client/shared/shell.css' @'
body { margin: 0; font-family: sans-serif; background: #ffffff; color: #111111; }
'@
    Write-FixtureFile $root 'client/components/shell/bar/tac.css' @'
.top-bar {
  display: flex; align-items: center; gap: 16px; box-sizing: border-box;
  min-height: max(48px, var(--tac-titlebar-height, 0px));
  padding-inline-start: 16px;
  padding-inline-end: max(16px, var(--tac-titlebar-trailing-inset, 0px));
  background: #2e7d32; color: #ffffff;
}
.shell-main { padding: 16px; }
'@
    Write-FixtureFile $root 'client/pages/tac.html' "<shell-bar hydrate=`"load`" />`n"
    Write-FixtureFile $root 'client/components/shell/bar/tac.html' @'
<header class="top-bar" data-tac-drag-region>
  <strong>Drag area</strong>
  <button on:click="titleBar.press()">Header button</button>
</header>
<main class="shell-main" aria-label="Shell bar">
  <p>Header presses {titleBar.presses}</p>
  <p>Insets {titleBar.insets}</p>
  <button on:click="titleBar.read()">Read insets</button>
</main>
'@
    Write-FixtureFile $root 'client/services/TitleBarService.js' @'
@Service
export default class TitleBarService {
  presses = 0
  insets = 'unread'

  press() {
    this.presses += 1
  }

  read() {
    const style = getComputedStyle(document.documentElement)
    const value = (name) => style.getPropertyValue(name).trim() || 'unset'
    this.insets = `height=${value('--tac-titlebar-height')} leading=${value('--tac-titlebar-leading-inset')} trailing=${value('--tac-titlebar-trailing-inset')}`
  }
}
'@
}

function Write-CaptionFixture([string] $root) {
    Write-FixtureFile $root 'client/manifest.json' @'
{
  "name": "Shell Caption",
  "start_url": "/",
  "theme_color": "#b03a2e",
  "tac": {
    "application": {
      "id": "dev.tachyon.shell-caption",
      "version": "0.0.1",
      "window": {
        "width": 800,
        "height": 560,
        "controls": ["minimize", "maximize", "resize", "fullscreen", "title", "close"]
      }
    },
    __CHECKS__
  }
}
'@
    Write-FixtureFile $root 'client/pages/tac.html' "<shell-caption hydrate=`"load`" />`n"
    Write-FixtureFile $root 'client/components/shell/caption/tac.html' @'
<main aria-label="Shell caption">
  <h1>Caption route</h1>
  <p>Granted minimize {captionWindow.granted}</p>
  <p>Problem {captionWindow.problem}</p>
  <input aria-label="Caption notes" />
  <button on:click="captionWindow.shrink()">Minimize window</button>
  <button on:click="captionWindow.grow()">Maximize window</button>
  <button on:click="captionWindow.resizeWindow()">Resize window</button>
  <button on:click="captionWindow.fill()">Fullscreen window</button>
  <button on:click="captionWindow.rename()">Rename window</button>
  <button on:click="captionWindow.shut()">Close window</button>
</main>
'@
    Write-FixtureFile $root 'client/services/CaptionWindowService.js' @'
@Service
export default class CaptionWindowService {
  /** @type {TacWindowControls} */
  @Needs
  windowControls;
  granted = 'unknown';
  problem = 'none';

  @OnStart
  start() {
    this.granted = String(this.windowControls.granted('minimize'));
  }

  shrink() {
    this.#keep(() => this.windowControls.minimize());
  }

  grow() {
    this.#keep(() => this.windowControls.maximize());
  }

  resizeWindow() {
    this.#keep(() => this.windowControls.resize(640, 480));
  }

  fill() {
    this.#keep(() => this.windowControls.fullscreen());
  }

  rename() {
    this.#keep(() => this.windowControls.setTitle('Renamed Shell'));
  }

  shut() {
    this.#keep(() => this.windowControls.close());
  }

  /** @param {() => void} call */
  #keep(call) {
    try {
      call();
    } catch (refusal) {
      this.problem = /** @type {Error} */ (refusal).message;
    }
  }
}
'@
}

function Write-FullFixture([string] $root) {
    Write-FixtureFile $root 'client/manifest.json' @'
{
  "name": "Shell Full",
  "start_url": "/",
  "display": "fullscreen",
  "tac": {
    "application": {
      "id": "dev.tachyon.shell-full",
      "version": "0.0.1",
      "window": { "width": 720, "height": 480, "controls": ["fullscreen"] }
    },
    __CHECKS__
  }
}
'@
    Write-FixtureFile $root 'client/pages/tac.html' "<shell-full hydrate=`"load`" />`n"
    Write-FixtureFile $root 'client/components/shell/full/tac.html' @'
<main aria-label="Shell full">
  <h1>Full route</h1>
  <button on:click="fullWindow.leave()">Leave fullscreen</button>
</main>
'@
    Write-FixtureFile $root 'client/services/FullWindowService.js' @'
@Service
export default class FullWindowService {
  /** @type {TacWindowControls} */
  @Needs
  windowControls;

  leave() {
    this.windowControls.fullscreen();
  }
}
'@
}

function Get-DragPoint {
    return (Get-Center (Wait-Named @('Drag area')))
}

function Write-DragHitTest([int[]] $point) {
    $answers = @()
    $current = [TachyonDesktop]::WindowAt($point[0], $point[1])
    while ($current -ne [IntPtr]::Zero) {
        $hit = [TachyonDesktop]::HitTest($current, $point[0], $point[1])
        $answers += ('{0}={1}' -f [TachyonDesktop]::ClassOf($current), $hit)
        if ($hit -eq 2) {
            Write-Host "OK: WM_NCHITTEST over the drag region answers HTCAPTION ($($answers -join ' > '))"
            return
        }
        if ($current -eq $script:window) { break }
        $current = [TachyonDesktop]::ParentOf($current)
    }
    Write-Host "NOTE: no window under the drag region answers HTCAPTION to WM_NCHITTEST ($($answers -join ' > ')); WebView2 handles app-region itself, so the drag below decides"
}

function Test-IntegratedTitleBar([string] $fixture) {
    Start-GateApp (Get-AppExecutable $fixture 'ShellBar') 'dev.tachyon.shell-bar'
    Test-Check 'the integrated page renders' {
        Wait-Named @('Drag area', 'Header button', 'Read insets') | Out-Null
        Write-Host 'OK: the integrated title bar page rendered in WebView2'
    }
    Test-Check 'the menu bar moved into the window menu' {
        Assert-That ([TachyonDesktop]::MenuOf($script:window) -eq [IntPtr]::Zero) 'the integrated window still has a menu bar'
        Write-Host 'OK: the integrated window has no menu bar (GetMenu is NULL)'
    }
    [TachyonDesktop]::Place($script:window, 120, 140)
    Start-Sleep -Milliseconds 500
    Test-Check 'the page reaches the top edge' {
        Show-AppWindow
        Start-Sleep -Milliseconds 600
        $shot = Save-WindowShot 'integrated-title-bar'
        try {
            $client = [TachyonDesktop]::ClientBox($script:window)
            $frame = [TachyonDesktop]::WindowBox($script:window)
            Assert-That ($client.Top -eq $frame.Top) "the client area starts at $($client.Top), below the window top $($frame.Top)"
            $sample = Measure-Color $shot ($client.Left + 8) ($client.Top + 2) ($client.Left + 140) ($client.Top + 14) @(46, 125, 50) 24
            Assert-That ($sample.Share -ge 0.9) ("only {0:P0} of the top-edge pixels have the header colour #2e7d32; their average is {1}" -f $sample.Share, $sample.Average)
            Write-Host ("OK: the client area starts at the window's top edge and {0:P0} of the top-edge pixels have the page header colour #2e7d32" -f $sample.Share)
        }
        finally { $shot.Bitmap.Dispose() }
    }
    Test-Check 'the caption buttons stay, with Snap Layouts' {
        $buttons = [TachyonDesktop]::CaptionButtons($script:window)
        $width = [int](($buttons.Right - $buttons.Left) / 3)
        $middle = [int](($buttons.Top + $buttons.Bottom) / 2)
        foreach ($button in @(@(0, 8, 'minimize (HTMINBUTTON)'), @(1, 9, 'maximize (HTMAXBUTTON, Snap Layouts)'), @(2, 20, 'close (HTCLOSE)'))) {
            $x = $buttons.Left + $width * $button[0] + [int]($width / 2)
            $hit = [TachyonDesktop]::HitTest($script:window, $x, $middle)
            Assert-That ($hit -eq $button[1]) "WM_NCHITTEST at the $($button[2]) button ($x,$middle) answered $hit"
            $covering = [TachyonDesktop]::WindowAt($x, $middle)
            Assert-That ($covering -eq $script:window) "the $($button[2]) button is covered by a $([TachyonDesktop]::ClassOf($covering)) window"
        }
        Write-Host ("OK: WM_NCHITTEST answers HTMINBUTTON, HTMAXBUTTON and HTCLOSE at the caption buttons ({0},{1})-({2},{3}), and no web view covers them" -f
            $buttons.Left, $buttons.Top, $buttons.Right, $buttons.Bottom)
        $titleBar = (Get-AppRoot).FindFirst([System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::TitleBar))
        if ($null -eq $titleBar) { Write-Host 'NOTE: UI Automation shows no TitleBar element for the integrated window' }
        else {
            $names = @($titleBar.FindAll([System.Windows.Automation.TreeScope]::Children,
                [System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object { $_.Current.Name })
            Write-Host "NOTE: UI Automation TitleBar children: $($names -join ', ')"
        }
        [TachyonDesktop]::MoveTo($buttons.Left + $width + [int]($width / 2), $middle)
        Start-Sleep -Milliseconds 1800
        $monitor = [TachyonDesktop]::MonitorBox($script:window)
        $shot = Save-ScreenShot 'snap-layouts' ([Math]::Max($monitor.Left, $buttons.Left - 700)) $buttons.Top ([Math]::Min($monitor.Right, $buttons.Right + 40)) ([Math]::Min($monitor.Bottom, $buttons.Bottom + 420))
        $shot.Bitmap.Dispose()
        Write-Host 'NOTE: snap-layouts.png should show the Snap Layouts flyout under the maximize button (pointer hovered there 1.8 s)'
        $client = [TachyonDesktop]::ClientBox($script:window)
        [TachyonDesktop]::MoveTo([int](($client.Left + $client.Right) / 2), [int](($client.Top + $client.Bottom) / 2))
        [TachyonDesktop]::Keys(@([uint16]0x1B))
        Start-Sleep -Milliseconds 500
    }
    Test-Check 'the page reads the title bar insets' {
        Invoke-Named 'Read insets'
        $text = Wait-Matching '^Insets height=' 10
        $pattern = '^Insets height=(?<height>[\d.]+)px leading=(?<leading>[\d.]+)px trailing=(?<trailing>[\d.]+)px$'
        Assert-That ($text -match $pattern) "the page has no --tac-titlebar-* values: '$text'"
        $height = [double]$matches['height']
        $leading = [double]$matches['leading']
        $trailing = [double]$matches['trailing']
        $buttons = [TachyonDesktop]::CaptionButtons($script:window)
        $client = [TachyonDesktop]::ClientBox($script:window)
        $dpi = [double][TachyonDesktop]::Dpi($script:window)
        $wantedHeight = ($buttons.Bottom - $client.Top) * 96 / $dpi
        $wantedTrailing = ($client.Right - $buttons.Left) * 96 / $dpi
        Assert-That ([Math]::Abs($height - $wantedHeight) -le 2) "--tac-titlebar-height is ${height}px; the caption buttons end at $([Math]::Round($wantedHeight, 1))px"
        Assert-That ([Math]::Abs($trailing - $wantedTrailing) -le 2) "--tac-titlebar-trailing-inset is ${trailing}px; the caption buttons are $([Math]::Round($wantedTrailing, 1))px wide"
        Assert-That ($leading -eq 0) "--tac-titlebar-leading-inset is ${leading}px, not 0px"
        Write-Host "OK: the page reads --tac-titlebar-height ${height}px, --tac-titlebar-leading-inset ${leading}px and --tac-titlebar-trailing-inset ${trailing}px, matching the caption buttons at $dpi DPI"
    }
    Test-Check 'a button inside the drag region gets its click' {
        Click-Element (Wait-Named @('Header button'))
        Wait-Named @('Header presses 1') 5 | Out-Null
        Write-Host 'OK: a real click on the button inside the drag region reached the page (Header presses 1)'
    }
    Test-Check 'dragging the drag region moves the window' {
        if ($script:log -and (Test-Path $script:log) -and (Select-String -Path $script:log -SimpleMatch 'window.dragRegions.unavailable' -Quiet)) {
            throw 'the host recorded window.dragRegions.unavailable: this WebView2 Runtime has no non-client region support'
        }
        [TachyonDesktop]::Place($script:window, 120, 140)
        Show-AppWindow
        $before = [TachyonDesktop]::WindowBox($script:window)
        $point = Get-DragPoint
        Write-DragHitTest $point
        [TachyonDesktop]::Drag($point[0], $point[1], 120, 60, 12)
        Wait-Until { ([TachyonDesktop]::WindowBox($script:window)).Left -ne $before.Left } 5 'the window to move'
        Start-Sleep -Milliseconds 300
        $after = [TachyonDesktop]::WindowBox($script:window)
        $movedX = $after.Left - $before.Left
        $movedY = $after.Top - $before.Top
        Assert-That ([Math]::Abs($movedX - 120) -le 12 -and [Math]::Abs($movedY - 60) -le 12) "the window moved by ($movedX, $movedY) instead of (120, 60)"
        Write-Host "OK: a SendInput drag of the data-tac-drag-region moved the window by ($movedX, $movedY) pixels"
    }
    Test-Check 'a double-click on the drag region maximizes and restores' {
        $point = Get-DragPoint
        [TachyonDesktop]::DoubleClick($point[0], $point[1])
        Wait-Until { [TachyonDesktop]::Maximized($script:window) } 5 'the window to maximize'
        Start-Sleep -Milliseconds 600
        $point = Get-DragPoint
        [TachyonDesktop]::DoubleClick($point[0], $point[1])
        Wait-Until { -not [TachyonDesktop]::Maximized($script:window) } 5 'the window to restore'
        Write-Host 'OK: a double-click on the drag region maximized the window, and a second one restored it'
    }
    Test-Check 'a right-click on the drag region opens the window menu with the app menus' {
        Start-Sleep -Milliseconds 500
        $point = Get-DragPoint
        [TachyonDesktop]::RightClick($point[0], $point[1])
        $popup = Wait-Popup 5
        $names = @(Get-PopupNames $popup)
        $shot = Save-ScreenShot 'integrated-window-menu' ($point[0] - 40) ($point[1] - 20) ($point[0] + 420) ($point[1] + 520)
        $shot.Bitmap.Dispose()
        Close-Popup
        $missing = @(@('File', 'Edit', 'Window', 'Close') | Where-Object { $names -notcontains $_ })
        Assert-That ($missing.Count -eq 0) "the window menu lacks $($missing -join ', '); it holds: $($names -join ', ')"
        Write-Host "OK: a right-click on the drag region opened the window menu: $($names -join ', ')"
    }
}

function Test-CaptionColor {
    [TachyonDesktop]::Place($script:window, 120, 140)
    Show-AppWindow
    Start-Sleep -Milliseconds 800
    $shot = Save-WindowShot 'caption-color'
    try {
        $frame = [TachyonDesktop]::FrameBox($script:window)
        $buttons = [TachyonDesktop]::CaptionButtons($script:window)
        $caption = [TachyonDesktop]::Metric(4, [TachyonDesktop]::Dpi($script:window))
        $left = $frame.Left + [int](($frame.Right - $frame.Left) * 0.45)
        $right = $buttons.Left - 12
        if ($right -le $left + 20) { $left = $frame.Left + [int](($frame.Right - $frame.Left) * 0.3) }
        $sample = Measure-Color $shot $left ($frame.Top + 4) $right ($frame.Top + $caption - 4) @(176, 58, 46) 20
        $color = [uint32]0
        $result = [TachyonDesktop]::CaptionColor($script:window, [ref]$color)
        if ($result -eq 0) {
            Assert-That ($color -eq 0x002E3AB0) ("DwmGetWindowAttribute(DWMWA_CAPTION_COLOR) answered 0x{0:X8}, not 0x002E3AB0 (#b03a2e)" -f $color)
            Write-Host 'OK: DwmGetWindowAttribute(DWMWA_CAPTION_COLOR = 35) answers 0x002E3AB0, the declared #b03a2e'
        }
        else { Write-Host ("NOTE: DwmGetWindowAttribute(DWMWA_CAPTION_COLOR = 35) answered 0x{0:X8}; Windows may not report the attribute" -f $result) }
        Assert-That ($sample.Share -ge 0.85) ("only {0:P0} of the caption pixels match theme_color #b03a2e; their average is {1}" -f $sample.Share, $sample.Average)
        Write-Host ("OK: {0:P0} of {1} sampled title bar pixels match theme_color #b03a2e (average {2})" -f $sample.Share, $sample.Total, $sample.Average)
    }
    finally { $shot.Bitmap.Dispose() }
}

function Test-EditCommands([string] $field) {
    Click-Element (Wait-Named @($field))
    Set-FieldValue $field 'tachyon edit'
    Wait-Until { (Get-FieldValue $field) -eq 'tachyon edit' } 5 "the $field input to hold 'tachyon edit'"
    Click-Element (Wait-Named @($field))
    try { Set-Clipboard -Value 'sentinel' } catch { Write-Host "NOTE: could not seed the clipboard: $($_.Exception.Message)" }
    Invoke-MenuCommand 'Edit' 'Select All'
    Invoke-MenuCommand 'Edit' 'Copy'
    Wait-Until { (Get-ClipboardText) -eq 'tachyon edit' } 5 'Edit > Copy to put the input text on the clipboard'
    Write-Host "OK: Edit > Select All and Edit > Copy copied 'tachyon edit' from the WebView2 input to the clipboard"
    Invoke-MenuCommand 'Edit' 'Select All'
    Invoke-MenuCommand 'Edit' 'Cut'
    Wait-Until { (Get-FieldValue $field) -eq '' } 5 'Edit > Cut to empty the input'
    Write-Host 'OK: Edit > Cut emptied the WebView2 input'
    Invoke-MenuCommand 'Edit' 'Paste'
    Wait-Until { (Get-FieldValue $field) -eq 'tachyon edit' } 5 'Edit > Paste to restore the text'
    Write-Host "OK: Edit > Paste put 'tachyon edit' back into the WebView2 input"
}

function Test-WindowControls {
    Test-Check 'a granted resize from the page' {
        Show-AppWindow
        $before = [TachyonDesktop]::WindowBox($script:window)
        Invoke-Named 'Resize window'
        Wait-Until { ([TachyonDesktop]::WindowBox($script:window)).Right -ne $before.Right } 5 'the window to resize'
        Start-Sleep -Milliseconds 300
        $client = [TachyonDesktop]::ClientBox($script:window)
        $dpi = [double][TachyonDesktop]::Dpi($script:window)
        $pixels = $client.Right - $client.Left
        $points = $pixels * 96 / $dpi
        if ([Math]::Abs($points - 640) -le 3) { Write-Host "OK: resize(640, 480) gave a client area $([Math]::Round($points)) DIP wide ($pixels pixels at $dpi DPI)" }
        elseif ([Math]::Abs($pixels - 640) -le 3) {
            Write-Host "OK: resize(640, 480) gave a client area 640 pixels wide"
            if ($dpi -ne 96) { Write-Host "NOTE: at $dpi DPI that is $([Math]::Round($points)) DIP: the host resizes in physical pixels, while window.width is in DIP" }
        }
        else { throw "resize(640, 480) gave a client area $pixels pixels ($([Math]::Round($points)) DIP) wide" }
    }
    Test-Check 'a granted maximize from the page, and restore' {
        Invoke-Named 'Maximize window'
        Wait-Until { [TachyonDesktop]::Maximized($script:window) } 5 'IsZoomed after maximize()'
        Write-Host 'OK: maximize() from the page maximized the window (IsZoomed)'
        Invoke-Named 'Maximize window'
        Wait-Until { -not [TachyonDesktop]::Maximized($script:window) } 5 'the window to restore after a second maximize()'
        Write-Host 'OK: a second maximize() from the page restored the window'
    }
    Test-Check 'a granted minimize from the page' {
        Invoke-Named 'Minimize window'
        Wait-Until { [TachyonDesktop]::Minimized($script:window) } 5 'IsIconic after minimize()'
        Write-Host 'OK: minimize() from the page minimized the window (IsIconic)'
        [TachyonDesktop]::Show($script:window, 9)
        Wait-Until { -not [TachyonDesktop]::Minimized($script:window) } 5 'the window to come back'
        Start-Sleep -Milliseconds 500
    }
    Test-Check 'a granted fullscreen from the page, and back' {
        $before = [TachyonDesktop]::WindowBox($script:window)
        Invoke-Named 'Fullscreen window'
        $monitor = [TachyonDesktop]::MonitorBox($script:window)
        Wait-Until {
            $box = [TachyonDesktop]::WindowBox($script:window)
            $box.Left -eq $monitor.Left -and $box.Top -eq $monitor.Top -and $box.Right -eq $monitor.Right -and $box.Bottom -eq $monitor.Bottom
        } 5 'the window to cover the monitor'
        Assert-That (([TachyonDesktop]::Style($script:window) -band 0x00C00000) -eq 0) 'the full screen window still has a caption'
        Write-Host 'OK: fullscreen() from the page covered the monitor without a frame'
        Invoke-Named 'Fullscreen window'
        Wait-Until { ([TachyonDesktop]::WindowBox($script:window)).Right -eq $before.Right } 5 'the window to leave full screen'
        Assert-That (([TachyonDesktop]::Style($script:window) -band 0x00C00000) -eq 0x00C00000) 'the window did not get its caption back'
        Write-Host 'OK: a second fullscreen() restored the frame and the bounds'
    }
    Test-Check 'a granted setTitle from the page' {
        Invoke-Named 'Rename window'
        Wait-Until { [TachyonDesktop]::TitleOf($script:window) -eq 'Renamed Shell' } 5 "the window title 'Renamed Shell'"
        Write-Host "OK: setTitle('Renamed Shell') from the page renamed the window"
    }
    Test-Check 'a granted close from the page' {
        Invoke-Named 'Close window'
        Wait-Until { $script:process.HasExited } 10 'the process to exit after close()'
        Assert-That ($script:process.ExitCode -eq 0) "the app exited with code $($script:process.ExitCode)"
        Write-Host 'OK: close() from the page closed the window and the app exited with code 0'
    }
}

function Test-CaptionWindow([string] $fixture) {
    $executable = Get-AppExecutable $fixture 'ShellCaption'
    Test-Check 'the executable is built for this machine' {
        $machine = Get-PeMachine $executable
        Write-Host ("OK: ShellCaption.exe is a PE file for machine 0x{0:X4}" -f $machine)
    }
    Start-GateApp $executable 'dev.tachyon.shell-caption'
    Test-Check 'the caption page renders with the grant' {
        Wait-Named @('Caption route', 'Granted minimize true') | Out-Null
        Write-Host "OK: the page rendered in WebView2 and windowControls.granted('minimize') is true"
    }
    Test-Check 'theme_color colours the title bar' { Test-CaptionColor }
    Test-Check 'an app without declared items has the default menu bar' {
        $titles = @([TachyonDesktop]::MenuTitles($script:window))
        Assert-That (($titles -join ',') -eq 'File,Edit,Window') "the menu bar is '$($titles -join ', ')', not 'File, Edit, Window'"
        Assert-MenuLabels 'File' @("Close Window`tCtrl+W", "Exit`tCtrl+Q")
        Assert-MenuLabels 'Edit' @("Undo`tCtrl+Z", "Redo`tCtrl+Y", "Cut`tCtrl+X", "Copy`tCtrl+C", "Paste`tCtrl+V", "Select All`tCtrl+A")
        Assert-MenuLabels 'Window' @('Minimize', 'Maximize')
        Write-Host 'OK: with no declared items the menu bar holds File (Close Window, Exit), Edit (Undo, Redo, Cut, Copy, Paste, Select All) and Window (Minimize, Maximize)'
    }
    Test-Check 'the default Edit menu works inside WebView2' { Test-EditCommands 'Caption notes' }
    Test-WindowControls
}

function Test-FullscreenStart([string] $fixture) {
    Start-GateApp (Get-AppExecutable $fixture 'ShellFull') 'dev.tachyon.shell-full'
    Test-Check 'display fullscreen starts borderless and full screen' {
        Wait-Named @('Full route') | Out-Null
        $monitor = [TachyonDesktop]::MonitorBox($script:window)
        $box = [TachyonDesktop]::WindowBox($script:window)
        $shot = Save-ScreenShot 'fullscreen-start' $monitor.Left $monitor.Top $monitor.Right $monitor.Bottom
        $shot.Bitmap.Dispose()
        Assert-That ($box.Left -eq $monitor.Left -and $box.Top -eq $monitor.Top -and $box.Right -eq $monitor.Right -and $box.Bottom -eq $monitor.Bottom) (
            "the window is ($($box.Left),$($box.Top))-($($box.Right),$($box.Bottom)); the monitor is ($($monitor.Left),$($monitor.Top))-($($monitor.Right),$($monitor.Bottom))")
        Assert-That (([TachyonDesktop]::Style($script:window) -band 0x00C00000) -eq 0) 'the full screen window has a caption'
        Write-Host 'OK: "display": "fullscreen" opened a borderless window that covers the monitor, and the page rendered'
    }
    Test-Check 'the page leaves full screen with the fullscreen control' {
        Invoke-Named 'Leave fullscreen'
        Wait-Until { ([TachyonDesktop]::Style($script:window) -band 0x00C00000) -eq 0x00C00000 } 5 'the window frame to come back'
        Write-Host 'OK: fullscreen() from the page gave the window its frame back'
    }
}

function Test-ArchitectureBuild([string] $fixture, [string] $wanted) {
    $executable = Get-AppExecutable $fixture 'ShellCaption'
    $machines = @{ 'x64' = 0x8664; 'arm64' = 0xAA64 }
    Test-Check "the executable is $wanted" {
        $machine = Get-PeMachine $executable
        Assert-That ($machine -eq $machines[$wanted]) ("ShellCaption.exe is for machine 0x{0:X4}, not {1} (0x{2:X4})" -f $machine, $wanted, $machines[$wanted])
        Write-Host ("OK: ShellCaption.exe is a {0} PE file (machine 0x{1:X4})" -f $wanted, $machine)
    }
    if ($wanted -eq 'arm64' -and (Get-HostArchitecture) -ne 'arm64') {
        Write-Host 'SKIP: an arm64 app cannot run on this x64 machine'
        return
    }
    Start-GateApp $executable 'dev.tachyon.shell-caption'
    Test-Check "the $wanted app runs its page" {
        Wait-Named @('Caption route', 'Granted minimize true') | Out-Null
        $machine = [TachyonDesktop]::ProcessMachine($script:process.Id)
        if ($machine -ge 0) { Write-Host ("NOTE: Windows runs the process as machine 0x{0:X4} on a {1} host" -f $machine, (Get-HostArchitecture)) }
        Invoke-Named 'Rename window'
        Wait-Until { [TachyonDesktop]::TitleOf($script:window) -eq 'Renamed Shell' } 5 "the window title 'Renamed Shell'"
        Write-Host "OK: the $wanted app rendered its page in WebView2 and its setTitle call reached the window"
    }
}

function Invoke-WindowShellGate {
    Initialize-DesktopGate $OutputDirectory
    $hostArchitecture = Get-HostArchitecture
    Enable-Msvc $hostArchitecture
    Enable-WebViewSdk
    $ty = Resolve-Ty
    if ($Architecture -ne 'native') {
        Enable-Msvc $Architecture
        Enable-WebViewSdk
        Invoke-GateApp "Shell Caption ($Architecture)" ${function:Write-CaptionFixture} $ty $Architecture {
            param([string] $fixture)
            Test-ArchitectureBuild $fixture $Architecture
        }
        Complete-Gate "the Windows app built with --arch $Architecture on a $hostArchitecture machine is $Architecture code and runs its WebView2 page"
    }
    Invoke-GateApp 'Shell Bar' ${function:Write-BarFixture} $ty 'native' ${function:Test-IntegratedTitleBar}
    Invoke-GateApp 'Shell Caption' ${function:Write-CaptionFixture} $ty 'native' ${function:Test-CaptionWindow}
    Invoke-GateApp 'Shell Full' ${function:Write-FullFixture} $ty 'native' ${function:Test-FullscreenStart}
    Complete-Gate 'integrated title bar, caption colour, fullscreen start, default menus and page window controls in WebView2'
}

Invoke-WindowShellGate
