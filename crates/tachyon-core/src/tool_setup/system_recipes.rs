use super::{HostPlatform, InstallContext, InstallPlan, NeededTool, PlanKind};

pub(crate) struct SystemRecipes;

impl SystemRecipes {
    const MACOS: usize = 0;
    const LINUX: usize = 1;
    const WINDOWS: usize = 2;
    const ADMIN_PREFIX: &'static str = "sudo ";
    const APPLE_ONLY: &'static str = "Apple apps (macOS and iOS) build only on a Mac with Xcode. Build them on a Mac, or remove the target.";
    const LINUX_ONLY: &'static str =
        "Linux apps build only on Linux. Build them on a Linux computer, or remove the target.";
    const WINDOWS_ONLY: &'static str = "Windows apps build only on Windows. Build them on a Windows computer, or remove the target.";
    const PYTHON: [&'static str; 3] = [
        "Install Python 3 from https://www.python.org/downloads/macos/ (the installer asks for your password), or run: brew install python",
        "sudo apt-get install -y python3\nThese are the Debian and Ubuntu commands. On Fedora use dnf, and on Arch use pacman.",
        "Install Python 3 from https://www.python.org/downloads/windows/ and choose \"Add python.exe to PATH\", or run: winget install --scope user Python.Python.3.14",
    ];
    const PHP: [&'static str; 3] = [
        "Run: brew install php (Homebrew's PHP includes php-cgi), or see https://www.php.net/downloads",
        "sudo apt-get install -y php-cgi\nThese are the Debian and Ubuntu commands. On Fedora use dnf, and on Arch use pacman.",
        "Download the PHP zip for Windows from https://windows.php.net/download/ (it holds php-cgi.exe and lists its SHA-256), unzip it into your home folder, and add that folder to your Path.",
    ];
    const RUBY: [&'static str; 3] = [
        "Run: brew install ruby, or see https://www.ruby-lang.org/en/documentation/installation/",
        "sudo apt-get install -y ruby-full\nThese are the Debian and Ubuntu commands. On Fedora use dnf, and on Arch use pacman.",
        "Install Ruby with the Devkit from https://rubyinstaller.org/downloads/ for your user.",
    ];
    const LUA: [&'static str; 3] = [
        "Run: brew install lua, or build Lua 5.4 from https://www.lua.org/download.html (check the SHA-256 listed there)",
        "sudo apt-get install -y lua5.4\nThese are the Debian and Ubuntu commands. On Fedora use dnf, and on Arch use pacman. Make sure `lua -v` prints Lua 5.4 or newer.",
        "Run: winget install --scope user DEVCOM.Lua, or download a Lua 5.4 build from https://luabinaries.sourceforge.net/ and add its folder to your Path as lua.exe",
    ];
    const C_COMPILER: [&'static str; 3] = [
        "Run: xcode-select --install\nA window opens. Choose Install and wait until it finishes.",
        "sudo apt-get install -y build-essential\nThese are the Debian and Ubuntu commands. On Fedora use dnf, and on Arch use pacman.",
        "Install Ruby with the Devkit from https://rubyinstaller.org/downloads/, then run: ridk install",
    ];
    const XCODE: [&'static str; 3] = [
        "Install Xcode from the App Store: https://apps.apple.com/app/xcode/id497799835\nOpen Xcode once and accept the license. It installs the iOS and macOS SDKs and swiftc.",
        Self::APPLE_ONLY,
        Self::APPLE_ONLY,
    ];
    const ANDROID: [&'static str; 3] = [
        "Install Android Studio from https://developer.android.com/studio, or only its command-line tools from the same page.\nIn the SDK Manager, install one Android SDK platform and the build tools.\nSet ANDROID_HOME to the SDK folder, for example: export ANDROID_HOME=\"$HOME/Library/Android/sdk\"",
        "Install Android Studio from https://developer.android.com/studio, or only its command-line tools from the same page.\nIn the SDK Manager, install one Android SDK platform and the build tools.\nSet ANDROID_HOME to the SDK folder, for example: export ANDROID_HOME=\"$HOME/Android/Sdk\"",
        "Install Android Studio from https://developer.android.com/studio, or only its command-line tools from the same page.\nIn the SDK Manager, install one Android SDK platform and the build tools.\nSet ANDROID_HOME to the SDK folder, for example: setx ANDROID_HOME \"%LOCALAPPDATA%\\Android\\Sdk\"",
    ];
    const LINUX_DESKTOP: [&'static str; 3] = [
        Self::LINUX_ONLY,
        "sudo apt-get install -y build-essential pkg-config libgtk-4-dev libwebkitgtk-6.0-dev squashfs-tools\nThese are the Debian and Ubuntu commands. The app links GTK 4 and WebKitGTK 6.0. Release packages (ty bundle --release) also need dpkg-deb and mksquashfs from squashfs-tools.\nOn Fedora run: sudo dnf install gcc pkgconf-pkg-config gtk4-devel webkitgtk6.0-devel squashfs-tools dpkg\nOn Arch run: sudo pacman -S base-devel pkgconf gtk4 webkitgtk-6.0 squashfs-tools dpkg",
        Self::LINUX_ONLY,
    ];
    const MSVC: [&'static str; 3] = [
        Self::WINDOWS_ONLY,
        Self::WINDOWS_ONLY,
        "Install the Visual Studio Build Tools from https://visualstudio.microsoft.com/visual-cpp-build-tools/ and choose the \"Desktop development with C++\" workload. It installs cl and the Windows SDK, which has rc.exe.\nRun ty from a Visual Studio developer prompt, so that cl and rc.exe are on PATH. Open the prompt for the architecture you build (x64 or ARM64).",
    ];
    const WEBVIEW2: [&'static str; 3] = [
        Self::WINDOWS_ONLY,
        Self::WINDOWS_ONLY,
        "Download the Microsoft.Web.WebView2 package from https://www.nuget.org/packages/Microsoft.Web.WebView2 (Download package). The .nupkg file is a zip file: unzip it into a folder in your home folder.\nSet TAC_WEBVIEW2_SDK to that folder, for example: setx TAC_WEBVIEW2_SDK \"%USERPROFILE%\\WebView2SDK\"\nThe folder must hold build\\native\\include\\WebView2.h.",
    ];

    pub(crate) fn plan(tool: NeededTool, context: &InstallContext) -> InstallPlan {
        Self::advice(context, tool, Self::texts(tool))
    }

    fn texts(tool: NeededTool) -> [&'static str; 3] {
        match tool {
            NeededTool::Python => Self::PYTHON,
            NeededTool::Php => Self::PHP,
            NeededTool::Ruby => Self::RUBY,
            NeededTool::Lua => Self::LUA,
            NeededTool::CCompiler => Self::C_COMPILER,
            NeededTool::Xcode => Self::XCODE,
            NeededTool::AndroidSdk => Self::ANDROID,
            NeededTool::Msvc => Self::MSVC,
            NeededTool::WebView2Sdk => Self::WEBVIEW2,
            _ => Self::LINUX_DESKTOP,
        }
    }

    fn advice(context: &InstallContext, tool: NeededTool, texts: [&'static str; 3]) -> InstallPlan {
        let text = texts[Self::system_index(context.platform)];
        let lines = text.lines().map(String::from).collect();
        InstallPlan::advice(tool, Self::kind(text), lines)
    }

    fn kind(text: &str) -> PlanKind {
        if text.starts_with(Self::ADMIN_PREFIX) {
            return PlanKind::NeedsAdmin;
        }
        PlanKind::Manual
    }

    fn system_index(platform: HostPlatform) -> usize {
        if platform.is_macos() {
            Self::MACOS
        } else if platform.is_windows() {
            Self::WINDOWS
        } else {
            Self::LINUX
        }
    }
}
