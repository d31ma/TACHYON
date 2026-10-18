#![allow(clippy::expect_used)]

struct FilePickerLimitCheck;

impl FilePickerLimitCheck {
    const IMPORTS: &str = "import AppKit\nimport UniformTypeIdentifiers\n";
    const CALLER: &str = r#"import Foundation

@main @MainActor struct PickerLimits {
    static func owner(_ index: Int) -> String { String(format: "00000000-0000-4000-8000-%012d", index) }

    static func status(_ answer: String) -> FilePickerStatus {
        let result = try? JSONDecoder().decode(FilePickerResult.self, from: Data(answer.utf8))
        return result?.status ?? .unavailable
    }

    static func handle(_ answer: String) -> String {
        let result = try? JSONDecoder().decode(FilePickerResult.self, from: Data(answer.utf8))
        return result?.handle ?? ""
    }

    static func refusesTheHandleAfterTheLimit(_ file: URL) {
        let picker = MacOsFilePicker()
        picker.granted = true
        let owners = MacOsFilePicker.handleLimit / MacOsFilePicker.ownerLimit
        for index in 0..<owners {
            for _ in 0..<MacOsFilePicker.ownerLimit { _ = picker.keep(owner(index), file, scoped: false) }
        }
        precondition(picker.files.count == MacOsFilePicker.handleLimit, "the fixture must fill every handle")
        let refused = picker.call("open", ["owner": owner(owners), "types": ["text/plain"]])
        precondition(status(refused) == .unavailable, "the handle after the limit must be refused: \(refused)")
        precondition(picker.files.count == MacOsFilePicker.handleLimit, "a refused open must not add a handle")
        print("handle limit refused")
    }

    static func refusesAnOwnerAfterItsLimit(_ file: URL) {
        let picker = MacOsFilePicker()
        picker.granted = true
        for _ in 0..<MacOsFilePicker.ownerLimit { _ = picker.keep(owner(0), file, scoped: false) }
        let refused = picker.call("open", ["owner": owner(0), "types": ["text/plain"]])
        precondition(status(refused) == .unavailable, "one owner must not pass its own limit: \(refused)")
        print("owner limit refused")
    }

    static func removesAnExpiredHandle(_ file: URL) {
        let picker = MacOsFilePicker()
        picker.granted = true
        var now = Date(timeIntervalSince1970: 1_000_000)
        picker.clock = { now }
        let kept = handle(picker.keep(owner(0), file, scoped: false))
        let request: [String: Any] = ["owner": owner(0), "handle": kept]
        now += MacOsFilePicker.lifetime - 1
        precondition(status(picker.call("readText", request)) == .read, "a handle must work until its lifetime ends")
        now += 1
        let expired = picker.call("readText", request)
        precondition(status(expired) == .invalid, "an expired handle must answer invalid: \(expired)")
        precondition(picker.files[kept] == nil, "an expired handle must be removed")
        precondition(status(picker.call("release", request)) == .invalid, "an expired handle cannot be released")
        print("expired handle removed")
    }

    static func main() {
        let file = URL(fileURLWithPath: CommandLine.arguments[1])
        refusesTheHandleAfterTheLimit(file)
        refusesAnOwnerAfterItsLimit(file)
        removesAnExpiredHandle(file)
    }
}
"#;

    async fn run() -> String {
        let root = tempfile::tempdir().expect("project");
        let sources = [
            (
                "FilePickerStatus.swift",
                include_str!("capabilities/FilePickerStatus.swift").to_owned(),
            ),
            (
                "FilePickerResult.swift",
                include_str!("capabilities/FilePickerResult.swift").to_owned(),
            ),
            (
                "MacOsFilePicker.swift",
                format!(
                    "{}{}",
                    Self::IMPORTS,
                    include_str!("capabilities/MacOsFilePicker.swift")
                ),
            ),
            ("PickerLimits.swift", Self::CALLER.to_owned()),
        ];
        let paths = sources
            .iter()
            .map(|(name, source)| {
                let path = root.path().join(name);
                std::fs::write(&path, source).expect("swift source");
                path.to_string_lossy().into_owned()
            })
            .collect::<Vec<_>>();
        let selected = root.path().join("selected.txt");
        std::fs::write(&selected, "selected\n").expect("selected file");
        let program = root.path().join("picker-limits");
        let program = program.to_string_lossy();
        let mut arguments = vec!["swiftc", "-parse-as-library"];
        arguments.extend(paths.iter().map(String::as_str));
        arguments.extend(["-o", &program]);
        super::host::run_tool("/usr/bin/xcrun", &arguments)
            .await
            .expect("compile");
        super::host::run_tool(&program, &[&selected.to_string_lossy()])
            .await
            .expect("execute")
    }
}

#[tokio::test]
async fn the_macos_file_picker_refuses_extra_handles_and_removes_expired_ones() {
    let output = FilePickerLimitCheck::run().await;
    for line in [
        "handle limit refused",
        "owner limit refused",
        "expired handle removed",
    ] {
        assert!(output.contains(line), "{output}");
    }
}
