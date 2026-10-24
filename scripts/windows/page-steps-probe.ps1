param(
    [int] $ProcessId = 0,
    [string] $Executable = '',
    [string] $Steps = '',
    [string] $StepsFile = '',
    [int] $WaitSeconds = 30,
    [int] $LaunchSeconds = 90,
    [int] $RequestSeconds = 120
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:process = $null
$script:window = $null
$script:handle = [IntPtr]::Zero
$script:caretReadable = $true
$script:caretModel = 0
$script:otherWindow = $null
$script:started = [DateTime]::UtcNow

function Write-Line([string] $text) {
    [Console]::Out.WriteLine($text)
    [Console]::Out.Flush()
}

function Add-ProbeTypes {
    Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
    Add-Type -Language CSharp -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class TachyonProbeNative
{
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left; public int Top; public int Right; public int Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    struct MouseInput { public int X; public int Y; public uint Data; public uint Flags; public uint Time; public IntPtr Extra; }

    [StructLayout(LayoutKind.Sequential)]
    struct KeyboardInput { public ushort Key; public ushort Scan; public uint Flags; public uint Time; public IntPtr Extra; }

    [StructLayout(LayoutKind.Explicit)]
    struct InputUnion { [FieldOffset(0)] public MouseInput Mouse; [FieldOffset(0)] public KeyboardInput Keyboard; }

    [StructLayout(LayoutKind.Sequential)]
    struct Input { public uint Type; public InputUnion Data; }

    [StructLayout(LayoutKind.Sequential)]
    struct GuiThreadInfo
    {
        public int Size; public uint Flags; public IntPtr Active; public IntPtr Focus; public IntPtr Capture;
        public IntPtr MenuOwner; public IntPtr MoveSize; public IntPtr Caret; public Rect CaretRect;
    }

    const uint MouseEvent = 0;
    const uint KeyboardEvent = 1;
    const uint KeyExtended = 0x1;
    const uint KeyUp = 0x2;
    const uint KeyUnicode = 0x4;
    const uint MouseLeftDown = 0x2;
    const uint MouseLeftUp = 0x4;
    const byte AltKey = 0x12;
    const uint MouseExtraDown = 0x80;
    const uint MouseExtraUp = 0x100;
    const int Restore = 9;
    const int Minimize = 6;
    const uint CloseMessage = 0x0010;

    [DllImport("user32.dll", SetLastError = true)] static extern uint SendInput(uint count, Input[] inputs, int size);
    [DllImport("user32.dll")] static extern uint MapVirtualKeyW(uint code, uint mapType);
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr window);
    [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll")] static extern bool PostMessageW(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, IntPtr process);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint attach, uint to, bool on);
    [DllImport("user32.dll")] static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] static extern bool GetGUIThreadInfo(uint thread, ref GuiThreadInfo info);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassNameW(IntPtr window, StringBuilder name, int size);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] static extern bool AdjustWindowRectExForDpi(ref Rect rect, uint style, bool menu, uint extendedStyle, uint dpi);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongW")] static extern int GetWindowLong(IntPtr window, int index);
    [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);

    public static void UsePhysicalPixels()
    {
        try { SetThreadDpiAwarenessContext(new IntPtr(-4)); }
        catch (EntryPointNotFoundException) { }
    }

    static Input Key(ushort key, ushort scan, uint flags)
    {
        Input input = new Input();
        input.Type = KeyboardEvent;
        input.Data.Keyboard.Key = key;
        input.Data.Keyboard.Scan = scan;
        input.Data.Keyboard.Flags = flags;
        return input;
    }

    static Input Mouse(uint flags)
    {
        Input input = new Input();
        input.Type = MouseEvent;
        input.Data.Mouse.Flags = flags;
        return input;
    }

    static void Send(Input[] inputs)
    {
        uint sent = SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(Input)));
        if (sent != inputs.Length)
        {
            throw new InvalidOperationException("SendInput delivered " + sent + " of " + inputs.Length + " events (Win32 error " + Marshal.GetLastWin32Error() + ")");
        }
    }

    public static void TypeText(string text)
    {
        Input[] inputs = new Input[text.Length * 2];
        for (int index = 0; index < text.Length; index++)
        {
            inputs[index * 2] = Key(0, text[index], KeyUnicode);
            inputs[index * 2 + 1] = Key(0, text[index], KeyUnicode | KeyUp);
        }
        Send(inputs);
    }

    public static void PressKey(ushort key, bool extended)
    {
        ushort scan = (ushort)MapVirtualKeyW(key, 0);
        uint flags = extended ? KeyExtended : 0;
        Send(new Input[] { Key(key, scan, flags), Key(key, scan, flags | KeyUp) });
    }

    public static void PressWithAlt(ushort key, bool extended)
    {
        ushort scan = (ushort)MapVirtualKeyW(key, 0);
        ushort altScan = (ushort)MapVirtualKeyW(AltKey, 0);
        uint flags = extended ? KeyExtended : 0;
        Send(new Input[] { Key(AltKey, altScan, 0), Key(key, scan, flags), Key(key, scan, flags | KeyUp), Key(AltKey, altScan, KeyUp) });
    }

    public static void ExtraButton(int x, int y, uint button)
    {
        UsePhysicalPixels();
        if (!SetCursorPos(x, y)) throw new InvalidOperationException("SetCursorPos refused " + x + "," + y);
        Input down = Mouse(MouseExtraDown);
        down.Data.Mouse.Data = button;
        Input up = Mouse(MouseExtraUp);
        up.Data.Mouse.Data = button;
        Send(new Input[] { down, up });
    }

    public static void Click(int x, int y)
    {
        UsePhysicalPixels();
        if (!SetCursorPos(x, y)) throw new InvalidOperationException("SetCursorPos refused " + x + "," + y);
        Send(new Input[] { Mouse(MouseLeftDown), Mouse(MouseLeftUp) });
    }

    public static IntPtr Foreground()
    {
        return GetForegroundWindow();
    }

    public static bool BringToFront(IntPtr window)
    {
        if (GetForegroundWindow() == window) return true;
        if (IsIconic(window)) ShowWindow(window, Restore);
        keybd_event(AltKey, 0, 0, UIntPtr.Zero);
        SetForegroundWindow(window);
        keybd_event(AltKey, 0, KeyUp, UIntPtr.Zero);
        if (GetForegroundWindow() == window) return true;
        uint own = GetCurrentThreadId();
        uint other = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        bool attached = other != 0 && other != own && AttachThreadInput(own, other, true);
        BringWindowToTop(window);
        SetForegroundWindow(window);
        if (attached) AttachThreadInput(own, other, false);
        return GetForegroundWindow() == window;
    }

    public static bool IsMinimized(IntPtr window)
    {
        return IsIconic(window);
    }

    public static void MinimizeWindow(IntPtr window)
    {
        ShowWindow(window, Minimize);
    }

    public static bool CloseWindow(IntPtr window)
    {
        return PostMessageW(window, CloseMessage, IntPtr.Zero, IntPtr.Zero);
    }

    public static string ClassName(IntPtr window)
    {
        if (window == IntPtr.Zero) return "";
        StringBuilder name = new StringBuilder(256);
        GetClassNameW(window, name, name.Capacity);
        return name.ToString();
    }

    public static IntPtr KeyboardFocus(IntPtr window)
    {
        GuiThreadInfo info = new GuiThreadInfo();
        info.Size = Marshal.SizeOf(typeof(GuiThreadInfo));
        uint thread = GetWindowThreadProcessId(window, IntPtr.Zero);
        if (thread == 0 || !GetGUIThreadInfo(thread, ref info)) return IntPtr.Zero;
        return info.Focus;
    }

    public static int[] Size(IntPtr window, int width, int height)
    {
        UsePhysicalPixels();
        Rect outer;
        Rect client;
        GetWindowRect(window, out outer);
        GetClientRect(window, out client);
        uint dpi = GetDpiForWindow(window);
        Rect expected = new Rect();
        expected.Right = width;
        expected.Bottom = height;
        AdjustWindowRectExForDpi(ref expected, (uint)GetWindowLong(window, -16), false, 0, dpi);
        return new int[] {
            outer.Right - outer.Left, outer.Bottom - outer.Top,
            client.Right - client.Left, client.Bottom - client.Top,
            expected.Right - expected.Left, expected.Bottom - expected.Top, (int)dpi
        };
    }
}
'@
    try {
        Add-Type -Language CSharp -ReferencedAssemblies Accessibility -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using Accessibility;

public static class TachyonProbeAccessible
{
    [StructLayout(LayoutKind.Sequential)]
    public struct Point { public int X; public int Y; }

    [DllImport("oleacc.dll")]
    static extern int AccessibleObjectFromPoint(Point point, [MarshalAs(UnmanagedType.Interface)] out IAccessible accessible, out object child);

    public static string DefaultAction(int x, int y)
    {
        Point point = new Point();
        point.X = x;
        point.Y = y;
        IAccessible accessible;
        object child;
        if (AccessibleObjectFromPoint(point, out accessible, out child) != 0 || accessible == null) return null;
        string name = accessible.get_accName(child);
        accessible.accDoDefaultAction(child);
        return name == null ? "" : name;
    }
}
'@
    }
    catch {
        Write-Line "NOTE: the MSAA default action fallback is unavailable: $($_.Exception.Message)"
    }
    [TachyonProbeNative]::UsePhysicalPixels()
}

function Read-Steps {
    $text = $Steps
    if ($StepsFile) { $text = [IO.File]::ReadAllText($StepsFile) }
    if (-not $text) { throw 'Pass the steps as JSON with -Steps or -StepsFile' }
    $parsed = ConvertFrom-Json ('{"steps":' + $text + '}')
    return , @($parsed.steps)
}

function Wait-Briefly([int] $milliseconds) {
    if ($null -ne $script:otherWindow) { [System.Windows.Forms.Application]::DoEvents() }
    Start-Sleep -Milliseconds $milliseconds
}

function Open-AppProcess([int] $id) {
    $found = Get-Process -Id $id
    $null = $found.Handle
    return $found
}

function Find-AppProcess {
    if ($ProcessId -gt 0) { return Open-AppProcess $ProcessId }
    if (-not $Executable) { throw 'Pass the app with -ProcessId or -Executable' }
    $path = [IO.Path]::GetFullPath($Executable)
    $deadline = [DateTime]::UtcNow.AddSeconds($LaunchSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        $found = @(Get-CimInstance Win32_Process -OperationTimeoutSec 5 |
            Where-Object { $_.ExecutablePath -and ($_.ExecutablePath -ieq $path) })
        if ($found.Count -gt 0) { return Open-AppProcess $found[0].ProcessId }
        Start-Sleep -Milliseconds 500
    }
    throw "No process runs $path after $LaunchSeconds seconds"
}

function Assert-AppRunning {
    if ($script:process.HasExited) { throw "The app exited with code $($script:process.ExitCode)" }
}

function Find-AppWindow {
    if ($null -ne $script:window) {
        try {
            $null = $script:window.Current.ProcessId
            return $script:window
        }
        catch { $script:window = $null }
    }
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $owned = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $script:process.Id)
    $hostClass = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ClassNameProperty, 'TachyonNativeHost')
    $found = $root.FindFirst([System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.AndCondition]::new($owned, $hostClass))
    if ($null -eq $found) { $found = $root.FindFirst([System.Windows.Automation.TreeScope]::Children, $owned) }
    if ($null -ne $found) {
        $script:window = $found
        $script:handle = [IntPtr]$found.Current.NativeWindowHandle
    }
    return $found
}

function Wait-AppWindow {
    $deadline = [DateTime]::UtcNow.AddSeconds($LaunchSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        Assert-AppRunning
        if ($null -ne (Find-AppWindow)) { return }
        Wait-Briefly 200
    }
    throw "The app showed no window in $LaunchSeconds seconds"
}

function ConvertTo-Shown([string] $text) {
    if ($null -eq $text) { return '' }
    return ($text -replace '\s+', ' ').Trim()
}

function Get-Elements {
    $window = Find-AppWindow
    $list = New-Object 'System.Collections.Generic.List[object]'
    if ($null -eq $window) { return , $list }
    $request = [System.Windows.Automation.CacheRequest]::new()
    $request.Add([System.Windows.Automation.AutomationElement]::NameProperty)
    $request.Add([System.Windows.Automation.AutomationElement]::ControlTypeProperty)
    $request.Add([System.Windows.Automation.AutomationElement]::LocalizedControlTypeProperty)
    $scope = $request.Activate()
    try {
        $found = $window.FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
    }
    catch {
        $script:window = $null
        return , $list
    }
    finally { $scope.Dispose() }
    foreach ($element in $found) {
        $list.Add([pscustomobject]@{
            Element = $element
            Name = (ConvertTo-Shown $element.Cached.Name)
            Type = ($element.Cached.ControlType.ProgrammaticName -replace '^ControlType\.', '')
            Localized = [string]$element.Cached.LocalizedControlType
        })
    }
    return , $list
}

function Get-ShownNames($elements) {
    $names = New-Object 'System.Collections.Generic.HashSet[string]'
    foreach ($item in $elements) { if ($item.Name) { [void]$names.Add($item.Name) } }
    return , $names
}

function Format-Shown($names) {
    $sorted = @($names | Sort-Object | Select-Object -First 80)
    return "'" + ($sorted -join "', '") + "'"
}

function Wait-Texts([string[]] $texts, [int] $seconds) {
    $wanted = @($texts | ForEach-Object { ConvertTo-Shown $_ })
    $deadline = [DateTime]::UtcNow.AddSeconds($seconds)
    $names = New-Object 'System.Collections.Generic.HashSet[string]'
    do {
        Assert-AppRunning
        $names = Get-ShownNames (Get-Elements)
        $missing = @($wanted | Where-Object { -not $names.Contains($_) })
        if ($missing.Count -eq 0) {
            Write-Line ('OK: ' + ($texts -join ', '))
            return
        }
        Wait-Briefly 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Missing '$($missing -join "', '")'; the window shows $(Format-Shown $names)"
}

function Wait-Gone([string[]] $texts) {
    $wanted = @($texts | ForEach-Object { ConvertTo-Shown $_ })
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    $shown = @()
    do {
        Assert-AppRunning
        $names = Get-ShownNames (Get-Elements)
        $shown = @($wanted | Where-Object { $names.Contains($_) })
        if ($shown.Count -eq 0 -and $null -ne $script:window) {
            Write-Line ('OK gone: ' + ($texts -join ', '))
            return
        }
        Wait-Briefly 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "The page still shows '$($shown -join "', '")'"
}

function Read-Text($step) {
    $prefix = [string]$step.prefix
    $skipped = @()
    if ($step.PSObject.Properties.Name -contains 'not') { $skipped = @($step.not) }
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    $names = New-Object 'System.Collections.Generic.HashSet[string]'
    do {
        Assert-AppRunning
        $names = Get-ShownNames (Get-Elements)
        foreach ($name in $names) {
            if (-not $name.StartsWith($prefix.Trim())) { continue }
            $value = $name.Substring($prefix.Trim().Length).Trim()
            if ($skipped -contains $value) { continue }
            Write-Line "READ: $value"
            return
        }
        Wait-Briefly 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "No text starts with '$prefix' (other than $($skipped -join ', ')); the window shows $(Format-Shown $names)"
}

function Find-Named([string] $name, [string] $type) {
    $wanted = ConvertTo-Shown $name
    $named = New-Object 'System.Collections.Generic.List[object]'
    foreach ($item in (Get-Elements)) {
        if ($item.Name -ne $wanted) { continue }
        if ($type -and $item.Type -ne $type) { continue }
        $named.Add($item.Element)
    }
    return , $named
}

function Get-Center($element) {
    $bounds = $element.Current.BoundingRectangle
    if ($bounds.IsEmpty -or $bounds.Width -le 0 -or $bounds.Height -le 0) { return $null }
    return , @([int]($bounds.X + $bounds.Width / 2), [int]($bounds.Y + $bounds.Height / 2))
}

function Invoke-Pattern($element) {
    $pattern = $null
    if ($element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        ([System.Windows.Automation.InvokePattern]$pattern).Invoke()
        return 'Invoke'
    }
    if ($element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$pattern)) {
        ([System.Windows.Automation.TogglePattern]$pattern).Toggle()
        return 'Toggle'
    }
    if ($element.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$pattern)) {
        ([System.Windows.Automation.SelectionItemPattern]$pattern).Select()
        return 'SelectionItem'
    }
    return $null
}

function Invoke-DefaultAction($element) {
    if (-not ('TachyonProbeAccessible' -as [type])) { return $null }
    $center = Get-Center $element
    if ($null -eq $center) { return $null }
    $name = [TachyonProbeAccessible]::DefaultAction($center[0], $center[1])
    if ($null -eq $name) { return $null }
    return "the MSAA default action of '$name'"
}

function Invoke-Control([string] $name) {
    Wait-Texts @($name) $WaitSeconds
    $candidates = Find-Named $name ''
    foreach ($element in $candidates) {
        $how = Invoke-Pattern $element
        if ($null -ne $how) {
            if ($how -ne 'Invoke') { Write-Line "NOTE: '$name' was activated through $how" }
            return
        }
    }
    foreach ($element in $candidates) {
        $how = Invoke-DefaultAction $element
        if ($null -ne $how) { Write-Line "NOTE: '$name' has no Invoke, Toggle or SelectionItem pattern; used $how"; return }
    }
    foreach ($element in $candidates) {
        $center = Get-Center $element
        if ($null -eq $center) { continue }
        Show-AppWindow
        [TachyonProbeNative]::Click($center[0], $center[1])
        Write-Line "NOTE: '$name' has no UI Automation activation; clicked it with the mouse"
        return
    }
    throw "Rendered control '$name' has no UI Automation activation and no point to click"
}

function Find-FileDialog {
    $owned = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $script:process.Id)
    $dialogClass = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ClassNameProperty, '#32770')
    $found = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
        [System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.AndCondition]::new($owned, $dialogClass))
    $window = Find-AppWindow
    if ($null -eq $found -and $null -ne $window) {
        $found = $window.FindFirst([System.Windows.Automation.TreeScope]::Children, $dialogClass)
    }
    return $found
}

function Wait-FileDialog([bool] $open) {
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    do {
        Assert-AppRunning
        $found = Find-FileDialog
        if ($open -and $null -ne $found) { return $found }
        if (-not $open -and $null -eq $found) { return $null }
        Wait-Briefly 200
    } while ([DateTime]::UtcNow -lt $deadline)
    if ($open) { throw "No file dialog opened in $WaitSeconds seconds" }
    throw "The file dialog was still open after $WaitSeconds seconds"
}

function Set-FileName($dialog, [string] $file) {
    $nameBox = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::AutomationIdProperty, '1148'),
        [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Edit))
    $entry = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $nameBox)
    if ($null -eq $entry) { throw 'The file dialog has no file name box' }
    $pattern = $null
    if (-not $entry.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)) {
        throw 'The file name box takes no value'
    }
    $entry.SetFocus()
    ([System.Windows.Automation.ValuePattern]$pattern).SetValue($file)
}

function Invoke-Pick($step) {
    $name = [string]$step.control
    Wait-Texts @($name) $WaitSeconds
    $center = $null
    foreach ($element in (Find-Named $name '')) {
        $center = Get-Center $element
        if ($null -ne $center) { break }
    }
    if ($null -eq $center) { throw "Rendered control '$name' has no point to click" }
    Show-AppWindow
    [TachyonProbeNative]::Click($center[0], $center[1])
    $dialog = Wait-FileDialog $true
    Wait-ForegroundWindow ([IntPtr]$dialog.Current.NativeWindowHandle) 'the file dialog'
    $key = 'Escape'
    if ($step.PSObject.Properties.Name -contains 'file') {
        Set-FileName $dialog ([string]$step.file)
        $key = 'Enter'
    }
    [TachyonProbeNative]::PressKey((Get-KeyCode $key), $false)
    Wait-FileDialog $false | Out-Null
    Write-Line "OK: the file dialog closed with $key"
}

function Show-AppWindow {
    Wait-ForegroundWindow $script:handle 'the app window'
}

function Wait-ForegroundWindow([IntPtr] $handle, [string] $label) {
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        if ([TachyonProbeNative]::BringToFront($handle)) { return }
        Wait-Briefly 200
    } while ([DateTime]::UtcNow -lt $deadline)
    $front = [TachyonProbeNative]::Foreground()
    throw "Windows kept $label behind '$([TachyonProbeNative]::ClassName($front))' (handle $front); is the desktop locked?"
}

function Find-Entry([string] $name) {
    $entries = Find-Named $name 'Edit'
    if ($entries.Count -gt 0) { return $entries[0] }
    $any = Find-Named $name ''
    if ($any.Count -gt 0) { return $any[0] }
    throw "No text entry named '$name'"
}

function Read-Caret($entry) {
    $pattern = $null
    if (-not $entry.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern, [ref]$pattern)) { return -1 }
    $text = [System.Windows.Automation.TextPattern]$pattern
    $selection = $text.GetSelection()
    if ($null -eq $selection -or $selection.Length -eq 0) { return -1 }
    $before = $text.DocumentRange.Clone()
    $before.MoveEndpointByRange([System.Windows.Automation.Text.TextPatternRangeEndpoint]::End,
        $selection[0], [System.Windows.Automation.Text.TextPatternRangeEndpoint]::Start)
    return $before.GetText(-1).Length
}

function Get-EntryState([string] $name) {
    $entry = Find-Entry $name
    $value = $null
    $text = ''
    if ($entry.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$value)) {
        $text = ([System.Windows.Automation.ValuePattern]$value).Current.Value
    }
    $caret = -1
    if ($script:caretReadable) {
        $caret = Read-Caret $entry
        if ($caret -lt 0) {
            $script:caretReadable = $false
            Write-Line 'NOTE: UI Automation gives no caret for this entry (no TextPattern selection); the probe checks the text and focus, and follows the caret itself'
        }
    }
    if (-not $script:caretReadable) { $caret = $script:caretModel }
    return [pscustomobject]@{ text = [string]$text; caret = [int]$caret; focused = [bool]$entry.Current.HasKeyboardFocus }
}

function Wait-Entry([string] $name, [hashtable] $expected, [string] $shown) {
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    $state = $null
    $names = New-Object 'System.Collections.Generic.HashSet[string]'
    do {
        Assert-AppRunning
        $state = Get-EntryState $name
        $names = Get-ShownNames (Get-Elements)
        $same = $true
        foreach ($key in $expected.Keys) { if ($state.$key -ne $expected[$key]) { $same = $false } }
        if ($same -and ((-not $shown) -or $names.Contains((ConvertTo-Shown $shown)))) {
            $caretSource = 'read through UI Automation'
            if (-not $script:caretReadable) { $caretSource = 'followed by the probe' }
            $page = $shown
            if (-not $page) { $page = 'the same text' }
            Write-Line "OK: $name holds '$($state.text)' with the caret at $($state.caret) ($caretSource) and focus; page shows '$page'"
            return
        }
        Wait-Briefly 100
    } while ([DateTime]::UtcNow -lt $deadline)
    $typed = @($names | Where-Object { $_.StartsWith('Typed') })
    $wanted = ($expected.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ', '
    throw "${name}: wanted $wanted and '$shown'; the entry has text='$($state.text)', caret=$($state.caret), focused=$($state.focused); the page shows $(Format-Shown $typed)"
}

function Test-WebFocus {
    $focus = [TachyonProbeNative]::KeyboardFocus($script:handle)
    return ([TachyonProbeNative]::ClassName($focus)).StartsWith('Chrome_')
}

function Wait-Focus([string] $name, [int] $seconds, [bool] $inWebView) {
    $deadline = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        $entry = Find-Entry $name
        if ($entry.Current.HasKeyboardFocus -and ((-not $inWebView) -or (Test-WebFocus))) { return $true }
        Wait-Briefly 100
    } while ([DateTime]::UtcNow -lt $deadline)
    return $false
}

function Set-EntryFocus([string] $name) {
    Wait-Texts @($name) $WaitSeconds
    Show-AppWindow
    $entry = Find-Entry $name
    $entry.SetFocus()
    if (-not (Wait-Focus $name 5 $true)) {
        $focus = [TachyonProbeNative]::KeyboardFocus($script:handle)
        $center = Get-Center (Find-Entry $name)
        if ($null -eq $center) { throw "$name did not take focus and has no point to click" }
        Write-Line "NOTE: after UI Automation SetFocus, $name has HasKeyboardFocus=$((Find-Entry $name).Current.HasKeyboardFocus) and Windows keyboard focus is on '$([TachyonProbeNative]::ClassName($focus))'; clicking the entry"
        Show-AppWindow
        [TachyonProbeNative]::Click($center[0], $center[1])
        if (-not (Wait-Focus $name $WaitSeconds $false)) { throw "$name did not take focus" }
    }
    $script:caretModel = (Get-EntryState $name).text.Length
    Write-Line "OK: $name has focus"
}

function Assert-TypingTarget {
    if ([TachyonProbeNative]::Foreground() -ne $script:handle) {
        Write-Line 'NOTE: the app window was not in front before typing; bringing it back'
        Show-AppWindow
    }
}

function Send-Text($step) {
    $name = [string]$step.entry
    $state = Get-EntryState $name
    $text = $state.text
    $caret = [Math]::Max(0, [Math]::Min($text.Length, $state.caret))
    Assert-TypingTarget
    foreach ($character in ([string]$step.keys).ToCharArray()) {
        [TachyonProbeNative]::TypeText([string]$character)
        $text = $text.Substring(0, $caret) + $character + $text.Substring($caret)
        $caret += 1
        $script:caretModel = $caret
        if ($step.pace -eq 'each') {
            Wait-Entry $name @{ text = $text; caret = $caret; focused = $true } ([string]$step.shown + $text)
        }
    }
    if ($step.pace -eq 'burst') {
        Wait-Entry $name @{ text = $text; caret = $caret; focused = $true } ([string]$step.shown + $text)
    }
}

function Get-KeyCode([string] $key) {
    $codes = @{ Left = 0x25; Right = 0x27; Up = 0x26; Down = 0x28; Home = 0x24; End = 0x23; Delete = 0x2E
        Backspace = 0x08; Tab = 0x09; Enter = 0x0D; Escape = 0x1B; F10 = 0x79 }
    if (-not $codes.ContainsKey($key)) { throw "Unknown key '$key'" }
    return [uint16]$codes[$key]
}

function Move-Caret($step) {
    $name = [string]$step.entry
    $length = (Get-EntryState $name).text.Length
    Assert-TypingTarget
    foreach ($key in @($step.keys)) {
        $extended = @('Left', 'Right', 'Up', 'Down', 'Home', 'End', 'Delete') -contains $key
        [TachyonProbeNative]::PressKey((Get-KeyCode $key), $extended)
        switch ($key) {
            'Left' { $script:caretModel = [Math]::Max(0, $script:caretModel - 1) }
            'Right' { $script:caretModel = [Math]::Min($length, $script:caretModel + 1) }
            'Home' { $script:caretModel = 0 }
            'End' { $script:caretModel = $length }
        }
    }
    Wait-Entry $name @{ caret = [int]$step.caret; focused = $true } ''
}

function Get-HistoryDirection([string] $direction) {
    $directions = @{ back = 'Left'; forward = 'Right' }
    if (-not $directions.ContainsKey($direction)) { throw "Unknown history direction '$direction'" }
    return $directions[$direction]
}

function Invoke-HistoryKeys([string] $direction) {
    Show-AppWindow
    [TachyonProbeNative]::PressWithAlt((Get-KeyCode (Get-HistoryDirection $direction)), $true)
    Write-Line "OK: sent Alt+$(Get-HistoryDirection $direction) to the app window"
}

function Invoke-HistoryButton([string] $direction) {
    Show-AppWindow
    $center = Get-Center (Find-AppWindow)
    if ($null -eq $center) { throw 'the app window has no bounds for a mouse button' }
    $button = [uint32](@{ Left = 1; Right = 2 }[(Get-HistoryDirection $direction)])
    [TachyonProbeNative]::ExtraButton($center[0], $center[1], $button)
    Write-Line "OK: pressed mouse button XBUTTON$button over the page"
}

function Request-Answer([string] $name) {
    $folder = $env:TAC_PROBE_HANDSHAKE
    if (-not $folder) { throw 'Set TAC_PROBE_HANDSHAKE to the handshake folder' }
    Set-Content -LiteralPath (Join-Path $folder "$name.asked") -Value 'asked'
    $done = Join-Path $folder "$name.done"
    $deadline = [DateTime]::UtcNow.AddSeconds($RequestSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if (Test-Path -LiteralPath $done) {
            Write-Line "OK: $name done"
            return
        }
        Wait-Briefly 100
    }
    throw "Nobody answered the request '$name' in $RequestSeconds seconds"
}

function ConvertTo-MenuName([string] $text) {
    return ConvertTo-Shown ((($text -split "`t")[0]) -replace '&', '')
}

function Find-MenuItem($scope, [string] $name) {
    $type = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::MenuItem)
    foreach ($item in $scope.FindAll([System.Windows.Automation.TreeScope]::Descendants, $type)) {
        if ((ConvertTo-MenuName $item.Current.Name) -eq $name) { return $item }
    }
    return $null
}

function Find-OpenMenuItem([string] $name) {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $owned = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $script:process.Id)
    foreach ($top in $root.FindAll([System.Windows.Automation.TreeScope]::Children, $owned)) {
        $item = Find-MenuItem $top $name
        if ($null -ne $item) { return $item }
    }
    return $null
}

function Open-Menu($menu) {
    $pattern = $null
    if ($menu.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern, [ref]$pattern)) {
        ([System.Windows.Automation.ExpandCollapsePattern]$pattern).Expand()
        return
    }
    if ($null -eq (Invoke-Pattern $menu)) { throw 'the File menu has no ExpandCollapse or Invoke pattern' }
}

function Invoke-MenuQuit([string] $name) {
    Show-AppWindow
    $file = Find-MenuItem (Find-AppWindow) 'File'
    if ($null -eq $file) { throw 'the window has no File menu in UI Automation' }
    Open-Menu $file
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    do {
        $item = Find-OpenMenuItem $name
        if ($null -ne $item) {
            if ($null -eq (Invoke-Pattern $item)) { throw "File > $name has no Invoke pattern" }
            return
        }
        Wait-Briefly 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "File > $name did not appear after the File menu opened"
}

function Wait-AppExit([int] $seconds) {
    return $script:process.WaitForExit($seconds * 1000)
}

function Stop-App([string] $name) {
    $how = "File > $name"
    try { Invoke-MenuQuit $name }
    catch {
        Write-Line "NOTE: could not choose File > $name through UI Automation ($($_.Exception.Message)); sending WM_CLOSE to the main window"
        $how = 'WM_CLOSE'
        if (-not [TachyonProbeNative]::CloseWindow($script:handle)) { throw 'PostMessage(WM_CLOSE) failed' }
    }
    if (-not (Wait-AppExit $WaitSeconds)) { throw "The app was still running $WaitSeconds seconds after $how" }
    $script:window = $null
    Write-Line "OK: the app quit through $how (exit code $($script:process.ExitCode))"
}

function Test-Role($item, [string] $role) {
    if ($role -eq 'Heading') {
        return ($item.Type -eq 'Text' -or $item.Type -eq 'Group') -and $item.Localized -match 'heading'
    }
    return $item.Type -eq $role
}

function Assert-Roles($pairs) {
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    $missing = @()
    $elements = $null
    do {
        Assert-AppRunning
        $elements = Get-Elements
        $missing = @()
        foreach ($pair in @($pairs)) {
            $role = [string]$pair[0]
            $name = ConvertTo-Shown ([string]$pair[1])
            $found = @($elements | Where-Object { $_.Name -eq $name -and (Test-Role $_ $role) })
            if ($found.Count -eq 0) { $missing += , @($role, $name) }
        }
        if ($missing.Count -eq 0) {
            Write-Line ('OK roles: ' + ((@($pairs) | ForEach-Object { "$($_[0]) '$($_[1])'" }) -join ', '))
            return
        }
        Wait-Briefly 200
    } while ([DateTime]::UtcNow -lt $deadline)
    $seen = foreach ($pair in $missing) {
        $named = @($elements | Where-Object { $_.Name -eq $pair[1] } | ForEach-Object { "$($_.Type)/$($_.Localized)" })
        "$($pair[0]) '$($pair[1])' (exposed as: $(if ($named.Count -gt 0) { $named -join ', ' } else { 'nothing with that name' }))"
    }
    throw "Missing roles: $($seen -join '; ')"
}

function Wait-Size($size) {
    $width = [int]$size[0]
    $height = [int]$size[1]
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    $measured = $null
    do {
        Assert-AppRunning
        $measured = [TachyonProbeNative]::Size($script:handle, $width, $height)
        if ([Math]::Abs($measured[0] - $measured[4]) -le 2 -and [Math]::Abs($measured[1] - $measured[5]) -le 2) {
            Write-Line "OK: window ${width}x${height} (frame $($measured[0])x$($measured[1]), client $($measured[2])x$($measured[3]) under the menu bar, $($measured[6]) dpi)"
            return
        }
        Wait-Briefly 200
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "The window frame is $($measured[0])x$($measured[1]) (client $($measured[2])x$($measured[3])), not $($measured[4])x$($measured[5]), the frame around a ${width}x${height} client at $($measured[6]) dpi"
}

function Show-OtherWindow([string] $title) {
    if ($null -eq $script:otherWindow) {
        $form = New-Object System.Windows.Forms.Form
        $form.Text = $title
        $form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
        $form.Left = 600
        $form.Top = 500
        $form.Width = 320
        $form.Height = 120
        $form.Show()
        $script:otherWindow = $form
    }
    [System.Windows.Forms.Application]::DoEvents()
    Wait-ForegroundWindow $script:otherWindow.Handle "'$title'"
    Write-Line "OK: '$title' is in front"
}

function Show-App {
    Show-AppWindow
    Write-Line 'OK: the app window is in front'
}

function Hide-App {
    [TachyonProbeNative]::MinimizeWindow($script:handle)
    $deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
    while (-not [TachyonProbeNative]::IsMinimized($script:handle)) {
        if ([DateTime]::UtcNow -ge $deadline) { throw 'The app window did not minimize' }
        Wait-Briefly 100
    }
    Write-Line 'OK: the app window is minimized'
}

function Write-Tree {
    if ($null -eq $script:process) { return }
    Write-Line "Process $($script:process.Id) exited: $($script:process.HasExited)"
    $front = [TachyonProbeNative]::Foreground()
    Write-Line "Foreground window: '$([TachyonProbeNative]::ClassName($front))' (handle $front); app window handle $($script:handle)"
    if ($script:handle -ne [IntPtr]::Zero) {
        Write-Line "Windows keyboard focus in the app thread: '$([TachyonProbeNative]::ClassName([TachyonProbeNative]::KeyboardFocus($script:handle)))'"
    }
    $window = $null
    try { $window = Find-AppWindow } catch { Write-Line "UIA: the window could not be read: $($_.Exception.Message)" }
    if ($null -eq $window) {
        Write-Line 'UIA: no top-level window was found for the app process'
        return
    }
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
    $queue = New-Object System.Collections.Queue
    $queue.Enqueue($window)
    $remaining = 96
    $until = [DateTime]::UtcNow.AddSeconds(5)
    while ($queue.Count -gt 0 -and $remaining -gt 0 -and [DateTime]::UtcNow -lt $until) {
        $element = $queue.Dequeue()
        try {
            $current = $element.Current
            $name = ConvertTo-Shown $current.Name
            if ($name.Length -gt 120) { $name = $name.Substring(0, 120) + '...' }
            Write-Line ("UIA: type={0}; localized={1}; name={2}; class={3}; focus={4}" -f
                ($current.ControlType.ProgrammaticName -replace '^ControlType\.', ''), $current.LocalizedControlType,
                $name, $current.ClassName, $current.HasKeyboardFocus)
            $remaining--
            $child = $walker.GetFirstChild($element)
            while ($null -ne $child -and $queue.Count -lt $remaining) {
                $queue.Enqueue($child)
                $child = $walker.GetNextSibling($child)
            }
        }
        catch { Write-Line "UIA: an element went away: $($_.Exception.Message)" }
    }
    Write-Line "UIA tree dump ended: $(96 - $remaining) nodes, $($queue.Count) not shown"
}

function Invoke-Step([string] $kind, $value) {
    switch ($kind) {
        'wait' { Wait-Texts @($value) $WaitSeconds }
        'gone' { Wait-Gone @($value) }
        'read' { Read-Text $value }
        'click' { Invoke-Control ([string]$value) }
        'pick' { Invoke-Pick $value }
        'focus' { Set-EntryFocus ([string]$value) }
        'type' { Send-Text $value }
        'move' { Move-Caret $value }
        'ask' { Request-Answer ([string]$value) }
        'quit' { Stop-App ([string]$value) }
        'roles' { Assert-Roles $value }
        'size' { Wait-Size @($value) }
        'activate' { Show-App }
        'deactivate' { Show-OtherWindow ([string]$value) }
        'minimize' { Hide-App }
        'historyKeys' { Invoke-HistoryKeys ([string]$value) }
        'historyButton' { Invoke-HistoryButton ([string]$value) }
        default { throw "Unknown probe step '$kind'" }
    }
}

function Invoke-Probe {
    Add-ProbeTypes
    $list = Read-Steps
    $script:process = Find-AppProcess
    Wait-AppWindow
    $index = 0
    foreach ($step in $list) {
        $kind = [string]$step[0]
        $value = $null
        if (@($step).Count -gt 1) { $value = $step[1] }
        Invoke-Step $kind $value
        if ($index -eq 0) {
            $seconds = ([DateTime]::UtcNow - $script:started).TotalSeconds
            Write-Line ('LOADED: the first page answered {0:N1} s after the probe started' -f $seconds)
        }
        $index++
    }
    Write-Line 'PASS: the Windows app passed every page step'
}

try {
    Invoke-Probe
    if ($null -ne $script:otherWindow) { $script:otherWindow.Close() }
    exit 0
}
catch {
    Write-Line "FAIL: $($_.Exception.Message)"
    Write-Line "At: $($_.InvocationInfo.PositionMessage)"
    try { Write-Tree } catch { Write-Line "The UIA tree dump failed: $($_.Exception.Message)" }
    if ($null -ne $script:otherWindow) { $script:otherWindow.Close() }
    exit 1
}
