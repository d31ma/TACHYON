use std::fmt::Write as _;
use std::path::Path;

pub(super) struct NativeCapabilities;

impl NativeCapabilities {
    pub(super) fn swift_service(source: &str, path: &Path, class: &str) -> (String, Vec<String>) {
        let fields =
            crate::template::required_fields::RequiredFields::constructed(path, source, class);
        let mut output = source.to_owned();
        let mut excluded = Vec::new();
        for (name, kind) in fields {
            let provider = match kind.as_str() {
                "FilePicker" => "filePicker",
                "WindowControls" => "windowControls",
                _ => continue,
            };
            let mut rewritten = String::new();
            for line in output.split_inclusive('\n') {
                let Some((prefix, declared)) = line.split_once(':') else {
                    rewritten.push_str(line);
                    continue;
                };
                if prefix.split_whitespace().last() == Some(name.as_str())
                    && declared.trim() == kind
                {
                    let _ = writeln!(
                        rewritten,
                        "{prefix}: {kind} = NativeCapabilities.{provider}()"
                    );
                    excluded.push(name.clone());
                } else {
                    rewritten.push_str(line);
                }
            }
            output = rewritten;
        }
        (output, excluded)
    }
}
