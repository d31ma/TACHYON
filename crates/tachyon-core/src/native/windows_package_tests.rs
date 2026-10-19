#![allow(clippy::expect_used)]

use super::config::NativeApplication;
use super::native_theme_color::NativeThemeColor;
use super::windows_architecture::WindowsArchitecture;
use super::windows_package_manifest::WindowsPackageManifest;
use super::windows_package_tiles::WindowsPackageTiles;
use super::windows_release::WindowsRelease;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Read;
use std::path::Path;

struct WindowsPackageTests;

impl WindowsPackageTests {
    const PUBLISHER: &str = "CN=Tachyon Test";
    const MINGW: &str = "x86_64-w64-mingw32-gcc";

    fn application(root: &Path) -> NativeApplication {
        let mut application = NativeApplication::discover(root).expect("application");
        application.name = String::from("Notes & Lists");
        application.executable_name = String::from("Notes");
        application.application_id = String::from("ma.del.notes_app");
        application.version = String::from("2.4.1");
        application.build = 9;
        application
    }

    fn lookup<'values>(
        values: &'values BTreeMap<&'static str, String>,
    ) -> impl Fn(&str) -> Option<OsString> + 'values {
        |name| values.get(name).map(OsString::from)
    }

    fn signed_with_file(root: &Path) -> BTreeMap<&'static str, String> {
        std::fs::write(root.join("signing.pfx"), b"certificate").expect("certificate");
        BTreeMap::from([
            (WindowsRelease::CERTIFICATE, String::from("signing.pfx")),
            (
                WindowsRelease::CERTIFICATE_PASSWORD,
                String::from("secret-value"),
            ),
            (
                WindowsRelease::CERTIFICATE_PUBLISHER,
                String::from(Self::PUBLISHER),
            ),
        ])
    }

    fn refusal(values: &BTreeMap<&'static str, String>, root: &Path) -> String {
        WindowsRelease::from_environment(Self::lookup(values), root)
            .err()
            .expect("refused")
            .to_string()
    }

    fn self_signed_certificate(root: &Path) {
        let path = |name: &str| root.join(name).display().to_string();
        Self::run(std::process::Command::new("openssl").args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-sha256",
            "-days",
            "2",
            "-nodes",
            "-subj",
            "/CN=Tachyon Test",
            "-addext",
            "extendedKeyUsage=codeSigning",
            "-addext",
            "keyUsage=critical,digitalSignature",
            "-addext",
            "basicConstraints=critical,CA:FALSE",
            "-keyout",
            &path("test.key"),
            "-out",
            &path("test.crt"),
        ]));
        Self::run(std::process::Command::new("openssl").args([
            "pkcs12",
            "-export",
            "-inkey",
            &path("test.key"),
            "-in",
            &path("test.crt"),
            "-out",
            &path("signing.pfx"),
            "-passout",
            "pass:secret-value",
        ]));
    }

    fn tool_available(program: &str) -> bool {
        std::process::Command::new(program)
            .arg("--version")
            .output()
            .is_ok()
    }

    fn run(command: &mut std::process::Command) -> String {
        let output = command.output().expect("tool");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{text}");
        text
    }
}

#[test]
fn the_package_name_comes_from_the_application_id_within_the_msix_limits() {
    assert_eq!(
        WindowsPackageManifest::identity_name("ma.del.notes_app").expect("name"),
        "ma.del.notes-app"
    );
    for refused in ["ab", &"a".repeat(51)] {
        let text = WindowsPackageManifest::identity_name(refused)
            .expect_err("refused")
            .to_string();
        assert!(text.contains("TY1601"), "{text}");
        assert!(text.contains("3 to 50 letters, digits, dots and hyphens"));
    }
    assert_eq!(
        WindowsPackageManifest::publisher_display_name("O=Contoso, CN=\"Contoso Ltd\", C=US"),
        "Contoso Ltd"
    );
}

#[test]
fn the_package_manifest_names_the_app_publisher_version_and_tiles() {
    let root = tempfile::tempdir().expect("root");
    let mut application = WindowsPackageTests::application(root.path());
    let manifest = WindowsPackageManifest {
        application: &application,
        publisher: "CN=Notes & Co, O=Notes, C=US",
        architecture: WindowsArchitecture::ARM64,
    };
    let xml = manifest.xml().expect("manifest");
    assert!(xml.contains(
        r#"<Identity Name="ma.del.notes-app" Publisher="CN=Notes &amp; Co, O=Notes, C=US" Version="2.4.1.9" ProcessorArchitecture="arm64"/>"#
    ));
    assert!(xml.contains("<DisplayName>Notes &amp; Lists</DisplayName>"));
    assert!(xml.contains("<PublisherDisplayName>Notes &amp; Co</PublisherDisplayName>"));
    assert!(
        xml.contains(r#"Executable="bin\Notes.exe" EntryPoint="Windows.FullTrustApplication""#)
    );
    assert!(xml.contains(r#"<Application Id="App" "#));
    assert!(xml.contains(r#"BackgroundColor="transparent""#));
    assert!(xml.contains(r#"Square150x150Logo="Assets\Square150x150Logo.png""#));
    assert!(xml.contains(r#"Square44x44Logo="Assets\Square44x44Logo.png""#));
    assert!(xml.contains(r"<Logo>Assets\StoreLogo.png</Logo>"));
    assert!(xml.contains(r#"<rescap:Capability Name="runFullTrust"/>"#));
    assert!(!xml.contains("<Extensions>"));
    application.links = super::native_links::NativeLinks::declared(&["notes"], &[]);
    application.window.theme_color = Some(NativeThemeColor {
        red: 15,
        green: 108,
        blue: 189,
    });
    let themed = WindowsPackageManifest {
        application: &application,
        publisher: WindowsPackageTests::PUBLISHER,
        architecture: WindowsArchitecture::X64,
    }
    .xml()
    .expect("manifest");
    assert!(themed.contains(r##"BackgroundColor="#0f6cbd""##));
    assert!(themed.contains(r#"ProcessorArchitecture="x64""#));
    assert!(themed.contains(
        "Square44x44Logo=\"Assets\\Square44x44Logo.png\"/>\n      <Extensions>\n        <uap:Extension Category=\"windows.protocol\"><uap:Protocol Name=\"notes\"/></uap:Extension>\n      </Extensions>\n    </Application>"
    ));
}

#[test]
fn each_tile_takes_the_smallest_png_icon_that_covers_it() {
    let root = tempfile::tempdir().expect("root");
    let mut application = WindowsPackageTests::application(root.path());
    application.icons = vec![
        (512, String::from("/icons/512.png")),
        (192, String::from("/icons/192.png")),
        (96, String::from("/icons/96.webp")),
        (48, String::from("/icons/48.png")),
    ];
    let chosen = |tile: WindowsPackageTiles| tile.icon_for(&application).map(String::from);
    assert_eq!(
        chosen(WindowsPackageTiles::SQUARE_44).as_deref(),
        Some("/icons/48.png")
    );
    assert_eq!(
        chosen(WindowsPackageTiles::STORE_LOGO).as_deref(),
        Some("/icons/192.png")
    );
    assert_eq!(
        chosen(WindowsPackageTiles::SQUARE_150).as_deref(),
        Some("/icons/192.png")
    );
    application.icons = vec![(32, String::from("/icons/32.png"))];
    assert_eq!(
        WindowsPackageTiles::SQUARE_150.icon_for(&application),
        Some("/icons/32.png")
    );
    application.icons.clear();
    assert_eq!(WindowsPackageTiles::SQUARE_150.icon_for(&application), None);
}

#[test]
fn an_app_without_a_png_icon_gets_plain_tiles_in_its_theme_color() {
    let root = tempfile::tempdir().expect("root");
    let mut application = WindowsPackageTests::application(root.path());
    application.window.theme_color = Some(NativeThemeColor {
        red: 1,
        green: 2,
        blue: 3,
    });
    let png = WindowsPackageTiles::SQUARE_44.plain_tile(&application);
    assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    assert_eq!(&png[12..16], b"IHDR");
    assert_eq!(&png[16..24], &[0, 0, 0, 44, 0, 0, 0, 44]);
    let length =
        usize::try_from(u32::from_be_bytes([png[33], png[34], png[35], png[36]])).expect("length");
    assert_eq!(&png[37..41], b"IDAT");
    let mut pixels = Vec::new();
    flate2::read::ZlibDecoder::new(&png[41..41 + length])
        .read_to_end(&mut pixels)
        .expect("pixels");
    assert_eq!(pixels.len(), 44 * (1 + 3 * 44));
    assert_eq!(&pixels[..4], &[0, 1, 2, 3]);
    assert!(png.ends_with(&[0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130]));
}

#[test]
fn the_staged_layout_holds_the_package_manifest_and_every_tile() {
    let root = tempfile::tempdir().expect("root");
    let web = root.path().join("web");
    std::fs::create_dir_all(web.join("icons")).expect("icons");
    std::fs::write(web.join("icons/app.png"), b"declared icon").expect("icon");
    let mut application = WindowsPackageTests::application(root.path());
    application.icons = vec![(64, String::from("/icons/app.png"))];
    let values = WindowsPackageTests::signed_with_file(root.path());
    let release =
        WindowsRelease::from_environment(WindowsPackageTests::lookup(&values), root.path())
            .expect("release");
    let layout = root.path().join("Notes");
    release
        .stage_layout(&application, &web, &layout, WindowsArchitecture::X64)
        .expect("layout");
    let manifest = std::fs::read_to_string(layout.join("AppxManifest.xml")).expect("manifest");
    assert!(manifest.contains(r#"Publisher="CN=Tachyon Test""#));
    assert_eq!(
        std::fs::read(layout.join("Assets/Square44x44Logo.png")).expect("tile"),
        b"declared icon"
    );
    let largest = std::fs::read(layout.join("Assets/Square150x150Logo.png")).expect("tile");
    assert_eq!(largest, b"declared icon");
    assert!(layout.join("Assets/StoreLogo.png").is_file());
}

#[test]
fn a_release_build_names_what_is_missing_from_the_signing_environment() {
    let root = tempfile::tempdir().expect("root");
    let mut values = WindowsPackageTests::signed_with_file(root.path());
    values.remove(WindowsRelease::CERTIFICATE_PUBLISHER);
    let text = WindowsPackageTests::refusal(&values, root.path());
    assert!(text.contains("TY1605"));
    assert!(text.contains("TAC_WINDOWS_CERTIFICATE_PUBLISHER is missing or has no CN= part"));
    assert!(!text.contains("secret-value"));

    values.insert(
        WindowsRelease::CERTIFICATE_PUBLISHER,
        String::from("O=Contoso"),
    );
    assert!(WindowsPackageTests::refusal(&values, root.path()).contains("has no CN= part"));

    let mut values = WindowsPackageTests::signed_with_file(root.path());
    values.remove(WindowsRelease::CERTIFICATE);
    assert!(
        WindowsPackageTests::refusal(&values, root.path())
            .contains("neither TAC_WINDOWS_CERTIFICATE nor TAC_WINDOWS_CERTIFICATE_THUMBPRINT")
    );

    let mut values = WindowsPackageTests::signed_with_file(root.path());
    values.insert(WindowsRelease::CERTIFICATE_THUMBPRINT, "AB".repeat(20));
    assert!(WindowsPackageTests::refusal(&values, root.path()).contains("Set only one of"));

    let mut values = WindowsPackageTests::signed_with_file(root.path());
    values.insert(WindowsRelease::CERTIFICATE, String::from("missing.pfx"));
    let text = WindowsPackageTests::refusal(&values, root.path());
    assert!(text.contains("missing.pfx', which is not a certificate file"));
    assert!(!text.contains("secret-value"));

    let mut values = WindowsPackageTests::signed_with_file(root.path());
    values.remove(WindowsRelease::CERTIFICATE);
    values.insert(
        WindowsRelease::CERTIFICATE_THUMBPRINT,
        String::from("12 34"),
    );
    assert!(WindowsPackageTests::refusal(&values, root.path()).contains("40 hexadecimal digits"));

    let mut values = WindowsPackageTests::signed_with_file(root.path());
    values.insert(
        WindowsRelease::CERTIFICATE_TIMESTAMP_URL,
        String::from("ftp://time.example"),
    );
    assert!(
        WindowsPackageTests::refusal(&values, root.path())
            .contains("must be an http:// or https:// timestamp server")
    );
}

#[test]
fn makeappx_and_signtool_get_sha256_and_the_chosen_certificate() {
    let root = tempfile::tempdir().expect("root");
    assert_eq!(
        WindowsRelease::pack_arguments(r"C:\stage\Notes", r"C:\stage\Notes.msix"),
        [
            "pack",
            "/o",
            "/h",
            "SHA256",
            "/d",
            r"C:\stage\Notes",
            "/p",
            r"C:\stage\Notes.msix"
        ]
    );
    let mut values = WindowsPackageTests::signed_with_file(root.path());
    values.insert(
        WindowsRelease::CERTIFICATE_TIMESTAMP_URL,
        String::from("http://timestamp.example"),
    );
    let release =
        WindowsRelease::from_environment(WindowsPackageTests::lookup(&values), root.path())
            .expect("release");
    let certificate = root.path().join("signing.pfx").display().to_string();
    assert_eq!(
        release.sign_arguments("Notes.msix"),
        [
            "sign",
            "/fd",
            "SHA256",
            "/f",
            &certificate,
            "/p",
            "secret-value",
            "/tr",
            "http://timestamp.example",
            "/td",
            "SHA256",
            "Notes.msix"
        ]
    );
    let stored = BTreeMap::from([
        (
            WindowsRelease::CERTIFICATE_THUMBPRINT,
            format!("{} ", "ab12".repeat(10)),
        ),
        (
            WindowsRelease::CERTIFICATE_PUBLISHER,
            String::from(WindowsPackageTests::PUBLISHER),
        ),
    ]);
    let release =
        WindowsRelease::from_environment(WindowsPackageTests::lookup(&stored), root.path())
            .expect("release");
    assert_eq!(release.publisher(), WindowsPackageTests::PUBLISHER);
    assert_eq!(
        release.sign_arguments("Notes.msix"),
        [
            "sign",
            "/fd",
            "SHA256",
            "/sha1",
            &"AB12".repeat(10),
            "Notes.msix"
        ]
    );
}

#[test]
fn the_windows_host_gives_webview2_the_package_folder_only_when_packaged() {
    let source = super::windows::WindowsHostGenerator::PACKAGE_DATA;
    assert!(source.contains("GetProcAddress(kernel, \"GetCurrentPackageFamilyName\")"));
    assert!(source.contains(r#"L"%ls\\Packages\\%ls\\LocalState\\WebView2""#));
    let root = tempfile::tempdir().expect("root");
    let application = WindowsPackageTests::application(root.path());
    let index = super::routes::NativeRouteIndex {
        contract_version: 2,
        entry_route: String::from("/"),
        entry_document: String::from("index.html"),
        routes: Vec::new(),
    };
    let host = super::windows::c_source(&application, &index, &[]);
    assert!(host.contains("static wchar_t *tachyon_package_data_folder(void) {"));
    assert!(host.contains(
        "  wchar_t *data_folder = tachyon_package_data_folder();\n  CreateCoreWebView2EnvironmentWithOptions(\n      NULL, data_folder, NULL,"
    ));
    assert!(!host.contains("__PACKAGE_DATA__"));
}

#[test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler and wine; runs the package folder helper unpackaged, not inside an MSIX package"]
fn the_package_folder_helper_runs_unpackaged_under_wine() {
    let root = tempfile::tempdir().expect("root");
    let source = root.path().join("package.c");
    let program = root.path().join("package.exe");
    let harness = format!(
        "#include <windows.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <wchar.h>\n{}\n{}",
        super::windows::WindowsHostGenerator::PACKAGE_DATA,
        "int main(void) {\n  SetEnvironmentVariableW(L\"LOCALAPPDATA\", L\"C:\\\\Users\\\\me\\\\AppData\\\\Local\");\n  wchar_t *folder = tachyon_package_folder_for(L\"Notes_8wekyb3d8bbwe\");\n  wprintf(L\"%ls\\n\", folder);\n  free(folder);\n  wprintf(L\"%ls\\n\", tachyon_package_data_folder() == NULL ? L\"unpackaged\" : L\"packaged\");\n  return 0;\n}\n"
    );
    std::fs::write(&source, harness).expect("source");
    WindowsPackageTests::run(
        std::process::Command::new(WindowsPackageTests::MINGW)
            .args(["-std=gnu17", "-Wall", "-Wextra", "-Werror", "-DUNICODE"])
            .arg(&source)
            .arg("-o")
            .arg(&program),
    );
    let output = WindowsPackageTests::run(
        std::process::Command::new("wine")
            .env("WINEDEBUG", "-all")
            .arg(&program),
    );
    assert!(
        output.contains(
            r"C:\Users\me\AppData\Local\Packages\Notes_8wekyb3d8bbwe\LocalState\WebView2"
        ),
        "{output}"
    );
    assert!(output.contains("unpackaged"), "{output}");
}

#[test]
#[ignore = "Requires makemsix from the MSIX SDK (TAC_MAKEMSIX), osslsigncode and openssl; validates the package against Microsoft's schemas and signs it, not makeappx, signtool or an install"]
fn the_staged_layout_packs_and_signs_with_the_msix_sdk() {
    let makemsix = std::env::var_os("TAC_MAKEMSIX").expect("TAC_MAKEMSIX names makemsix");
    assert!(WindowsPackageTests::tool_available("osslsigncode"));
    let root = tempfile::tempdir().expect("root");
    let path = |name: &str| root.path().join(name).display().to_string();
    WindowsPackageTests::self_signed_certificate(root.path());
    let web = root.path().join("web");
    std::fs::create_dir_all(&web).expect("web");
    let mut application = WindowsPackageTests::application(root.path());
    application.links = super::native_links::NativeLinks::declared(&["notes", "notes+v2"], &[]);
    let values = BTreeMap::from([
        (WindowsRelease::CERTIFICATE, String::from("signing.pfx")),
        (
            WindowsRelease::CERTIFICATE_PASSWORD,
            String::from("secret-value"),
        ),
        (
            WindowsRelease::CERTIFICATE_PUBLISHER,
            String::from(WindowsPackageTests::PUBLISHER),
        ),
    ]);
    let release =
        WindowsRelease::from_environment(WindowsPackageTests::lookup(&values), root.path())
            .expect("release");
    let layout = root.path().join("Notes");
    std::fs::create_dir_all(layout.join("bin")).expect("bin");
    std::fs::write(layout.join("bin/Notes.exe"), b"MZ").expect("executable");
    release
        .stage_layout(&application, &web, &layout, WindowsArchitecture::X64)
        .expect("layout");
    WindowsPackageTests::run(std::process::Command::new(&makemsix).args([
        "pack",
        "-d",
        &path("Notes"),
        "-p",
        &path("Notes.msix"),
    ]));
    WindowsPackageTests::run(std::process::Command::new("osslsigncode").args([
        "sign",
        "-pkcs12",
        &path("signing.pfx"),
        "-pass",
        "secret-value",
        "-in",
        &path("Notes.msix"),
        "-out",
        &path("Notes.signed.msix"),
    ]));
    let verified = WindowsPackageTests::run(std::process::Command::new("osslsigncode").args([
        "verify",
        "-CAfile",
        &path("test.crt"),
        "-in",
        &path("Notes.signed.msix"),
    ]));
    assert!(
        verified.contains("Signature verification: ok"),
        "{verified}"
    );
    WindowsPackageTests::run(std::process::Command::new(&makemsix).args([
        "unpack",
        "-p",
        &path("Notes.signed.msix"),
        "-d",
        &path("unpacked"),
        "-ac",
    ]));
    let unpacked =
        std::fs::read_to_string(root.path().join("unpacked/AppxManifest.xml")).expect("manifest");
    assert!(unpacked.contains(r#"Name="ma.del.notes-app""#));
}
