#![allow(clippy::expect_used)]

use super::managed_native_capabilities::ManagedNativeCapabilities;
use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;
use std::process::Command;

struct ManagedCapabilityCheck;

impl ManagedCapabilityCheck {
    fn available(program: &str, flag: &str) -> bool {
        Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    const CSHARP: &'static str = "[Service]\n[NeedsAll]\npublic class ImportService\n{\n    public readonly FilePicker Picker;\n    public readonly WindowControls Controls;\n    public string Choose() => Picker.Open().Status.ToString().ToLowerInvariant();\n    public bool Allowed() => Controls.Granted(\"minimize\");\n}\n";
    const KOTLIN: &'static str = "@Service\n@NeedsAll\nclass ImportService {\n    val picker: FilePicker\n    val controls: WindowControls\n    fun choose(): String = picker.open().status.wire\n    fun allowed(): Boolean = controls.granted(\"minimize\")\n}\n";
    const CSHARP_CALLER: &'static str = r#"public static unsafe class Program {
    private static int called, freed, replyMode;
    [UnmanagedCallersOnly]
    private static IntPtr Reply(IntPtr operation, IntPtr request, nuint* length) {
        var capability = Marshal.PtrToStringUTF8(operation);
        var result = capability == "window.granted" ? "{\"granted\":true}" : capability.StartsWith("window.") ? "{\"ok\":true}" : "{\"status\":\"cancelled\"}";
        var bytes = Encoding.UTF8.GetBytes(result);
        if (replyMode == 2) bytes = new byte[] { 255 };
        *length = replyMode == 1 ? 1048577 : (nuint)bytes.Length;
        var pointer = Marshal.AllocCoTaskMem(bytes.Length);
        Marshal.Copy(bytes, 0, pointer, bytes.Length);
        called++;
        return pointer;
    }
    [UnmanagedCallersOnly]
    private static void Free(IntPtr answer, nuint length) { Marshal.FreeCoTaskMem(answer); freed++; }
    public static void Main() {
        var owner = "6F9619FF-8B86-D011-B42D-00C04FC964FF";
        var prefix = "{\"route\":\"/\",\"owner\":\"" + owner + "\",\"service\":\"import\",";
        var init = prefix + "\"op\":\"init\"}";
        var choose = prefix + "\"op\":\"call\",\"name\":\"choose\",\"args\":[]}";
        var allowed = prefix + "\"op\":\"call\",\"name\":\"allowed\",\"args\":[]}";
        var start = Tac.Invoke(init);
        if (start.Contains("picker") || start.Contains("controls")) throw new Exception("Provider leaked to page fields");
        if (!Tac.Invoke(choose).Contains("unsupported")) throw new Exception("Default provider was advertised as supported");
        Tac.Invoke(prefix + "\"op\":\"dispose\"}");
        NativeCapabilities.Call = (capability, request) => {
            if (!request.Contains(owner)) throw new Exception("Missing activation owner");
            return capability == "window.granted" ? "{\"granted\":true}" : "{\"status\":\"cancelled\"}";
        };
        Tac.Invoke(init);
        NativeCapabilities.Call = (_, _) => "{\"status\":\"unsupported\"}";
        if (!Tac.Invoke(choose).Contains("cancelled") || !Tac.Invoke(allowed).Contains("true")) throw new Exception("Provider did not retain the registered fake");
        var retained = NativeCapabilities.Create("retained", () => new FilePicker(NativeCapabilities.Provider()));
        if (retained.Open(new(true)).Status != FilePickerStatus.Invalid) throw new Exception("Invalid options were accepted");
        NativeCapabilities.Dispose("retained");
        NativeCapabilities.Create("retained", () => new FilePicker(NativeCapabilities.Provider()));
        if (retained.Open().Status != FilePickerStatus.Disposed) throw new Exception("A reused owner revived a disposed provider");
        NativeCapabilities.Call = (_, _) => throw new Exception("private/path/secret");
        var failing = NativeCapabilities.Create("failing", () => new FilePicker(NativeCapabilities.Provider()));
        if (failing.Open().Status != FilePickerStatus.Unavailable) throw new Exception("Transport failure was not sanitized");
        if (FilePickerResult.Decode("{\"status\":\"denied\",\"name\":\"private/path\",\"text\":\"secret\"}").Text != null) throw new Exception("Failure response exposed private data");
        if (FilePickerResult.Decode("{\"status\":\"selected\",\"handle\":\"opaque\",\"name\":\"private/path\"}").Status != FilePickerStatus.Unavailable) throw new Exception("Selection exposed a path");
        if (failing.ReadText(new string('a', 65537)).Status != FilePickerStatus.Invalid) throw new Exception("Oversize request reached adapter");
        var register = (delegate* unmanaged<IntPtr, IntPtr, void>)&NativeCapabilityRegistration.Register;
        register((IntPtr)(delegate* unmanaged<IntPtr, IntPtr, nuint*, IntPtr>)&Reply, (IntPtr)(delegate* unmanaged<IntPtr, nuint, void>)&Free);
        var linkedPicker = NativeCapabilities.Create("linked-picker", () => new FilePicker(NativeCapabilities.Provider()));
        var linkedWindow = NativeCapabilities.Create("linked-window", () => new WindowControls(NativeCapabilities.Provider()));
        NativeCapabilities.Call = (_, _) => "{\"status\":\"unsupported\"}";
        if (linkedPicker.Open().Status != FilePickerStatus.Cancelled || !linkedWindow.Granted("minimize")) throw new Exception("Native capability ABI did not retain paired transport");
        linkedWindow.Minimize();
        replyMode = 1;
        if (linkedPicker.Open().Status != FilePickerStatus.Unavailable) throw new Exception("Oversize response accepted");
        replyMode = 2;
        if (linkedPicker.Open().Status != FilePickerStatus.Unavailable) throw new Exception("Invalid UTF-8 response accepted");
        if (called < 4 || called != freed) throw new Exception("Native capability ABI did not release each response exactly once");
        Console.WriteLine("PASS managed capabilities");
    }
}
"#;
    const KOTLIN_CALLER: &'static str = r#"fun main() {
    val owner = "6F9619FF-8B86-D011-B42D-00C04FC964FF"
    val prefix = "{\"route\":\"/\",\"owner\":\"$owner\",\"service\":\"import\","
    val init = prefix + "\"op\":\"init\"}"
    val choose = prefix + "\"op\":\"call\",\"name\":\"choose\",\"args\":[]}"
    val allowed = prefix + "\"op\":\"call\",\"name\":\"allowed\",\"args\":[]}"
    val start = tacInvokeOwned(init)
    check(!start.contains("picker") && !start.contains("controls")) { "Provider leaked to page fields: $start" }
    check(tacInvokeOwned(choose).contains("unsupported")) { "Default provider was advertised as supported" }
    tacInvokeOwned(prefix + "\"op\":\"dispose\"}")
    TacNativeCapabilityRegistration.register(TacNativeCapabilityCall { capability, request ->
        check(request.contains(owner)) { "Missing activation owner" }
        if (capability == "window.granted") "{\"granted\":true}" else "{\"status\":\"cancelled\"}"
    })
    tacInvokeOwned(init)
    NativeCapabilities.call = { _, _ -> "{\"status\":\"unsupported\"}" }
    check(tacInvokeOwned(choose).contains("cancelled") && tacInvokeOwned(allowed).contains("true")) { "Provider did not retain registered fake" }
    val retained = NativeCapabilities.create("retained") { FilePicker(NativeCapabilities.provider()) }
    check(retained.open(FilePickerOptions(multiple = true)).status == FilePickerStatus.INVALID)
    NativeCapabilities.dispose("retained")
    NativeCapabilities.create("retained") { FilePicker(NativeCapabilities.provider()) }
    check(retained.open().status == FilePickerStatus.DISPOSED) { "A reused owner revived a disposed provider" }
    NativeCapabilities.call = { _, _ -> error("private/path/secret") }
    val failing = NativeCapabilities.create("failing") { FilePicker(NativeCapabilities.provider()) }
    check(failing.open().status == FilePickerStatus.UNAVAILABLE)
    check(FilePickerResult.decode("{\"status\":\"denied\",\"name\":\"private/path\",\"text\":\"secret\"}").text == null)
    check(FilePickerResult.decode("{\"status\":\"selected\",\"handle\":\"opaque\",\"name\":\"private/path\"}").status == FilePickerStatus.UNAVAILABLE)
    check(failing.readText("a".repeat(65537)).status == FilePickerStatus.INVALID)
    println("PASS managed capabilities")
}
"#;

    fn registry(root: &Path, language: NativeCompanion, explicit: bool) -> String {
        let (extension, source) = if language == NativeCompanion::CSharp {
            ("cs", Self::CSHARP)
        } else {
            ("kt", Self::KOTLIN)
        };
        let source = if explicit {
            source
                .replace("[NeedsAll]\n", "")
                .replace("@NeedsAll\n", "")
                .replace("    public readonly", "    [Needs]\n    public readonly")
                .replace("    val ", "    @Needs\n    val ")
        } else {
            source.to_owned()
        };
        let file = root.join(format!("client/services/ImportService.{extension}"));
        std::fs::create_dir_all(file.parent().expect("parent")).expect("directory");
        std::fs::write(&file, source).expect("source");
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language,
            source: file,
            service: String::from("import"),
            class: String::from("ImportService"),
            web_assembly: false,
            application_provider: false,
        };
        super::registry::source(&[input], language)
            .expect("registry")
            .expect("source")
    }

    fn succeeds(command: &mut Command) -> String {
        let output = command.output().expect("tool");
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn csharp(explicit: bool) -> String {
        let root = tempfile::tempdir().expect("project");
        let registry = Self::registry(root.path(), NativeCompanion::CSharp, explicit);
        let program = format!(
            "using System;\nusing System.Collections.Generic;\nusing System.Collections;\nusing System.Globalization;\nusing System.Linq;\nusing System.Text.Json.Nodes;\nusing System.Runtime.InteropServices;\nusing System.Text.Json;\nusing System.Text;\ninternal static class TacApplication {{ internal const string Id = \"test.managed\"; }}\n{registry}\n{}\n{}\n{}",
            concat!(
                include_str!("prelude.cs"),
                include_str!("models/TacModels.cs")
            ),
            super::csharp_web_apis::CsharpWebApis::native(),
            Self::CSHARP_CALLER
        );
        std::fs::write(root.path().join("Program.cs"), program).expect("program");
        std::fs::write(root.path().join("capabilities.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net9.0</TargetFramework><AllowUnsafeBlocks>true</AllowUnsafeBlocks><Nullable>disable</Nullable><EnableDefaultCompileItems>false</EnableDefaultCompileItems></PropertyGroup><ItemGroup><Compile Include=\"Program.cs\" /></ItemGroup></Project>").expect("project");
        Self::succeeds(
            Command::new("dotnet")
                .arg("run")
                .arg("--project")
                .arg(root.path().join("capabilities.csproj")),
        )
    }

    fn kotlin(explicit: bool) -> String {
        let root = tempfile::tempdir().expect("project");
        let registry = Self::registry(root.path(), NativeCompanion::Kotlin, explicit);
        let platform = include_str!("prelude_web.kt")
            .split("private const val tacPageListeners")
            .nth(1)
            .expect("platform")
            .split("@OptIn(kotlin.js.ExperimentalJsExport::class)")
            .next()
            .expect("plain platform");
        let platform = format!(
            "private fun tacHostEmit(payload: String) {{}}\nprivate fun tacHostWeb(text: String): String = \"{{}}\"\nprivate const val tacPageListeners{platform}"
        );
        let models = include_str!("prelude_android.kt")
            .split("private fun tacPlainModel")
            .nth(1)
            .expect("native model functions")
            .split("@Synchronized")
            .next()
            .expect("model helpers");
        let models = format!(
            "{}{}private fun tacPlainModel{models}",
            include_str!("prelude_jvm_models.kt"),
            include_str!("prelude_native_tasks.kt")
        );
        let program = format!(
            "{registry}\n{}{}\n{platform}\n{models}\n{}\n{}\n{}\n{}\n{}",
            include_str!("prelude.kt"),
            include_str!("prelude_reads.kt"),
            ManagedNativeCapabilities::KOTLIN,
            ManagedNativeCapabilities::KOTLIN_WINDOWS,
            include_str!("capabilities/TacNativeCapabilityCall.kt"),
            include_str!("capabilities/TacNativeCapabilityRegistration.kt"),
            Self::KOTLIN_CALLER
        );
        let source = root.path().join("Capabilities.kt");
        std::fs::write(&source, program).expect("program");
        let jar = root.path().join("capabilities.jar");
        Self::succeeds(
            Command::new("kotlinc")
                .arg(&source)
                .arg("-include-runtime")
                .arg("-d")
                .arg(&jar),
        );
        Self::succeeds(Command::new("java").arg("-jar").arg(&jar))
    }

    fn android_companion() {
        let Some(platform) = std::env::var_os("ANDROID_HOME")
            .map(std::path::PathBuf::from)
            .map(|home| home.join("platforms/android-35/android.jar"))
            .filter(|platform| platform.is_file())
        else {
            eprintln!("skipping: Android platform 35 is unavailable");
            return;
        };
        let root = tempfile::tempdir().expect("project");
        Self::registry(root.path(), NativeCompanion::Kotlin, false);
        let input = NativeCompanionInput {
            snapshot_root: root.path().to_path_buf(),
            language: NativeCompanion::Kotlin,
            source: root.path().join("client/services/ImportService.kt"),
            service: String::from("import"),
            class: String::from("ImportService"),
            web_assembly: false,
            application_provider: false,
        };
        let project = root.path().join("android");
        assert!(
            super::android::stage_companion(
                &[input],
                &project,
                "app.capabilities",
                "app/capabilities"
            )
            .expect("stage")
        );
        Self::succeeds(
            Command::new("kotlinc")
                .arg("-classpath")
                .arg(platform)
                .arg(project.join("app/src/main/kotlin"))
                .arg("-d")
                .arg(root.path().join("classes")),
        );
    }
}

#[test]
fn managed_native_csharp_needs_and_needs_all_capture_providers() {
    if !ManagedCapabilityCheck::available("dotnet", "--version") {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    for explicit in [false, true] {
        assert!(ManagedCapabilityCheck::csharp(explicit).contains("PASS managed capabilities"));
    }
}

#[test]
fn managed_native_kotlin_needs_and_needs_all_capture_providers() {
    if !ManagedCapabilityCheck::available("kotlinc", "-version")
        || !ManagedCapabilityCheck::available("java", "-version")
    {
        eprintln!("skipping: Kotlin JVM tools are unavailable");
        return;
    }
    for explicit in [false, true] {
        assert!(ManagedCapabilityCheck::kotlin(explicit).contains("PASS managed capabilities"));
    }
    ManagedCapabilityCheck::android_companion();
}
