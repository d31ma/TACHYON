function Add-DesktopTypes {
    if ('TachyonDesktop' -as [type]) { return }
    Add-Type -Language CSharp -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public static class TachyonDesktop
{
    [StructLayout(LayoutKind.Sequential)]
    public struct Box { public int Left; public int Top; public int Right; public int Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    struct Spot { public int X; public int Y; }

    [StructLayout(LayoutKind.Sequential)]
    struct MonitorInfo { public int Size; public Box Monitor; public Box Work; public uint Flags; }

    [StructLayout(LayoutKind.Sequential)]
    struct NotifyIconIdentifier { public uint Size; public IntPtr Window; public uint Id; public Guid Item; }

    [StructLayout(LayoutKind.Sequential)]
    struct MouseInput { public int X; public int Y; public uint Data; public uint Flags; public uint Time; public IntPtr Extra; }

    [StructLayout(LayoutKind.Sequential)]
    struct KeyInput { public ushort Key; public ushort Scan; public uint Flags; public uint Time; public IntPtr Extra; }

    [StructLayout(LayoutKind.Explicit)]
    struct InputData { [FieldOffset(0)] public MouseInput Mouse; [FieldOffset(0)] public KeyInput Key; }

    [StructLayout(LayoutKind.Sequential)]
    struct Input { public uint Kind; public InputData Data; }

    [StructLayout(LayoutKind.Sequential)]
    struct ThreadInfo
    {
        public int Size; public uint Flags; public IntPtr Active; public IntPtr Focus; public IntPtr Capture;
        public IntPtr MenuOwner; public IntPtr MoveSize; public IntPtr Caret; public Box CaretBox;
    }

    [StructLayout(LayoutKind.Sequential)]
    struct MachineInfo { public ushort Machine; public ushort Reserved; public uint Attributes; }

    delegate bool WindowVisitor(IntPtr window, IntPtr data);

    [DllImport("user32.dll")] static extern bool EnumWindows(WindowVisitor visitor, IntPtr data);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassNameW(IntPtr window, StringBuilder name, int size);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowTextW(IntPtr window, StringBuilder text, int size);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll")] static extern bool IsZoomed(IntPtr window);
    [DllImport("user32.dll")] static extern bool IsWindow(IntPtr window);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr window, out Box box);
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr window, out Box box);
    [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr window, ref Spot spot);
    [DllImport("user32.dll")] static extern int GetWindowLongW(IntPtr window, int index);
    [DllImport("user32.dll")] static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll")] static extern bool GetMonitorInfoW(IntPtr monitor, ref MonitorInfo info);
    [DllImport("user32.dll")] static extern IntPtr SendMessageTimeoutW(IntPtr window, uint message, IntPtr first, IntPtr second, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll")] static extern bool PostMessageW(IntPtr window, uint message, IntPtr first, IntPtr second);
    [DllImport("user32.dll")] static extern bool ShowWindowAsync(IntPtr window, int command);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr window);
    [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint from, uint to, bool attach);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(Spot spot);
    [DllImport("user32.dll")] static extern IntPtr GetAncestor(IntPtr window, uint flags);
    [DllImport("user32.dll", SetLastError = true)] static extern uint SendInput(uint count, Input[] inputs, int size);
    [DllImport("user32.dll")] static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll")] static extern int GetSystemMetricsForDpi(int index, uint dpi);
    [DllImport("user32.dll")] static extern uint MapVirtualKeyW(uint code, uint kind);
    [DllImport("user32.dll")] static extern IntPtr GetMenu(IntPtr window);
    [DllImport("user32.dll")] static extern int GetMenuItemCount(IntPtr menu);
    [DllImport("user32.dll")] static extern IntPtr GetSubMenu(IntPtr menu, int position);
    [DllImport("user32.dll")] static extern uint GetMenuItemID(IntPtr menu, int position);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetMenuStringW(IntPtr menu, uint item, StringBuilder text, int size, uint flags);
    [DllImport("user32.dll")] static extern bool GetGUIThreadInfo(uint thread, ref ThreadInfo info);
    [DllImport("user32.dll")] static extern bool SetProcessDpiAwarenessContext(IntPtr context);
    [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
    [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr window, int attribute, out Box value, int size);
    [DllImport("dwmapi.dll", EntryPoint = "DwmGetWindowAttribute")] static extern int DwmGetWindowNumber(IntPtr window, int attribute, out uint value, int size);
    [DllImport("shell32.dll")] static extern int Shell_NotifyIconGetRect(ref NotifyIconIdentifier identifier, out Box box);
    [DllImport("kernel32.dll")] static extern IntPtr OpenProcess(uint access, bool inherit, uint id);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll")] static extern bool GetProcessInformation(IntPtr process, int kind, ref MachineInfo info, uint size);
    [DllImport("kernel32.dll")] static extern bool IsWow64Process2(IntPtr process, out ushort processMachine, out ushort nativeMachine);
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();

    public static void UsePhysicalPixels()
    {
        IntPtr perMonitor = new IntPtr(-4);
        try { SetProcessDpiAwarenessContext(perMonitor); } catch (EntryPointNotFoundException) { }
        try { SetThreadDpiAwarenessContext(perMonitor); } catch (EntryPointNotFoundException) { }
    }

    public static IntPtr[] Windows(int process, string className, bool visibleOnly)
    {
        List<IntPtr> found = new List<IntPtr>();
        WindowVisitor visitor = delegate (IntPtr window, IntPtr data)
        {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            if (owner == (uint)process && ClassOf(window) == className && (!visibleOnly || IsWindowVisible(window))) found.Add(window);
            return true;
        };
        EnumWindows(visitor, IntPtr.Zero);
        GC.KeepAlive(visitor);
        return found.ToArray();
    }

    public static string ClassOf(IntPtr window)
    {
        StringBuilder name = new StringBuilder(256);
        GetClassNameW(window, name, name.Capacity);
        return name.ToString();
    }

    public static string TitleOf(IntPtr window)
    {
        StringBuilder text = new StringBuilder(1024);
        GetWindowTextW(window, text, text.Capacity);
        return text.ToString();
    }

    public static bool Exists(IntPtr window) { return IsWindow(window); }
    public static bool Visible(IntPtr window) { return IsWindowVisible(window); }
    public static bool Minimized(IntPtr window) { return IsIconic(window); }
    public static bool Maximized(IntPtr window) { return IsZoomed(window); }
    public static int Style(IntPtr window) { return GetWindowLongW(window, -16); }
    public static uint Dpi(IntPtr window) { return GetDpiForWindow(window); }
    public static int Metric(int index, uint dpi) { return GetSystemMetricsForDpi(index, dpi); }
    public static IntPtr MenuOf(IntPtr window) { return GetMenu(window); }
    public static IntPtr Foreground() { return GetForegroundWindow(); }

    public static Box WindowBox(IntPtr window)
    {
        Box box;
        GetWindowRect(window, out box);
        return box;
    }

    public static Box FrameBox(IntPtr window)
    {
        Box box;
        if (DwmGetWindowAttribute(window, 9, out box, Marshal.SizeOf(typeof(Box))) != 0) GetWindowRect(window, out box);
        return box;
    }

    public static Box ClientBox(IntPtr window)
    {
        Box box;
        GetClientRect(window, out box);
        Spot origin = new Spot();
        ClientToScreen(window, ref origin);
        box.Left += origin.X; box.Right += origin.X; box.Top += origin.Y; box.Bottom += origin.Y;
        return box;
    }

    public static Box MonitorBox(IntPtr window)
    {
        MonitorInfo info = new MonitorInfo();
        info.Size = Marshal.SizeOf(typeof(MonitorInfo));
        GetMonitorInfoW(MonitorFromWindow(window, 2), ref info);
        return info.Monitor;
    }

    public static Box CaptionButtons(IntPtr window)
    {
        Box buttons;
        int result = DwmGetWindowAttribute(window, 5, out buttons, Marshal.SizeOf(typeof(Box)));
        if (result != 0) throw new InvalidOperationException(string.Format("DWMWA_CAPTION_BUTTON_BOUNDS failed with 0x{0:X8}", result));
        Box frame = WindowBox(window);
        buttons.Left += frame.Left; buttons.Right += frame.Left; buttons.Top += frame.Top; buttons.Bottom += frame.Top;
        return buttons;
    }

    public static int CaptionColor(IntPtr window, out uint color)
    {
        return DwmGetWindowNumber(window, 35, out color, 4);
    }

    public static int HitTest(IntPtr window, int x, int y)
    {
        IntPtr point = new IntPtr(((y & 0xFFFF) << 16) | (x & 0xFFFF));
        IntPtr answer;
        if (SendMessageTimeoutW(window, 0x84, IntPtr.Zero, point, 2, 3000, out answer) == IntPtr.Zero) return -1000;
        return (int)answer.ToInt64();
    }

    public static IntPtr WindowAt(int x, int y)
    {
        Spot spot = new Spot();
        spot.X = x; spot.Y = y;
        return WindowFromPoint(spot);
    }

    public static IntPtr ParentOf(IntPtr window) { return GetAncestor(window, 1); }

    public static IntPtr FocusedWindow()
    {
        ThreadInfo info = new ThreadInfo();
        info.Size = Marshal.SizeOf(typeof(ThreadInfo));
        return GetGUIThreadInfo(0, ref info) ? info.Focus : IntPtr.Zero;
    }

    public static bool Post(IntPtr window, uint message, long first, long second)
    {
        return PostMessageW(window, message, new IntPtr(first), new IntPtr(second));
    }

    public static void Show(IntPtr window, int command) { ShowWindowAsync(window, command); }

    public static void Place(IntPtr window, int x, int y)
    {
        SetWindowPos(window, IntPtr.Zero, x, y, 0, 0, 0x0001 | 0x0004 | 0x0010);
    }

    public static bool Front(IntPtr window)
    {
        if (IsIconic(window)) { ShowWindowAsync(window, 9); Thread.Sleep(300); }
        SetWindowPos(window, new IntPtr(-1), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0010);
        SetWindowPos(window, new IntPtr(-2), 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0010);
        if (SetForegroundWindow(window) && GetForegroundWindow() == window) return true;
        uint ignored;
        uint foreground = GetWindowThreadProcessId(GetForegroundWindow(), out ignored);
        uint current = GetCurrentThreadId();
        if (foreground != current) AttachThreadInput(current, foreground, true);
        BringWindowToTop(window);
        SetForegroundWindow(window);
        if (foreground != current) AttachThreadInput(current, foreground, false);
        if (GetForegroundWindow() == window) return true;
        Keys(new ushort[] { 0x12 });
        SetForegroundWindow(window);
        Thread.Sleep(100);
        return GetForegroundWindow() == window;
    }

    static Input Mouse(int x, int y, uint flags)
    {
        int left = GetSystemMetrics(76), top = GetSystemMetrics(77);
        int width = Math.Max(2, GetSystemMetrics(78)), height = Math.Max(2, GetSystemMetrics(79));
        Input input = new Input();
        input.Kind = 0;
        input.Data.Mouse.X = (int)Math.Round((x - left) * 65535.0 / (width - 1));
        input.Data.Mouse.Y = (int)Math.Round((y - top) * 65535.0 / (height - 1));
        input.Data.Mouse.Flags = flags | 0x0001 | 0x8000 | 0x4000;
        return input;
    }

    static void Send(Input[] inputs)
    {
        uint sent = SendInput((uint)inputs.Length, inputs, Marshal.SizeOf(typeof(Input)));
        if (sent != inputs.Length) throw new InvalidOperationException(string.Format("SendInput sent {0} of {1} events (error {2}); is the desktop locked?", sent, inputs.Length, Marshal.GetLastWin32Error()));
    }

    public static void MoveTo(int x, int y) { Send(new Input[] { Mouse(x, y, 0) }); }

    public static void Click(int x, int y)
    {
        MoveTo(x, y);
        Thread.Sleep(60);
        Send(new Input[] { Mouse(x, y, 0x0002) });
        Thread.Sleep(40);
        Send(new Input[] { Mouse(x, y, 0x0004) });
    }

    public static void RightClick(int x, int y)
    {
        MoveTo(x, y);
        Thread.Sleep(60);
        Send(new Input[] { Mouse(x, y, 0x0008) });
        Thread.Sleep(40);
        Send(new Input[] { Mouse(x, y, 0x0010) });
    }

    public static void DoubleClick(int x, int y)
    {
        Click(x, y);
        Thread.Sleep(80);
        Send(new Input[] { Mouse(x, y, 0x0002) });
        Thread.Sleep(40);
        Send(new Input[] { Mouse(x, y, 0x0004) });
    }

    public static void Drag(int x, int y, int distanceX, int distanceY, int steps)
    {
        MoveTo(x, y);
        Thread.Sleep(120);
        Send(new Input[] { Mouse(x, y, 0x0002) });
        Thread.Sleep(200);
        for (int step = 1; step <= steps; step++)
        {
            MoveTo(x + distanceX * step / steps, y + distanceY * step / steps);
            Thread.Sleep(25);
        }
        Thread.Sleep(200);
        Send(new Input[] { Mouse(x + distanceX, y + distanceY, 0x0004) });
        Thread.Sleep(200);
    }

    static Input Key(ushort key, bool up)
    {
        Input input = new Input();
        input.Kind = 1;
        input.Data.Key.Key = key;
        input.Data.Key.Scan = (ushort)MapVirtualKeyW(key, 0);
        input.Data.Key.Flags = up ? 0x0002u : 0u;
        return input;
    }

    public static void Keys(ushort[] chord)
    {
        List<Input> inputs = new List<Input>();
        foreach (ushort key in chord) inputs.Add(Key(key, false));
        for (int index = chord.Length - 1; index >= 0; index--) inputs.Add(Key(chord[index], true));
        foreach (Input input in inputs) { Send(new Input[] { input }); Thread.Sleep(30); }
    }

    public static void Type(string text)
    {
        foreach (char character in text)
        {
            Input down = new Input();
            down.Kind = 1;
            down.Data.Key.Scan = character;
            down.Data.Key.Flags = 0x0004;
            Input up = down;
            up.Data.Key.Flags = 0x0004 | 0x0002;
            Send(new Input[] { down, up });
            Thread.Sleep(15);
        }
    }

    static string MenuText(IntPtr menu, int position)
    {
        StringBuilder text = new StringBuilder(512);
        GetMenuStringW(menu, (uint)position, text, text.Capacity, 0x0400);
        return text.ToString().Replace("&&", "\u0001").Replace("&", "").Replace("\u0001", "&");
    }

    public static string[] MenuTitles(IntPtr window)
    {
        IntPtr bar = GetMenu(window);
        List<string> titles = new List<string>();
        if (bar == IntPtr.Zero) return titles.ToArray();
        for (int position = 0; position < GetMenuItemCount(bar); position++) titles.Add(MenuText(bar, position));
        return titles.ToArray();
    }

    static IntPtr Popup(IntPtr window, string title)
    {
        IntPtr bar = GetMenu(window);
        if (bar == IntPtr.Zero) return IntPtr.Zero;
        for (int position = 0; position < GetMenuItemCount(bar); position++)
        {
            if (MenuText(bar, position) == title) return GetSubMenu(bar, position);
        }
        return IntPtr.Zero;
    }

    public static string[] MenuLabels(IntPtr window, string title)
    {
        IntPtr popup = Popup(window, title);
        List<string> labels = new List<string>();
        if (popup == IntPtr.Zero) return labels.ToArray();
        for (int position = 0; position < GetMenuItemCount(popup); position++)
        {
            string text = MenuText(popup, position);
            if (text.Length > 0) labels.Add(text);
        }
        return labels.ToArray();
    }

    public static int MenuCommand(IntPtr window, string title, string label)
    {
        IntPtr popup = Popup(window, title);
        if (popup == IntPtr.Zero) return -1;
        for (int position = 0; position < GetMenuItemCount(popup); position++)
        {
            string text = MenuText(popup, position);
            int tab = text.IndexOf('\t');
            if ((tab < 0 ? text : text.Substring(0, tab)) == label) return (int)GetMenuItemID(popup, position);
        }
        return -1;
    }

    public static int TrayIconBox(IntPtr window, uint id, out Box box)
    {
        NotifyIconIdentifier identifier = new NotifyIconIdentifier();
        identifier.Size = (uint)Marshal.SizeOf(typeof(NotifyIconIdentifier));
        identifier.Window = window;
        identifier.Id = id;
        return Shell_NotifyIconGetRect(ref identifier, out box);
    }

    public static int ProcessMachine(int process)
    {
        IntPtr handle = OpenProcess(0x1000, false, (uint)process);
        if (handle == IntPtr.Zero) return -1;
        try
        {
            MachineInfo info = new MachineInfo();
            return GetProcessInformation(handle, 9, ref info, (uint)Marshal.SizeOf(typeof(MachineInfo))) ? info.Machine : -1;
        }
        catch (EntryPointNotFoundException) { return -1; }
        finally { CloseHandle(handle); }
    }

    public static int HostMachine()
    {
        try
        {
            ushort processMachine, nativeMachine;
            return IsWow64Process2(GetCurrentProcess(), out processMachine, out nativeMachine) ? nativeMachine : -1;
        }
        catch (EntryPointNotFoundException) { return -1; }
    }
}
'@
}

function Initialize-DesktopGate([string] $outputDirectory) {
    Set-Location (Join-Path $PSScriptRoot '..\..')
    $script:failures = [System.Collections.Generic.List[string]]::new()
    $script:shots = $outputDirectory
    $script:process = $null
    $script:window = [IntPtr]::Zero
    $script:log = $null
    $script:appName = ''
    $script:appFailed = $false
    $script:baseEnvironment = @{}
    foreach ($entry in [Environment]::GetEnvironmentVariables('Process').GetEnumerator()) {
        $script:baseEnvironment[[string]$entry.Key] = [string]$entry.Value
    }
    New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
    Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, WindowsBase, System.Drawing
    Add-DesktopTypes
    [TachyonDesktop]::UsePhysicalPixels()
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    if ((Get-Process -Id $PID).SessionId -eq 0 -or
        -not ($identity.Groups -contains [Security.Principal.SecurityIdentifier]'S-1-5-4')) {
        throw 'this gate needs the signed-in desktop session; run it through scripts/windows/desktop-run.ps1'
    }
    Write-Host "Screenshots go to $outputDirectory"
}

function Get-HostArchitecture {
    $machine = [TachyonDesktop]::HostMachine()
    if ($machine -eq 0xAA64) { return 'arm64' }
    if ($machine -eq 0x8664) { return 'x64' }
    $named = @($env:PROCESSOR_ARCHITEW6432, $env:PROCESSOR_ARCHITECTURE) | Where-Object { $_ } | Select-Object -First 1
    if ($named -eq 'ARM64') { return 'arm64' }
    return 'x64'
}

function Restore-BaseEnvironment {
    foreach ($name in @([Environment]::GetEnvironmentVariables('Process').Keys)) {
        if (-not $script:baseEnvironment.ContainsKey([string]$name)) {
            [Environment]::SetEnvironmentVariable([string]$name, $null, 'Process')
        }
    }
    foreach ($entry in $script:baseEnvironment.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'Process')
    }
}

function Enable-Msvc([string] $architecture) {
    $compiler = Get-Command cl.exe -ErrorAction SilentlyContinue
    if ($compiler -and $env:VSCMD_ARG_TGT_ARCH -eq $architecture) { return }
    if ($compiler -and -not $env:VSCMD_ARG_TGT_ARCH -and $architecture -eq (Get-HostArchitecture)) { return }
    Restore-BaseEnvironment
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { throw 'MSVC requires Visual Studio Build Tools' }
    $component = 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64'
    if ($architecture -eq 'arm64') { $component = 'Microsoft.VisualStudio.Component.VC.Tools.ARM64' }
    $installation = & $vswhere -latest -products '*' -requires $component -property installationPath
    if ($LASTEXITCODE -ne 0 -or -not $installation) { throw "MSVC $architecture tools ($component) are not installed" }
    $hostArchitecture = Get-HostArchitecture
    $developer = Join-Path $installation 'Common7\Tools\VsDevCmd.bat'
    $environment = & $env:ComSpec /d /s /c "`"$developer`" -no_logo -arch=$architecture -host_arch=$hostArchitecture >nul && set"
    if ($LASTEXITCODE -ne 0) { throw "could not initialize the MSVC developer environment for $architecture" }
    foreach ($line in $environment) {
        if ($line -match '^([^=]+)=(.*)$') {
            [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process')
        }
    }
    Remove-Item Env:Platform -ErrorAction SilentlyContinue
    $compiler = Get-Command cl.exe -ErrorAction SilentlyContinue
    if (-not $compiler) { throw "VsDevCmd -arch=$architecture -host_arch=$hostArchitecture put no cl.exe on PATH" }
    Write-Host "MSVC: $architecture tools on a $hostArchitecture host ($($compiler.Source); VSCMD_ARG_TGT_ARCH=$env:VSCMD_ARG_TGT_ARCH)"
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
    cargo build --locked --bin ty | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    $target = 'target'
    if ($env:CARGO_TARGET_DIR) { $target = $env:CARGO_TARGET_DIR }
    return (Resolve-Path (Join-Path $target 'debug\ty.exe')).Path
}

function Get-FixtureChecks {
    $switches = @('dependencies', 'names', 'signatures', 'constants', 'visibility', 'types', 'tests',
        'cohesion', 'duplication', 'accessibility') | ForEach-Object { "`"$_`": { `"check`": false }" }
    return ($switches -join ",`n    ")
}

function Write-FixtureFile([string] $root, [string] $relative, [string] $text) {
    $path = Join-Path $root $relative
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    [IO.File]::WriteAllText($path, $text.Replace('__CHECKS__', (Get-FixtureChecks)), [Text.UTF8Encoding]::new($false))
}

function Build-GateApp([string] $ty, [string] $fixture, [string] $architecture) {
    if ($architecture -eq 'native') {
        Write-Host "Building $fixture with: ty build --target windows"
        & $ty build $fixture --target windows | Out-Host
    }
    else {
        Write-Host "Building $fixture with: ty bundle --target windows --arch $architecture"
        & $ty bundle $fixture --target windows --arch $architecture | Out-Host
    }
    if ($LASTEXITCODE -ne 0) { throw "ty could not build the Windows app (exit code $LASTEXITCODE)" }
}

function Get-AppExecutable([string] $fixture, [string] $name) {
    return (Join-Path $fixture "dist\windows\$name\bin\$name.exe")
}

function Get-PeMachine([string] $path) {
    $stream = [IO.File]::OpenRead($path)
    try {
        $reader = [IO.BinaryReader]::new($stream)
        $stream.Seek(0x3C, 'Begin') | Out-Null
        $header = $reader.ReadInt32()
        $stream.Seek($header, 'Begin') | Out-Null
        if ($reader.ReadUInt32() -ne 0x4550) { throw "$path is not a PE file" }
        return [int]$reader.ReadUInt16()
    }
    finally { $stream.Dispose() }
}

function Assert-That([bool] $condition, [string] $message) {
    if (-not $condition) { throw $message }
}

function Assert-Running {
    if ($null -eq $script:process) { throw 'the app is not running' }
    if ($script:process.HasExited) { throw "the app exited with code $($script:process.ExitCode)" }
}

function Test-Check([string] $name, [scriptblock] $check) {
    try { & $check }
    catch {
        $message = $_.Exception.Message
        $script:failures.Add("$($script:appName): $name -> $message")
        $script:appFailed = $true
        Write-Host "FAIL: $name -> $message"
    }
}

function Wait-Until([scriptblock] $condition, [int] $seconds, [string] $what) {
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        if (& $condition) { return }
        Start-Sleep -Milliseconds 150
    } while ([DateTime]::UtcNow -lt $until)
    throw "timed out after $seconds s waiting for $what"
}

function Start-GateApp([string] $executable, [string] $applicationId) {
    if (-not (Test-Path $executable)) { throw "missing artifact $executable" }
    $script:log = Join-Path $env:LOCALAPPDATA "Tachyon\$applicationId.jsonl"
    if (Test-Path $script:log) { Remove-Item $script:log }
    $script:process = Start-Process -FilePath $executable -PassThru
    $script:window = [IntPtr]::Zero
    Wait-Until {
        Assert-Running
        $found = @([TachyonDesktop]::Windows($script:process.Id, 'TachyonNativeHost', $true))
        if ($found.Count -gt 0) { $script:window = $found[0] }
        $script:window -ne [IntPtr]::Zero
    } 30 'the app window'
    Write-Host ("Started {0} (pid {1}, window 0x{2:X})" -f $executable, $script:process.Id, $script:window.ToInt64())
}

function Get-AppRoot {
    return [System.Windows.Automation.AutomationElement]::FromHandle($script:window)
}

function Find-Named([string] $name) {
    try {
        $condition = [System.Windows.Automation.PropertyCondition]::new(
            [System.Windows.Automation.AutomationElement]::NameProperty, $name)
        return (Get-AppRoot).FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
    }
    catch { return $null }
}

function Find-Matching([string] $pattern) {
    try {
        $all = (Get-AppRoot).FindAll([System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
        foreach ($element in $all) {
            if ($element.Current.Name -match $pattern) { return $element }
        }
    }
    catch { return $null }
    return $null
}

function Wait-Named([string[]] $names, [int] $seconds = 20) {
    $missing = $names
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        Assert-Running
        $missing = @($names | Where-Object { $null -eq (Find-Named $_) })
        if ($missing.Count -eq 0) { return (Find-Named $names[0]) }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $until)
    throw "the page does not show: $($missing -join ', ')"
}

function Wait-Matching([string] $pattern, [int] $seconds = 20) {
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        Assert-Running
        $element = Find-Matching $pattern
        if ($null -ne $element) { return $element.Current.Name }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $until)
    throw "the page shows no text matching '$pattern'"
}

function Invoke-Named([string] $name) {
    $element = Wait-Named @($name)
    $pattern = $null
    if (-not $element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        throw "the control '$name' exposes no UI Automation InvokePattern"
    }
    ([System.Windows.Automation.InvokePattern]$pattern).Invoke()
}

function Get-Center($element) {
    $box = $element.Current.BoundingRectangle
    if ($box.IsEmpty -or [double]::IsInfinity($box.Width) -or $box.Width -le 0) {
        throw "the element '$($element.Current.Name)' has no place on the screen"
    }
    return @([int]($box.X + $box.Width / 2), [int]($box.Y + $box.Height / 2))
}

function Show-AppWindow {
    if (-not [TachyonDesktop]::Front($script:window)) {
        Write-Host 'NOTE: Windows kept another window in the foreground; continuing with the app raised to the top'
    }
    Start-Sleep -Milliseconds 300
}

function Click-Element($element) {
    Show-AppWindow
    $center = Get-Center $element
    [TachyonDesktop]::Click($center[0], $center[1])
    Start-Sleep -Milliseconds 250
}

function Get-FieldValue([string] $name) {
    $element = Find-Named $name
    if ($null -eq $element) { return $null }
    return $element.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value
}

function Set-FieldValue([string] $name, [string] $text) {
    $element = Wait-Named @($name)
    try { $element.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($text) }
    catch {
        Write-Host "NOTE: ValuePattern.SetValue failed ($($_.Exception.Message)); typing the text instead"
        Click-Element $element
        [TachyonDesktop]::Keys(@([uint16]0x11, [uint16]0x41))
        [TachyonDesktop]::Type($text)
    }
}

function Get-ClipboardText {
    try { return [string](Get-Clipboard -Format Text -Raw) }
    catch { return $null }
}

function Invoke-MenuCommand([string] $title, [string] $label) {
    $command = [TachyonDesktop]::MenuCommand($script:window, $title, $label)
    if ($command -lt 0) { throw "the $title menu has no '$label' item" }
    [TachyonDesktop]::Post($script:window, 0x0111, $command, 0) | Out-Null
    Start-Sleep -Milliseconds 400
}

function Assert-MenuLabels([string] $title, [string[]] $labels) {
    $present = @([TachyonDesktop]::MenuLabels($script:window, $title))
    $missing = @($labels | Where-Object { $present -notcontains $_ })
    if ($missing.Count -gt 0) {
        throw "the $title menu lacks $($missing -join ', '); it holds: $(($present -replace "`t", ' ') -join '; ')"
    }
}

function Save-ScreenShot([string] $name, [int] $left, [int] $top, [int] $right, [int] $bottom) {
    $bitmap = [System.Drawing.Bitmap]::new([Math]::Max(1, $right - $left), [Math]::Max(1, $bottom - $top))
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try { $graphics.CopyFromScreen($left, $top, 0, 0, $bitmap.Size) }
    finally { $graphics.Dispose() }
    $path = Join-Path $script:shots "$name.png"
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Host "SHOT: $path"
    return [pscustomobject]@{ Path = $path; Bitmap = $bitmap; Left = $left; Top = $top }
}

function Save-WindowShot([string] $name) {
    $frame = [TachyonDesktop]::FrameBox($script:window)
    return (Save-ScreenShot $name $frame.Left $frame.Top $frame.Right $frame.Bottom)
}

function Measure-Color($shot, [int] $left, [int] $top, [int] $right, [int] $bottom, [int[]] $color, [int] $tolerance) {
    $matched = 0
    $total = 0
    $sum = @(0, 0, 0)
    for ($y = $top; $y -lt $bottom; $y += 2) {
        for ($x = $left; $x -lt $right; $x += 3) {
            $pixelX = $x - $shot.Left
            $pixelY = $y - $shot.Top
            if ($pixelX -lt 0 -or $pixelY -lt 0 -or $pixelX -ge $shot.Bitmap.Width -or $pixelY -ge $shot.Bitmap.Height) { continue }
            $pixel = $shot.Bitmap.GetPixel($pixelX, $pixelY)
            $total++
            $sum[0] += $pixel.R
            $sum[1] += $pixel.G
            $sum[2] += $pixel.B
            if ([Math]::Abs($pixel.R - $color[0]) -le $tolerance -and [Math]::Abs($pixel.G - $color[1]) -le $tolerance -and
                [Math]::Abs($pixel.B - $color[2]) -le $tolerance) { $matched++ }
        }
    }
    if ($total -eq 0) { throw "the sampled area ($left,$top)-($right,$bottom) lies outside the screenshot" }
    $average = '#{0:x2}{1:x2}{2:x2}' -f [int]($sum[0] / $total), [int]($sum[1] / $total), [int]($sum[2] / $total)
    return [pscustomobject]@{ Matched = $matched; Total = $total; Share = $matched / $total; Average = $average }
}

function Wait-Popup([int] $seconds = 5) {
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    do {
        $found = @([TachyonDesktop]::Windows($script:process.Id, '#32768', $true))
        if ($found.Count -gt 0) {
            Start-Sleep -Milliseconds 300
            return $found[0]
        }
        Start-Sleep -Milliseconds 150
    } while ([DateTime]::UtcNow -lt $until)
    throw "timed out after $seconds s waiting for a popup menu of the app"
}

function Get-PopupItems([IntPtr] $popup) {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::MenuItem)
    return @([System.Windows.Automation.AutomationElement]::FromHandle($popup).FindAll(
        [System.Windows.Automation.TreeScope]::Descendants, $condition))
}

function Get-PopupNames([IntPtr] $popup) {
    return @(Get-PopupItems $popup | ForEach-Object { ($_.Current.Name -split "`t")[0].Trim() })
}

function Invoke-PopupItem([IntPtr] $popup, [string] $label) {
    $item = Get-PopupItems $popup | Where-Object { ($_.Current.Name -split "`t")[0].Trim() -eq $label } | Select-Object -First 1
    if ($null -eq $item) { throw "the popup menu has no '$label' item; it holds: $((Get-PopupNames $popup) -join ', ')" }
    if (-not $item.Current.IsEnabled) { throw "the popup menu item '$label' is disabled" }
    $pattern = $null
    if ($item.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$pattern)) {
        ([System.Windows.Automation.InvokePattern]$pattern).Invoke()
    }
    else {
        $center = Get-Center $item
        [TachyonDesktop]::Click($center[0], $center[1])
    }
    Start-Sleep -Milliseconds 400
}

function Close-Popup {
    [TachyonDesktop]::Post($script:window, 0x001F, 0, 0) | Out-Null
    [TachyonDesktop]::Keys(@([uint16]0x1B))
    Start-Sleep -Milliseconds 300
}

function Write-UiaTree {
    try { $root = Get-AppRoot }
    catch { Write-Host 'UIA: the app window is not available'; return }
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
    $queue = [System.Collections.Queue]::new()
    $queue.Enqueue($root)
    $remaining = 64
    $until = [DateTime]::UtcNow.AddSeconds(3)
    while ($queue.Count -gt 0 -and $remaining -gt 0 -and [DateTime]::UtcNow -lt $until) {
        $element = $queue.Dequeue()
        $current = $element.Current
        $name = ($current.Name -replace '[\r\n\t]', ' ')
        if ($name.Length -gt 120) { $name = $name.Substring(0, 120) + '...' }
        Write-Host ("UIA: type={0}; name={1}; class={2}" -f $current.ControlType.ProgrammaticName, $name, $current.ClassName)
        $remaining--
        $child = $walker.GetFirstChild($element)
        while ($null -ne $child -and $queue.Count -lt $remaining) {
            $queue.Enqueue($child)
            $child = $walker.GetNextSibling($child)
        }
    }
}

function Write-GateDiagnostics {
    Write-Host "Diagnostics for $($script:appName) (before cleanup):"
    if ($script:log -and (Test-Path $script:log)) {
        Write-Host "Native lifecycle log $($script:log) (last 24 records):"
        Get-Content -LiteralPath $script:log -Tail 24 | ForEach-Object { Write-Host $_ }
    }
    else { Write-Host 'Native lifecycle log: absent' }
    if ($null -eq $script:process) { return }
    $script:process.Refresh()
    if ($script:process.HasExited) {
        Write-Host "App process: exited with code $($script:process.ExitCode)"
        return
    }
    $foreground = [TachyonDesktop]::Foreground()
    $focus = [TachyonDesktop]::FocusedWindow()
    Write-Host ("App window: 0x{0:X}; visible={1}; minimized={2}; maximized={3}; foreground={4} ({5}); focus class={6}" -f
        $script:window.ToInt64(), [TachyonDesktop]::Visible($script:window), [TachyonDesktop]::Minimized($script:window),
        [TachyonDesktop]::Maximized($script:window), ($foreground -eq $script:window), [TachyonDesktop]::ClassOf($foreground),
        [TachyonDesktop]::ClassOf($focus))
    if ([TachyonDesktop]::Visible($script:window)) { Write-UiaTree }
}

function Stop-GateApp {
    $process = $script:process
    $script:process = $null
    $script:window = [IntPtr]::Zero
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
        if (-not $ownedProcess.WaitForExit($wait)) { & taskkill /PID $ownedProcess.Id /T /F | Out-Null }
    }
    foreach ($ownedProcess in $owned) {
        if (-not $ownedProcess.WaitForExit(5000)) { Write-Warning "process $($ownedProcess.Id) did not stop" }
    }
}

function Remove-Fixture([string] $fixture) {
    $until = [DateTime]::UtcNow.AddSeconds(10)
    while (Test-Path -LiteralPath $fixture) {
        try { Remove-Item -LiteralPath $fixture -Recurse -Force; return }
        catch {
            if ([DateTime]::UtcNow -ge $until) { Write-Warning "could not remove $fixture"; return }
            Start-Sleep -Milliseconds 300
        }
    }
}

function Invoke-GateApp([string] $name, [scriptblock] $write, [string] $ty, [string] $architecture, [scriptblock] $checks) {
    $script:appName = $name
    $script:appFailed = $false
    $fixture = Join-Path $env:TEMP ('ty-' + ($name -replace '\W', '-').ToLowerInvariant() + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    Write-Host ''
    Write-Host "=== $name ($fixture)"
    try {
        & $write $fixture
        Build-GateApp $ty $fixture $architecture
    }
    catch {
        $script:failures.Add("${name}: build -> $($_.Exception.Message)")
        Write-Host "FAIL: build -> $($_.Exception.Message)"
        Write-Host "Fixture kept for review: $fixture"
        return
    }
    try { & $checks $fixture }
    catch {
        $script:failures.Add("${name}: stopped -> $($_.Exception.Message)")
        $script:appFailed = $true
        Write-Host "FAIL: stopped -> $($_.Exception.Message)"
    }
    finally {
        if ($script:appFailed) {
            try { Write-GateDiagnostics }
            catch { Write-Warning "diagnostics were incomplete: $($_.Exception.Message)" }
        }
        try { Stop-GateApp }
        catch { Write-Warning "cleanup failed: $($_.Exception.Message)" }
        if ($script:appFailed) { Write-Host "Fixture kept for review: $fixture" }
        else { Remove-Fixture $fixture }
    }
}

function Complete-Gate([string] $passed) {
    Write-Host ''
    Write-Host "Screenshots: $($script:shots)"
    if ($script:failures.Count -gt 0) {
        Write-Host "FAIL: $($script:failures.Count) check(s) failed:"
        foreach ($failure in $script:failures) { Write-Host "  - $failure" }
        exit 1
    }
    Write-Host "PASS: $passed"
    exit 0
}
