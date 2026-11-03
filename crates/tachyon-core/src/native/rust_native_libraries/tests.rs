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
fn static_native_link_refuses_metadata_from_a_failed_compiler() {
    let output = ToolOutput {
        status: std::process::ExitStatus::from_raw(1),
        stdout: Vec::new(),
        stderr: b"note: native-static-libs: -lsqlite3\n".to_vec(),
    };
    let error = RustNativeLibraries::read(&output)
        .err()
        .unwrap_or_else(|| panic!("failed compilation was accepted"))
        .to_string();
    assert!(error.contains("Rust compilation failed"), "{error}");
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

#[test]
fn static_native_link_name_alphabet_matches_the_exact_ascii_contract() {
    for value in 0_u8..=127 {
        let character = char::from(value);
        let name = format!("Library{character}Name");
        let expected =
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '+');
        assert_eq!(
            RustNativeLibraries::require_name(&name).is_ok(),
            expected,
            "wrong acceptance for ASCII byte {value}"
        );
        let leading_name = format!("{character}Library");
        assert_eq!(
            RustNativeLibraries::require_name(&leading_name).is_ok(),
            expected && character != '-',
            "wrong leading-character acceptance for ASCII byte {value}"
        );
    }
}

#[test]
fn static_native_link_name_boundaries_keep_the_exact_refusal_diagnostic() {
    let expected = RustNativeLibraries::invalid().to_string();
    for name in [
        "",
        "-",
        "-System",
        "Å",
        "Library中",
        "Library\u{fffd}",
        "Library💡",
    ] {
        let error = RustNativeLibraries::require_name(name)
            .err()
            .unwrap_or_else(|| panic!("invalid name was accepted: {name:?}"));
        assert_eq!(error.to_string(), expected, "{name:?}");
    }
    for arguments in ["-l", "-l-System", "-framework", "-framework -System"] {
        let error = RustNativeLibraries::parse(&format!("native-static-libs: {arguments}"))
            .err()
            .unwrap_or_else(|| panic!("invalid token boundary was accepted: {arguments}"));
        assert_eq!(error.to_string(), expected, "{arguments}");
    }
}

#[test]
fn static_native_link_marker_whitespace_keeps_record_and_token_boundaries() {
    for metadata in [
        "\t native-static-libs:\t-lsqlite3\t-framework Security \t",
        "\t note: native-static-libs: -lsqlite3 -framework Security \t",
    ] {
        let libraries = RustNativeLibraries::parse(metadata)
            .unwrap_or_else(|error| panic!("{metadata:?}: {error}"));
        assert_eq!(libraries.arguments, ["-lsqlite3", "-framework", "Security"]);
    }
    let missing = RustNativeLibraries::missing().to_string();
    for metadata in [
        "note:\tnative-static-libs: -lSystem",
        "note:native-static-libs: -lSystem",
        "warning: native-static-libs: -lSystem",
    ] {
        let error = RustNativeLibraries::parse(metadata)
            .err()
            .unwrap_or_else(|| panic!("wrong marker prefix was accepted: {metadata}"));
        assert_eq!(error.to_string(), missing, "{metadata}");
    }
    let error = RustNativeLibraries::parse(
        "native-static-libs: -lSystem\n\t note: native-static-libs: -lSystem",
    )
    .err()
    .unwrap_or_else(|| panic!("duplicate records were accepted"));
    assert_eq!(
        error.to_string(),
        RustNativeLibraries::invalid().to_string()
    );
}
