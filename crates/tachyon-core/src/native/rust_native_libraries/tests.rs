use super::RustNativeLibraries;
use crate::external_command::ToolOutput;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

#[test]
fn static_native_link_preserves_library_order_and_duplicates() {
    let libraries = RustNativeLibraries::parse(
        "warning: an unrelated diagnostic\nnote: native-static-libs: -lsqlite3 -lSystem -lc -lsqlite3\n",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        libraries.arguments,
        ["-lsqlite3", "-lSystem", "-lc", "-lsqlite3"]
    );
}

#[test]
fn static_native_link_keeps_framework_arguments_together() {
    let libraries = RustNativeLibraries::parse(
        "note: native-static-libs: -framework Security -framework CoreFoundation -lSystem",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        libraries.arguments,
        [
            "-framework",
            "Security",
            "-framework",
            "CoreFoundation",
            "-lSystem"
        ]
    );
}

#[test]
fn static_native_link_accepts_an_explicit_empty_library_list() {
    let libraries = RustNativeLibraries::parse("note: native-static-libs:")
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(libraries.arguments.is_empty());
}

#[test]
fn static_native_link_reads_successful_stderr_instead_of_losing_it() {
    let output = ToolOutput {
        status: std::process::ExitStatus::from_raw(0),
        stdout: b"a compiler progress line\n".to_vec(),
        stderr: b"note: native-static-libs: -lsqlite3 -lSystem\n".to_vec(),
    };
    let libraries = RustNativeLibraries::read(&output).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(libraries.arguments, ["-lsqlite3", "-lSystem"]);
}

#[test]
fn static_native_link_accepts_stdout_metadata_and_unrelated_stderr() {
    let output = ToolOutput {
        status: std::process::ExitStatus::from_raw(0),
        stdout: b"native-static-libs: -lSystem\n".to_vec(),
        stderr: b"Finished release profile\n".to_vec(),
    };
    let libraries = RustNativeLibraries::read(&output).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(libraries.arguments, ["-lSystem"]);
}

#[test]
fn static_native_link_refuses_missing_or_unrelated_metadata() {
    for output in [
        "",
        "Finished release profile",
        "warning: native-static-libs: -lWrong",
    ] {
        let error = RustNativeLibraries::parse(output)
            .err()
            .unwrap_or_else(|| panic!("missing metadata was accepted: {output}"))
            .to_string();
        assert!(
            error.contains("did not report the native libraries"),
            "{error}"
        );
    }
}

#[test]
fn static_native_link_refuses_multiple_metadata_records() {
    for output in [
        "note: native-static-libs: -lSystem\nnote: native-static-libs: -lsqlite3",
        "note: native-static-libs: -lSystem\nnote: native-static-libs: -lSystem",
    ] {
        assert!(RustNativeLibraries::parse(output).is_err(), "{output}");
    }
}

#[test]
fn static_native_link_refuses_malformed_frameworks_and_unknown_flags() {
    for arguments in [
        "-framework",
        "-framework -lSystem",
        "-l",
        "sqlite3",
        "-o replaced-host",
        "-Wl,-force_load,other.a",
        "-lfoo;command",
        "-lfoo\0bar",
        "-framework 'Core Foundation'",
    ] {
        let error = RustNativeLibraries::parse(&format!("note: native-static-libs: {arguments}"))
            .err()
            .unwrap_or_else(|| panic!("invalid arguments accepted: {arguments}"))
            .to_string();
        assert!(
            error.contains("unsupported native static library arguments"),
            "{error}"
        );
    }
}

#[test]
fn static_native_link_default_does_not_add_libraries_to_shared_targets() {
    assert!(RustNativeLibraries::default().arguments.is_empty());
}
