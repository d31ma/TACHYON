param([string] $OutputDirectory = (Join-Path $env:TEMP 'tachyon-sd1-shots'))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot 'desktop-gate-common.ps1')

function Write-TrayIcon([string] $path) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    $bitmap = [System.Drawing.Bitmap]::new(32, 32)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.Clear([System.Drawing.Color]::FromArgb(255, 21, 101, 192))
        $graphics.FillEllipse([System.Drawing.Brushes]::White, 8, 8, 16, 16)
    }
    finally { $graphics.Dispose() }
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}

function Write-MenusFixture([string] $root) {
    Write-FixtureFile $root 'client/manifest.json' @'
{
  "name": "Shell Menus",
  "start_url": "/",
  "tac": {
    "application": {
      "id": "dev.tachyon.shell-menus",
      "version": "0.0.1",
      "window": { "width": 760, "height": 560 },
      "menus": {
        "file": [
          { "id": "open-about", "label": "About Shell", "route": "/about", "shortcut": { "key": "k", "modifiers": ["command"] } },
          { "id": "open-home", "label": "Shell Home", "route": "/" }
        ],
        "view": [
          {
            "id": "show-running",
            "label": "Show Running",
            "action": { "service": "ShellMenuService", "method": "show_running", "event": "SectionChosen" },
            "shortcut": { "key": "1", "modifiers": ["command"] }
          },
          {
            "id": "show-activity",
            "label": "Show Activity",
            "action": { "service": "ShellMenuService", "method": "show_activity", "event": "SectionChosen" },
            "shortcut": { "key": "2", "modifiers": ["command"] }
          }
        ],
        "help": [
          { "id": "open-help", "label": "Shell Help", "route": "/help" }
        ]
      },
      "tray": {
        "icon": "/shared/tray.png",
        "tooltip": "Shell Menus tray",
        "items": [
          { "id": "tray-help", "label": "Tray Help", "route": "/help" },
          { "id": "tray-section", "label": "Tray Section", "action": { "service": "ShellMenuService", "method": "show_tray", "event": "SectionChosen" } }
        ],
        "closeHidesWindow": true
      }
    },
    __CHECKS__
  }
}
'@
    Write-TrayIcon (Join-Path $root 'client/shared/tray.png')
    Write-FixtureFile $root 'client/models/SectionChosen.rs' @'
pub struct SectionChosen {
    pub section: String,
}
'@
    Write-FixtureFile $root 'client/services/menus/ShellMenuService.rs' @'
use crate::models::SectionChosen;

#[Service]
#[Lifetime({"value":"application"})]
struct ShellMenuService {
    page_events: CurrentPageEvents,
}

impl ShellMenuService {
    fn new(page_events: CurrentPageEvents) -> Self {
        Self { page_events }
    }

    fn show_running(&self) {
        self.page_events.publish_event(SectionChosen { section: String::from("running") });
    }

    fn show_activity(&self) {
        self.page_events.publish_event(SectionChosen { section: String::from("activity") });
    }

    fn show_tray(&self) {
        self.page_events.publish_event(SectionChosen { section: String::from("tray") });
    }
}
'@
    Write-FixtureFile $root 'client/services/SectionService.rs' @'
use crate::models::SectionChosen;

#[Service]
#[derive(Default)]
struct SectionService {
    section: String,
}

impl SectionService {
    #[OnEvent]
    fn chosen(&mut self, chosen: SectionChosen) {
        self.section = chosen.section;
    }
}
'@
    Write-FixtureFile $root 'client/services/HelpCountService.js' @'
@Service
export default class HelpCountService {
  count = 0

  add() {
    this.count += 1
  }
}
'@
    Write-FixtureFile $root 'client/pages/tac.html' "<shell-home hydrate=`"load`" />`n"
    Write-FixtureFile $root 'client/components/shell/home/tac.html' @'
<main aria-label="Shell menus">
  <h1>Shell home</h1>
  <input aria-label="Notes" />
  <p>Section {section.section}</p>
</main>
'@
    Write-FixtureFile $root 'client/pages/about/tac.html' "<shell-about />`n"
    Write-FixtureFile $root 'client/components/shell/about/tac.html' @'
<main aria-label="Shell about">
  <h1>About route</h1>
</main>
'@
    Write-FixtureFile $root 'client/pages/help/tac.html' "<shell-help hydrate=`"load`" />`n"
    Write-FixtureFile $root 'client/components/shell/help/tac.html' @'
<main aria-label="Shell help">
  <h1>Help route</h1>
  <p>Help count {helpCount.count}</p>
  <button on:click="helpCount.add()">Count help</button>
</main>
'@
}

function Open-Home {
    Invoke-MenuCommand 'File' 'Shell Home'
    Wait-Named @('Shell home', 'Notes') | Out-Null
}

function Invoke-SectionAction([string] $title, [string] $label, [string] $section) {
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        Invoke-MenuCommand $title $label
        try {
            Wait-Named @("Section $section") 4 | Out-Null
            return
        }
        catch { if ($attempt -eq 3) { throw "$title > $label did not show 'Section $section' on the page (the item stays disabled while no page service listens)" } }
    }
}

function Send-PageShortcut([uint16] $key, [string] $text) {
    Click-Element (Wait-Named @('Notes'))
    $focus = [TachyonDesktop]::FocusedWindow()
    Assert-That ([TachyonDesktop]::Foreground() -eq $script:window) 'the app window is not in the foreground after the click'
    [TachyonDesktop]::Keys(@([uint16]0x11, $key))
    Wait-Named @($text) 5 | Out-Null
    return [TachyonDesktop]::ClassOf($focus)
}

function Test-Menus {
    Test-Check 'the menu bar shows the declared items' {
        $titles = @([TachyonDesktop]::MenuTitles($script:window))
        Assert-That (($titles -join ',') -eq 'File,Edit,View,Window,Help') "the menu bar is '$($titles -join ', ')'"
        Assert-MenuLabels 'File' @("About Shell`tCtrl+K", 'Shell Home', "Close Window`tCtrl+W", "Exit`tCtrl+Q")
        Assert-MenuLabels 'View' @("Show Running`tCtrl+1", "Show Activity`tCtrl+2")
        Assert-MenuLabels 'Help' @('Shell Help')
        Show-AppWindow
        $shot = Save-WindowShot 'menus-window'
        $shot.Bitmap.Dispose()
        Write-Host 'OK: the menu bar holds File, Edit, View, Window and Help with the declared items and their Ctrl shortcuts'
    }
    Test-Check 'route items open their pages' {
        Invoke-MenuCommand 'File' 'About Shell'
        Wait-Named @('About route') | Out-Null
        Write-Host 'OK: File > About Shell opened /about in WebView2'
        Invoke-MenuCommand 'Help' 'Shell Help'
        Wait-Named @('Help route', 'Help count 0') | Out-Null
        Write-Host 'OK: Help > Shell Help opened /help in WebView2'
        Open-Home
        Write-Host 'OK: File > Shell Home opened / again'
    }
    Test-Check 'a menu action reaches the page as an event' {
        Invoke-SectionAction 'View' 'Show Running' 'running'
        Write-Host "OK: View > Show Running ran the application service, and its SectionChosen event reached the page service (Section running)"
    }
    Test-Check 'Ctrl+2 inside WebView2 runs a menu action' {
        $focus = Send-PageShortcut ([uint16]0x32) 'Section activity'
        Write-Host "OK: Ctrl+2, sent with SendInput while the focus was in WebView2 ($focus), ran View > Show Activity through AcceleratorKeyPressed"
    }
    Test-Check 'Ctrl+K inside WebView2 opens a route item' {
        $focus = Send-PageShortcut ([uint16]0x4B) 'About route'
        Write-Host "OK: Ctrl+K, sent with SendInput while the focus was in WebView2 ($focus), opened /about"
        Open-Home
    }
    Test-Check 'the Edit menu works inside WebView2' {
        Click-Element (Wait-Named @('Notes'))
        Set-FieldValue 'Notes' 'tachyon edit'
        Wait-Until { (Get-FieldValue 'Notes') -eq 'tachyon edit' } 5 "the Notes input to hold 'tachyon edit'"
        Click-Element (Wait-Named @('Notes'))
        try { Set-Clipboard -Value 'sentinel' } catch { Write-Host "NOTE: could not seed the clipboard: $($_.Exception.Message)" }
        Invoke-MenuCommand 'Edit' 'Select All'
        Invoke-MenuCommand 'Edit' 'Copy'
        Wait-Until { (Get-ClipboardText) -eq 'tachyon edit' } 5 'Edit > Copy to put the input text on the clipboard'
        Write-Host "OK: Edit > Select All and Edit > Copy copied 'tachyon edit' from the WebView2 input"
        Invoke-MenuCommand 'Edit' 'Select All'
        Invoke-MenuCommand 'Edit' 'Cut'
        Wait-Until { (Get-FieldValue 'Notes') -eq '' } 5 'Edit > Cut to empty the input'
        Write-Host 'OK: Edit > Cut emptied the WebView2 input'
        Invoke-MenuCommand 'Edit' 'Paste'
        Wait-Until { (Get-FieldValue 'Notes') -eq 'tachyon edit' } 5 'Edit > Paste to restore the text'
        Write-Host "OK: Edit > Paste put 'tachyon edit' back into the WebView2 input"
    }
    Test-Check 'the Window menu minimizes and maximizes' {
        Invoke-MenuCommand 'Window' 'Minimize'
        Wait-Until { [TachyonDesktop]::Minimized($script:window) } 5 'IsIconic after Window > Minimize'
        Write-Host 'OK: Window > Minimize minimized the window'
        [TachyonDesktop]::Show($script:window, 9)
        Wait-Until { -not [TachyonDesktop]::Minimized($script:window) } 5 'the window to come back'
        Invoke-MenuCommand 'Window' 'Maximize'
        Wait-Until { [TachyonDesktop]::Maximized($script:window) } 5 'IsZoomed after Window > Maximize'
        Write-Host 'OK: Window > Maximize maximized the window'
        Invoke-MenuCommand 'Window' 'Maximize'
        Wait-Until { -not [TachyonDesktop]::Maximized($script:window) } 5 'the window to restore after a second Window > Maximize'
        Write-Host 'OK: a second Window > Maximize restored the window'
    }
}

function Open-TrayMenu([string] $shot) {
    [TachyonDesktop]::Post($script:window, 0x8007, 0, (1 * 65536) + 0x007B) | Out-Null
    $popup = Wait-Popup 5
    if ($shot) {
        $box = [TachyonDesktop]::WindowBox($popup)
        $picture = Save-ScreenShot $shot $box.Left $box.Top $box.Right $box.Bottom
        $picture.Bitmap.Dispose()
    }
    return $popup
}

function Close-AppWindowByButton {
    Show-AppWindow
    $buttons = [TachyonDesktop]::CaptionButtons($script:window)
    $width = [int](($buttons.Right - $buttons.Left) / 3)
    [TachyonDesktop]::Click($buttons.Right - [int]($width / 2), [int](($buttons.Top + $buttons.Bottom) / 2))
    Wait-Until { -not [TachyonDesktop]::Visible($script:window) } 5 'the window to hide after a click on its close button'
    Start-Sleep -Milliseconds 800
    Assert-Running
}

function Write-TaskbarNote {
    try {
        foreach ($explorer in @(Get-Process -Name explorer -ErrorAction SilentlyContinue)) {
            foreach ($taskbar in @([TachyonDesktop]::Windows($explorer.Id, 'Shell_TrayWnd', $true))) {
                $named = [System.Windows.Automation.AutomationElement]::FromHandle($taskbar).FindFirst(
                    [System.Windows.Automation.TreeScope]::Descendants,
                    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, 'Shell Menus tray'))
                if ($null -ne $named) {
                    Write-Host "NOTE: UI Automation finds the tray button 'Shell Menus tray' on the taskbar"
                    return
                }
            }
        }
        Write-Host 'NOTE: UI Automation finds no tray button on the taskbar itself; Windows 11 may keep new icons in the hidden-icons flyout'
    }
    catch { Write-Host "NOTE: the taskbar could not be read through UI Automation: $($_.Exception.Message)" }
}

function Test-Tray {
    Test-Check 'the notification-area icon exists' {
        $box = New-Object 'TachyonDesktop+Box'
        $result = [TachyonDesktop]::TrayIconBox($script:window, 1, [ref]$box)
        Assert-That ($result -eq 0) ("Shell_NotifyIconGetRect answered 0x{0:X8}: the notification area has no icon for the window" -f $result)
        Write-Host "OK: the notification area holds the app's icon at ($($box.Left),$($box.Top))-($($box.Right),$($box.Bottom))"
        Write-TaskbarNote
    }
    Test-Check 'the tray menu holds Open, the items and Exit, and an action item runs' {
        $popup = Open-TrayMenu 'tray-menu'
        $names = @(Get-PopupNames $popup)
        $missing = @(@('Open Shell Menus', 'Tray Help', 'Tray Section', 'Exit Shell Menus') | Where-Object { $names -notcontains $_ })
        if ($missing.Count -gt 0) {
            Close-Popup
            throw "the tray menu lacks $($missing -join ', '); it holds: $($names -join ', ')"
        }
        Write-Host "OK: the tray menu holds $($names -join ', ')"
        Invoke-PopupItem $popup 'Tray Section'
        Wait-Named @('Section tray') 5 | Out-Null
        Write-Host 'OK: the tray item Tray Section ran the application service, and its event reached the page (Section tray)'
    }
    Test-Check 'a tray route item opens its page' {
        $popup = Open-TrayMenu ''
        Invoke-PopupItem $popup 'Tray Help'
        Wait-Named @('Help route', 'Help count 0') | Out-Null
        Invoke-Named 'Count help'
        Wait-Named @('Help count 1') 5 | Out-Null
        Write-Host 'OK: the tray item Tray Help opened /help, and its page counts clicks (Help count 1)'
    }
    Test-Check 'closing the window hides it' {
        Close-AppWindowByButton
        Write-Host 'OK: a click on the close button hid the window (IsWindowVisible false) and the process kept running'
    }
    Test-Check 'a click on the tray icon shows the same page again' {
        [TachyonDesktop]::Post($script:window, 0x8007, 0, (1 * 65536) + 0x0400) | Out-Null
        Wait-Until { [TachyonDesktop]::Visible($script:window) } 5 'the window to show after NIN_SELECT'
        Wait-Named @('Help count 1') 5 | Out-Null
        Write-Host 'OK: a tray icon click (NIN_SELECT) showed the window, and the page kept its state (Help count 1)'
    }
    Test-Check 'the tray Open item shows the window' {
        Close-AppWindowByButton
        $popup = Open-TrayMenu ''
        Invoke-PopupItem $popup 'Open Shell Menus'
        Wait-Until { [TachyonDesktop]::Visible($script:window) } 5 'the window to show after Open Shell Menus'
        Write-Host 'OK: the tray item Open Shell Menus showed the hidden window'
    }
    Test-Check 'the tray Exit item ends the app' {
        $window = $script:window
        $popup = Open-TrayMenu ''
        Invoke-PopupItem $popup 'Exit Shell Menus'
        Wait-Until { $script:process.HasExited } 10 'the process to exit after Exit Shell Menus'
        Assert-That ($script:process.ExitCode -eq 0) "the app exited with code $($script:process.ExitCode)"
        Write-Host 'OK: the tray item Exit Shell Menus ended the process with exit code 0'
        $box = New-Object 'TachyonDesktop+Box'
        if ([TachyonDesktop]::TrayIconBox($window, 1, [ref]$box) -ne 0) { Write-Host 'OK: the notification-area icon is gone' }
        else { Write-Host 'NOTE: Shell_NotifyIconGetRect still finds the icon after Exit' }
    }
}

function Test-MenusAndTray([string] $fixture) {
    Start-GateApp (Get-AppExecutable $fixture 'ShellMenus') 'dev.tachyon.shell-menus'
    Test-Check 'the menus page renders' {
        Wait-Named @('Shell home', 'Notes') | Out-Null
        Write-Host 'OK: the page rendered in WebView2'
    }
    Test-Menus
    Test-Check 'the page is back at home for the tray checks' {
        if ($null -eq (Find-Named 'Shell home')) { Open-Home }
        Write-Host 'OK: the home page with its SectionChosen listener is shown'
    }
    Test-Tray
}

function Invoke-MenuTrayGate {
    Initialize-DesktopGate $OutputDirectory
    Enable-Msvc (Get-HostArchitecture)
    Enable-WebViewSdk
    $ty = Resolve-Ty
    Invoke-GateApp 'Shell Menus' ${function:Write-MenusFixture} $ty 'native' ${function:Test-MenusAndTray}
    Complete-Gate 'menu route items, menu actions with page events, AcceleratorKeyPressed shortcuts, Edit and Window menus, and the tray icon, its items and close-to-hide in WebView2'
}

Invoke-MenuTrayGate
